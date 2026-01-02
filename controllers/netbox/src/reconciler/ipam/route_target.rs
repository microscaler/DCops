//! NetBoxRouteTarget reconciler
//!
//! Handles reconciliation of NetBox Route Target resources.
//! Route targets are extended BGP communities used to manage route redistribution
//! among VRF tables, particularly in L3VPN scenarios.

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers::{extract_name_and_namespace, validate_status_and_drift, DriftCheckResult, update_resource_status, resolve_optional_dependency_id, validate_reference_kind, is_conflict_error};
use crate::kube_api_trait::KubeApiTrait;
use tracing::{info, error, debug, warn};
use crds::{NetBoxRouteTarget, ResourceState};
use netbox_client::RouteTargetId;

impl Reconciler {
    /// Check if Route Target needs updating by comparing spec with existing NetBox resource
    fn route_target_needs_update(
        spec: &crds::NetBoxRouteTargetSpec,
        existing: &netbox_client::RouteTarget,
        desired_tenant_id: Option<u64>,
    ) -> bool {
        // Compare name
        if spec.name != existing.name {
            debug!("Route Target name changed: '{}' -> '{}'", existing.name, spec.name);
            return true;
        }
        
        // Compare tenant
        let existing_tenant_id = existing.tenant.as_ref().map(|t| t.id);
        if desired_tenant_id != existing_tenant_id {
            debug!("Route Target tenant changed: {:?} -> {:?}", existing_tenant_id, desired_tenant_id);
            return true;
        }
        
        // Compare description
        let spec_desc = spec.description.as_deref().unwrap_or("");
        if spec_desc != existing.description {
            debug!("Route Target description changed: '{}' -> '{}'", existing.description, spec_desc);
            return true;
        }
        
        // Compare comments
        let spec_comments = spec.comments.as_deref().unwrap_or("");
        if spec_comments != existing.comments {
            debug!("Route Target comments changed: '{}' -> '{}'", existing.comments, spec_comments);
            return true;
        }
        
        // Compare tags using helper function
        if crate::reconcile_helpers::tags_differ(&existing.tags, &spec.tags) {
            return true;
        }
        
        false // No changes needed
    }

