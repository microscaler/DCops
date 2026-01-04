//! NetBoxAggregate reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxAggregate, ResourceState};
use netbox_client::{NetBoxClientTrait, RirId};

impl Reconciler {
    fn aggregate_needs_update(
        spec: &crds::NetBoxAggregateSpec,
        existing: &netbox_client::Aggregate,
        desired_rir_id: Option<u64>,
    ) -> bool {
        use crate::reconcile_helpers::{
            compare_string_field,
            compare_optional_string_field,
            compare_optional_dependency_id,
        };
        
        // Compare prefix - convert IpNet to String for comparison
        let existing_prefix_str = existing.prefix.to_string();
        let existing_rir_id = existing.rir.as_ref().map(|r| r.id);
        
        compare_string_field(&spec.prefix, &existing_prefix_str)
            || compare_optional_dependency_id(desired_rir_id, existing_rir_id)
            || compare_optional_string_field(&spec.date_allocated, &existing.date_allocated)
            || compare_optional_string_field(&spec.description, &existing.description)
            || compare_optional_string_field(&spec.comments, &existing.comments)
        // Tags are handled separately
    }

    pub async fn reconcile_netbox_aggregate(&self, aggregate_crd: &NetBoxAggregate) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::extract_name_and_namespace;
        let (name, namespace) = extract_name_and_namespace(aggregate_crd, "NetBoxAggregate")?;
        
        // Validate prefix format early with clear error message
        use std::str::FromStr;
        use ipnet::IpNet;
        let _prefix_net = IpNet::from_str(&aggregate_crd.spec.prefix)
            .map_err(|e| ControllerError::InvalidIPFormat(format!(
                "Invalid prefix format '{}' in NetBoxAggregate {}/{}: {}. Expected CIDR notation (e.g., '192.168.0.0/16' or '2001:db8::/32')",
                aggregate_crd.spec.prefix, namespace, name, e
            )))?;
        
        info!("Reconciling NetBoxAggregate {}/{}", namespace, name);
        
