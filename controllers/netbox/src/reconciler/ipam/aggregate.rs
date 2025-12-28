//! NetBoxAggregate reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxAggregate, ResourceState};
use netbox_client::{NetBoxClientTrait, RirId};

impl Reconciler {
    pub async fn reconcile_netbox_aggregate(&self, aggregate_crd: &NetBoxAggregate) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::extract_name_and_namespace;
        let (name, namespace) = extract_name_and_namespace(aggregate_crd, "NetBoxAggregate")?;
        
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
                    return Err(ControllerError::InvalidConfig(error_msg));
                }
                Err(e) => {
                    let error_msg = format!("Failed to get RIR '{}' for aggregate {}: {}", rir_name, name, e);
                    warn!("{} — will not create until resolved", error_msg);
                    update_status_error(&*self.netbox_aggregate_api, name, namespace, error_msg.clone(), aggregate_crd.status.as_ref()).await;
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
        
        let netbox_aggregate = match drift_result {
            DriftCheckResult::UseExisting(aggregate) => Some(aggregate),
            DriftCheckResult::StatusCleared { message } => {
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
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    aggregate_crd.status.as_ref(),
                    aggregate.id,
                    &aggregate.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    use crate::reconcile_helpers::update_resource_status;
                    let status_patch = Self::create_resource_status_patch(
                        aggregate.id,
                        aggregate.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    update_resource_status(
                        &*self.netbox_aggregate_api,
                        name,
                        namespace,
                        &status_patch,
                        "NetBoxAggregate",
                        aggregate.id,
                    ).await?;
                    debug!("Updated NetBoxAggregate {}/{} status: NetBox ID {}", namespace, name, aggregate.id);
                    return Ok(());
                } else {
                    debug!("NetBoxAggregate {}/{} already has correct status (ID: {}), skipping update", namespace, name, aggregate.id);
                    return Ok(());
                }
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
                
                if let Some(existing) = existing_aggregate {
                    existing
                } else {
                    debug!("Attempting to create NetBoxAggregate {} in NetBox", aggregate_crd.spec.prefix);
                    match netbox_client.create_aggregate(
                        &aggregate_crd.spec.prefix,
                        rir_id.map(RirId),
                        aggregate_crd.spec.date_allocated.as_deref(),
                        aggregate_crd.spec.description.clone(),
                        aggregate_crd.spec.comments.clone(),
                    ).await {
                        Ok(created_aggregate) => {
                            debug!("Successfully created NetBoxAggregate {} with ID {}", aggregate_crd.spec.prefix, created_aggregate.id);
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