    pub async fn reconcile_netbox_route_target(&self, route_target_crd: &NetBoxRouteTarget) -> Result<(), ControllerError> {
        let (name, namespace) = extract_name_and_namespace(route_target_crd, "NetBoxRouteTarget")?;
        
        info!("Reconciling NetBoxRouteTarget {}/{}", namespace, name);
        
        // Get tenant-specific client (if tenant is specified) or shared resource client
        let netbox_client = if let Some(tenant_ref) = &route_target_crd.spec.tenant {
            self.token_resolver
                .create_client_for_tenant(namespace, tenant_ref)
                .await?
        } else {
            self.token_resolver
                .create_client_for_shared_resource(namespace, "NetBoxRouteTarget", name)
                .await
                .map_err(|e| ControllerError::TokenResolution(e))?
        };
        
        // Helper function to update status with error
        async fn update_status_error(
            api: &dyn KubeApiTrait<NetBoxRouteTarget>,
            name: &str,
            namespace: &str,
            error_msg: String,
            current_status: Option<&crds::NetBoxRouteTargetStatus>,
        ) {
            if let Some(status) = current_status {
                if status.state == ResourceState::Failed && status.error.as_ref() == Some(&error_msg) {
                    debug!("NetBoxRouteTarget {}/{} already has this error in status, skipping update", namespace, name);
                    return;
                }
            }
            
            let status_patch = Reconciler::create_resource_status_patch(
                0,
                String::new(),
                ResourceState::Failed,
                Some(error_msg.clone()),
            );
            let pp = kube::api::PatchParams::default();
            if let Err(e) = api.patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone())).await {
                error!("Failed to update NetBoxRouteTarget {}/{} error status: {}", namespace, name, e);
            } else {
                info!("Updated NetBoxRouteTarget {}/{} status with error", namespace, name);
            }
        }
        
        // Validate status and check for drift
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                route_target_crd.status.as_ref(),
                "NetBoxRouteTarget",
                namespace,
                name,
                |netbox_id: u64| async move {
                    netbox_client_ref.get_route_target(RouteTargetId(netbox_id)).await
                },
            ).await?
        };
        
        let netbox_route_target = match drift_result {
            DriftCheckResult::UseExisting(existing_rt) => {
                // Resource exists - check if it needs updating
                // Resolve dependencies for comparison
                let tenant_id: Option<u64> = if let Some(tenant_ref) = &route_target_crd.spec.tenant {
                    if validate_reference_kind(tenant_ref, "NetBoxTenant", "tenant", name).is_ok() {
                        resolve_optional_dependency_id(
                            &*self.netbox_tenant_api,
                            Some(tenant_ref),
                            "NetBoxTenant",
                            "tenant",
                            name,
                            |crd| crd.status.as_ref(),
                        ).await
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                // Always resolve tags (even if nothing else changed, tags might need updating)
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &route_target_crd.spec.tags,
                    namespace,
                    name,
                ).await;
                
                // Convert resolved tags from Vec<serde_json::Value> to Vec<String>
                let resolved_tags: Option<Vec<String>> = resolved_tags_json.map(|tags| {
                    tags.into_iter()
                        .filter_map(|tag_value| {
                            if let Some(id) = tag_value.as_u64() {
                                Some(id.to_string())
                            } else {
                                warn!("Tag resolved to non-numeric format, skipping");
                                None
                            }
                        })
                        .collect()
                });
                
                // Check if any field changed (including tags)
                if Self::route_target_needs_update(
                    &route_target_crd.spec,
                    &existing_rt,
                    tenant_id,
                ) {
                    // Update the Route Target
                    debug!("Updating Route Target {} with tenant_id: {:?}, tags: {:?}", existing_rt.id, tenant_id, resolved_tags);
                    
                    match netbox_client.update_route_target(
                        RouteTargetId(existing_rt.id),
                        Some(&route_target_crd.spec.name),
                        tenant_id.map(netbox_client::TenantId),
                        route_target_crd.spec.description.clone(),
                        route_target_crd.spec.comments.clone(),
                        resolved_tags,
                    ).await {
                        Ok(updated_rt) => {
                            // Update successful
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::UPDATED,
                                &format!("Updated Route Target {} in NetBox (ID: {})", updated_rt.name, updated_rt.id),
                                route_target_crd,
                            ).await;
                            Some(updated_rt)
                        }
                        Err(e) => {
                            error!("Failed to update NetBoxRouteTarget {}/{} in NetBox: {}", namespace, name, e);
                            use crate::events::reasons;
                            self.record_event_warning(
                                reasons::RECONCILIATION_FAILED,
                                &format!("Failed to update NetBoxRouteTarget {}/{} in NetBox: {}", namespace, name, e),
                                route_target_crd,
                            ).await;
                            update_status_error(&*self.netbox_route_target_api, name, namespace, format!("{}", e), route_target_crd.status.as_ref()).await;
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                } else {
                    // No changes needed
                    Some(existing_rt)
                }
            }
            DriftCheckResult::StatusCleared { message } => {
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxRouteTarget {}/{} drift detected: {}", namespace, name, message),
                    route_target_crd,
                ).await;
                
                let status_patch = Self::create_resource_status_patch(
                    0,
                    String::new(),
                    ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_route_target_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxRouteTarget status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        // If we have a NetBox Route Target, update status and return
        if let Some(rt) = netbox_route_target {
            let status_patch = Self::create_resource_status_patch(
                rt.id,
                rt.url.clone(),
                ResourceState::Created,
                None,
            );
            update_resource_status(
                &*self.netbox_route_target_api,
                name,
                namespace,
                &status_patch,
                "NetBoxRouteTarget",
                rt.id,
            ).await?;
            return Ok(());
        }
        
        // Need to create the Route Target
        // Resolve dependencies
        let tenant_id: Option<netbox_client::TenantId> = if let Some(tenant_ref) = &route_target_crd.spec.tenant {
            if validate_reference_kind(tenant_ref, "NetBoxTenant", "tenant", name).is_ok() {
                resolve_optional_dependency_id(
                    &*self.netbox_tenant_api,
                    Some(tenant_ref),
                    "NetBoxTenant",
                    "tenant",
                    name,
                    |crd| crd.status.as_ref(),
                ).await.map(|id| netbox_client::TenantId(id))
            } else {
                None
            }
        } else {
            None
        };
        
        // Check for existing Route Target by name (idempotency)
        let existing_rt_opt = match netbox_client.get_route_target_by_name(&route_target_crd.spec.name).await {
            Ok(Some(existing)) => {
                // Route Target already exists - use it
                info!("Route Target '{}' already exists in NetBox (ID: {}), using it", 
                    existing.name, existing.id);
                Some(existing)
            }
            Ok(None) => None,
            Err(_) => None, // If query fails, proceed with creation
        };
        
        // Resolve tags for creation
        let resolved_tags_json = self.resolve_tag_references(
            netbox_client.as_ref(),
            &route_target_crd.spec.tags,
            namespace,
            name,
        ).await;
        
        // Convert resolved tags from Vec<serde_json::Value> to Vec<String>
        let resolved_tags: Option<Vec<String>> = resolved_tags_json.map(|tags| {
            tags.into_iter()
                .filter_map(|tag_value| {
                    if let Some(id) = tag_value.as_u64() {
                        Some(id.to_string())
                    } else {
                        warn!("Tag resolved to non-numeric format, skipping");
                        None
                    }
                })
                .collect()
        });
        
        debug!("Creating Route Target {} with tenant_id: {:?}, tags: {:?}", route_target_crd.spec.name, tenant_id, resolved_tags);
        
        let netbox_route_target = if let Some(existing) = existing_rt_opt {
            // Route Target was found in pre-check
            existing
        } else {
            // Create the Route Target
            match netbox_client.create_route_target(
                &route_target_crd.spec.name,
                tenant_id,
                route_target_crd.spec.description.clone(),
                route_target_crd.spec.comments.clone(),
                resolved_tags,
            ).await {
                Ok(created_rt) => {
                    // Creation successful
                    use crate::events::reasons;
                    self.record_event_normal(
                        reasons::CREATED,
                        &format!("Created Route Target {} in NetBox (ID: {})", created_rt.name, created_rt.id),
                        route_target_crd,
                    ).await;
                    created_rt
                }
                Err(e) => {
                    error!("Failed to create NetBoxRouteTarget {}/{} in NetBox: {}", namespace, name, e);
                    use crate::events::reasons;
                    self.record_event_warning(
                        reasons::RECONCILIATION_FAILED,
                        &format!("Failed to create NetBoxRouteTarget {}/{} in NetBox: {}", namespace, name, e),
                        route_target_crd,
                    ).await;
                    
                    if is_conflict_error(&e) {
                        // Conflict - resource may have been created concurrently
                        // Try to fetch it by name
                        match netbox_client.get_route_target_by_name(&route_target_crd.spec.name).await {
                            Ok(Some(existing)) => {
                                info!("Route Target '{}' was created concurrently, using existing (ID: {})", 
                                    existing.name, existing.id);
                                existing
                            }
                            _ => {
                                update_status_error(&*self.netbox_route_target_api, name, namespace, format!("{}", e), route_target_crd.status.as_ref()).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    } else {
                        update_status_error(&*self.netbox_route_target_api, name, namespace, format!("{}", e), route_target_crd.status.as_ref()).await;
                        return Err(ControllerError::NetBox(e));
                    }
                }
            }
        };
        
        // Update status with NetBox Route Target ID
        let status_patch = Self::create_resource_status_patch(
            netbox_route_target.id,
            netbox_route_target.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_route_target_api,
            name,
            namespace,
            &status_patch,
            "NetBoxRouteTarget",
            netbox_route_target.id,
        ).await?;
        
        Ok(())
    }
}