        // Local helper to patch status with an error message.
        async fn update_status_error(
            api: &(dyn crate::kube_api_trait::KubeApiTrait<NetBoxAggregate> + Send + Sync),
            name: &str,
            namespace: &str,
            error_msg: String,
            current_status: Option<&crds::NetBoxAggregateStatus>,
        ) {
            if let Some(status) = current_status {
                if status.state == ResourceState::Failed && status.error.as_ref() == Some(&error_msg) {
                    debug!("NetBoxAggregate {}/{} already has this error in status, skipping update", namespace, name);
                    return;
                }
            }
            
            let status_patch = Reconciler::create_resource_status_patch(
                0, // No netbox_id on error
                String::new(), // No URL on error
                ResourceState::Failed,
                Some(error_msg.clone()),
            );
            let pp = kube::api::PatchParams::default();
            if let Err(e) = api.patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone())).await {
                error!("Failed to update NetBoxAggregate {}/{} error status: {}", namespace, name, e);
            } else {
                info!("Updated NetBoxAggregate {}/{} status with error", namespace, name);
            }
        }
        
        // Get client for shared resource (falls back to main tenant)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxAggregate", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Resolve RIR ID (optional - if RIR is specified but doesn't exist, skip it)
        let rir_id = if let Some(rir_name) = &aggregate_crd.spec.rir {
            match netbox_client.get_rir_by_name(rir_name).await {
                Ok(Some(rir)) => {
                    debug!("Found RIR '{}' with ID {} for aggregate {}", rir_name, rir.id, name);
                    Some(rir.id)
                }
                Ok(None) => {
                    let error_msg = format!("RIR '{}' not found in NetBox for aggregate {}", rir_name, name);
                    warn!("{} — will not create until RIR exists or spec.rir is removed", error_msg);
                    update_status_error(&*self.netbox_aggregate_api, name, namespace, error_msg.clone(), aggregate_crd.status.as_ref()).await;
                    // Emit event for dependency not found
                    use crate::events::reasons;
                    self.record_event_warning(
                        reasons::DEPENDENCY_NOT_FOUND,
                        &error_msg,
                        aggregate_crd,
                    ).await;
                    return Err(ControllerError::InvalidConfig(error_msg));
                }
                Err(e) => {
                    let error_msg = format!("Failed to get RIR '{}' for aggregate {}: {}", rir_name, name, e);
                    warn!("{} — will not create until resolved", error_msg);
                    update_status_error(&*self.netbox_aggregate_api, name, namespace, error_msg.clone(), aggregate_crd.status.as_ref()).await;
                    // Emit event for reconciliation failure
                    use crate::events::reasons;
                    self.record_event_warning(
                        reasons::RECONCILIATION_FAILED,
                        &error_msg,
                        aggregate_crd,
                    ).await;
                    return Err(ControllerError::NetBox(e));
                }
            }
        } else {
            None // RIR is optional
        };
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                aggregate_crd.status.as_ref(),
                "NetBoxAggregate",
                namespace,
                name,
                |netbox_id: u64| async move {
                    netbox_client_ref.query_aggregates(&[("id", &netbox_id.to_string())], false)
                        .await
                        .map(|mut aggregates| aggregates.pop().ok_or_else(|| netbox_client::NetBoxError::NotFound(format!("Aggregate {} not found", netbox_id))))
                        .and_then(|res| res)
                },
            ).await?
        };
        
        // Check if drift detection is enabled (defaults to true)
        let drift_detection_enabled = aggregate_crd.spec.drift_detection.unwrap_or(true);
        
        let netbox_aggregate = match drift_result {
            DriftCheckResult::UseExisting(aggregate) => {
                // Check for field drift if enabled
                if drift_detection_enabled {
                    if Self::aggregate_needs_update(&aggregate_crd.spec, &aggregate, rir_id) {
                        // Field drift detected - update NetBox to match CRD (Git is source of truth)
                        warn!("NetBoxAggregate {}/{} fields differ from CRD spec, updating to match Git", namespace, name);
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DRIFT_DETECTED,
                            &format!("NetBoxAggregate {}/{} fields differ from CRD, updating to match Git", namespace, name),
                            aggregate_crd,
                        ).await;
                        
                        // Resolve tags for update
                        let resolved_tags_json = self.resolve_tag_references(
                            netbox_client.as_ref(),
                            &aggregate_crd.spec.tags,
                            namespace,
                            name,
                            Some(aggregate.id),
                        ).await;
                        let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                        
                        match netbox_client.update_aggregate(
                            netbox_client::AggregateId(aggregate.id),
                            rir_id.map(netbox_client::RirId),
                            aggregate_crd.spec.date_allocated.as_deref(),
                            aggregate_crd.spec.description.clone(),
                            aggregate_crd.spec.comments.clone(),
                            resolved_tags,
                        ).await {
                            Ok(updated) => {
                                use crate::events::reasons;
                                self.record_event_normal(
                                    reasons::UPDATED,
                                    &format!("Updated NetBoxAggregate {}/{} in NetBox to match CRD (ID: {})", namespace, name, updated.id),
                                    aggregate_crd,
                                ).await;
                                Some(updated)
                            }
                            Err(e) => {
                                error!("Failed to update NetBoxAggregate {}/{} in NetBox: {}", namespace, name, e);
                                Some(aggregate) // Use existing if update fails
                            }
                        }
                    } else {
                        // No drift - use existing
                        Some(aggregate)
                    }
                } else {
                    // Drift detection disabled - use existing
                    Some(aggregate)
                }
            }
            DriftCheckResult::StatusCleared { message } => {
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxAggregate {}/{} drift detected: {}", namespace, name, message),
                    aggregate_crd,
                ).await;
                
                let status_patch = Self::create_resource_status_patch(
                    0, String::new(), ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_aggregate_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxAggregate status: {}", update_err);
                }
                None
            }
            DriftCheckResult::Recreate => None,
        };
        
        let netbox_aggregate = match netbox_aggregate {
            Some(aggregate) => {
                // Update tags if they differ (tags are handled separately from field drift)
                let aggregate_id = aggregate.id;
                let aggregate_clone = aggregate.clone();
                let rir_id_clone = rir_id;
                let date_allocated = aggregate_crd.spec.date_allocated.clone();
                let description = aggregate_crd.spec.description.clone();
                let comments = aggregate_crd.spec.comments.clone();
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &aggregate_crd.spec.tags,
                    namespace,
                    name,
                    Some(aggregate_id),
                ).await;
                let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                
                let aggregate = match crate::reconcile_helpers::update_tags_if_differ(
                    aggregate,
                    &aggregate_crd.spec.tags,
                    resolved_tags.clone(),
                    |tags| {
                        let aggregate_id_clone = aggregate_id;
                        let rir_id_clone2 = rir_id_clone;
                        let date_allocated_clone = date_allocated.clone();
                        let description_clone = description.clone();
                        let comments_clone = comments.clone();
                        async move {
                            netbox_client.update_aggregate(
                                netbox_client::AggregateId(aggregate_id_clone),
                                rir_id_clone2.map(netbox_client::RirId),
                                date_allocated_clone.as_deref(),
                                description_clone,
                                comments_clone,
                                tags,
                            ).await
                        }
                    },
                    &format!("NetBoxAggregate {}/{}", namespace, name),
                ).await {
                    Ok(Some(updated)) => {
                        use crate::events::reasons;
                        self.record_event_normal(
                            reasons::UPDATED,
                            &format!("Updated NetBoxAggregate {}/{} tags in NetBox", namespace, name),
                            aggregate_crd,
                        ).await;
                        updated
                    }
                    Ok(None) => aggregate_clone, // Tags are up-to-date
                    Err(e) => {
                        warn!("Failed to update NetBoxAggregate {}/{} tags: {}", namespace, name, e);
                        aggregate_clone // Use existing if update fails
                    }
                };
                
                aggregate // Return existing Aggregate (status update happens at end)
            }
            None => {
                // Try to find existing aggregate by prefix
                let existing_aggregate = match netbox_client.query_aggregates(&[("prefix", &aggregate_crd.spec.prefix)], false).await {
                    Ok(mut aggregates) => {
                        aggregates.pop().map(|a| {
                            info!("Aggregate {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", a.prefix, a.id);
                            a
                        })
                    }
                    Err(e) => {
                        warn!("Failed to query aggregate: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(mut existing) = existing_aggregate {
                    // Check if tags need updating in idempotency path
                    let tags_need_update = crate::reconcile_helpers::tags_differ(&existing.tags, &aggregate_crd.spec.tags);
                    
                    if tags_need_update {
                        info!("Aggregate {}/{} tags differ in idempotency path, updating in NetBox", namespace, name);
                        // Resolve tags
                        let resolved_tags_json = self.resolve_tag_references(
                            netbox_client.as_ref(),
                            &aggregate_crd.spec.tags,
                            namespace,
                            name,
                        None,
                ).await;
                        let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                        
                        match netbox_client.as_ref().update_aggregate(
                            netbox_client::AggregateId(existing.id),
                            rir_id.map(netbox_client::RirId),
                            aggregate_crd.spec.date_allocated.as_deref(),
                            aggregate_crd.spec.description.clone(),
                            aggregate_crd.spec.comments.clone(),
                            resolved_tags,
                        ).await {
                            Ok(updated) => {
                                info!("Updated aggregate {} tags in NetBox (ID: {}) via idempotency path", updated.prefix, updated.id);
                                updated
                            }
                            Err(e) => {
                                warn!("Failed to update aggregate tags in idempotency path: {}", e);
                                // Continue with existing aggregate - tag update failure is non-fatal
                                existing
                            }
                        }
                    } else {
                        existing
                    }
                } else {
                    // Convert CRD string to IpNet
                    use std::str::FromStr;
                    use ipnet::IpNet;
                    let prefix_net = IpNet::from_str(&aggregate_crd.spec.prefix)
                        .map_err(|e| ControllerError::InvalidIPFormat(format!("Invalid prefix format in CRD: {} - {}", aggregate_crd.spec.prefix, e)))?;
                    
                    debug!("Attempting to create NetBoxAggregate {} in NetBox", aggregate_crd.spec.prefix);
                    
                    // Resolve tags before creation
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &aggregate_crd.spec.tags,
                        namespace,
                        name,
                    None,
                ).await;
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    match netbox_client.create_aggregate(
                        &prefix_net,
                        rir_id.map(RirId),
                        aggregate_crd.spec.date_allocated.as_deref(),
                        aggregate_crd.spec.description.clone(),
                        aggregate_crd.spec.comments.clone(),
                        resolved_tags,
                    ).await {
                        Ok(created_aggregate) => {
                            debug!("Successfully created NetBoxAggregate {} with ID {}", aggregate_crd.spec.prefix, created_aggregate.id);
                            // Emit event for successful creation
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::CREATED,
                                &format!("Created aggregate {} in NetBox (ID: {})", aggregate_crd.spec.prefix, created_aggregate.id),
                                aggregate_crd,
                            ).await;
                            created_aggregate
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create NetBoxAggregate in NetBox: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        // Update CRD status
        let patch = Self::create_resource_status_patch(
            netbox_aggregate.id,
            netbox_aggregate.url.clone(),
            ResourceState::Created,
            None,
        );
        self.netbox_aggregate_api
            .patch_status(name, &kube::api::PatchParams::default(), &kube::api::Patch::Merge(patch))
            .await
            .map_err(|e| ControllerError::Kube(e.into()))?;
        
        info!("Successfully reconciled NetBoxAggregate {}/{}", namespace, name);
        Ok(())
    }
}
