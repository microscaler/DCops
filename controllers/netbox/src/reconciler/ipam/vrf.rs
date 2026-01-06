//! NetBoxVRF reconciler
//!
//! Handles reconciliation of NetBox VRF (Virtual Routing and Forwarding) resources.
//! VRFs represent independent routing tables, allowing for the isolation of network traffic
//! and the use of overlapping IP address spaces.

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers::{extract_name_and_namespace, validate_status_and_drift, DriftCheckResult, update_resource_status, resolve_optional_dependency_id, validate_reference_kind, is_conflict_error};
use crate::kube_api_trait::KubeApiTrait;
use tracing::{info, error, debug, warn};
use crds::{NetBoxVRF, ResourceState};
use netbox_client::{VrfId, RouteTargetId};

impl Reconciler {
    /// Resolve Route Target references to IDs
    async fn resolve_route_target_ids(
        api: &dyn KubeApiTrait<crds::NetBoxRouteTarget>,
        route_target_refs: &Option<Vec<crds::references::NetBoxResourceReference>>,
        name: &str,
    ) -> Option<Vec<RouteTargetId>> {
        if let Some(refs) = route_target_refs {
            let mut ids = Vec::new();
            for rt_ref in refs {
                if validate_reference_kind(rt_ref, "NetBoxRouteTarget", "route_target", name).is_ok() {
                    if let Some(rt_id) = resolve_optional_dependency_id(
                        api,
                        Some(rt_ref),
                        "NetBoxRouteTarget",
                        "route_target",
                        name,
                        |crd| crd.status.as_ref(),
                    ).await {
                        ids.push(RouteTargetId(rt_id));
                    }
                }
            }
            if ids.is_empty() {
                None
            } else {
                Some(ids)
            }
        } else {
            None
        }
    }

    /// Check if VRF needs updating by comparing spec with existing NetBox resource
    fn vrf_needs_update(
        spec: &crds::NetBoxVRFSpec,
        existing: &netbox_client::Vrf,
        desired_tenant_id: Option<u64>,
        desired_import_target_ids: &Option<Vec<u64>>,
        desired_export_target_ids: &Option<Vec<u64>>,
    ) -> bool {
        use crate::reconcile_helpers::{
            compare_string_field,
            compare_optional_string_field,
            compare_optional_dependency_id,
        };
        
        let existing_tenant_id = existing.tenant.as_ref().map(|t| t.id);
        
        // VRF model has description and comments as String (not Option<String>)
        let spec_description = spec.description.as_deref().unwrap_or("");
        let spec_comments = spec.comments.as_deref().unwrap_or("");
        
        // Compare route target vectors (need to sort for comparison)
        let mut existing_import_ids: Vec<u64> = existing.import_targets.iter().map(|rt| rt.id).collect();
        existing_import_ids.sort();
        let mut desired_import_ids = desired_import_target_ids.as_ref().cloned().unwrap_or_default();
        desired_import_ids.sort();
        
        let mut existing_export_ids: Vec<u64> = existing.export_targets.iter().map(|rt| rt.id).collect();
        existing_export_ids.sort();
        let mut desired_export_ids = desired_export_target_ids.as_ref().cloned().unwrap_or_default();
        desired_export_ids.sort();
        
        // Evaluate all comparisons to log all field differences (no short-circuit)
        let name_diff = compare_string_field(&spec.name, &existing.name);
        let rd_diff = compare_optional_string_field(&spec.rd, &existing.rd);
        let enforce_unique_diff = spec.enforce_unique != existing.enforce_unique;
        let tenant_diff = compare_optional_dependency_id(desired_tenant_id, existing_tenant_id);
        let description_diff = compare_string_field(spec_description, &existing.description);
        let comments_diff = compare_string_field(spec_comments, &existing.comments);
        let import_ids_diff = existing_import_ids != desired_import_ids;
        let export_ids_diff = existing_export_ids != desired_export_ids;
        // Tags are handled separately using tags_differ helper
        
        name_diff || rd_diff || enforce_unique_diff || tenant_diff || description_diff || comments_diff || import_ids_diff || export_ids_diff
    }

    pub async fn reconcile_netbox_vrf(&self, vrf_crd: &NetBoxVRF) -> Result<(), ControllerError> {
        let (name, namespace) = extract_name_and_namespace(vrf_crd, "NetBoxVRF")?;
        
        info!("Reconciling NetBoxVRF {}/{}", namespace, name);
        
        // Get tenant-specific client (if tenant is specified) or shared resource client
        let netbox_client = if let Some(tenant_ref) = &vrf_crd.spec.tenant {
            self.token_resolver
                .create_client_for_tenant(namespace, tenant_ref)
                .await?
        } else {
            self.token_resolver
                .create_client_for_shared_resource(namespace, "NetBoxVRF", name)
                .await
                .map_err(|e| ControllerError::TokenResolution(e))?
        };
        
        // Helper function to update status with error
        async fn update_status_error(
            api: &dyn KubeApiTrait<NetBoxVRF>,
            name: &str,
            namespace: &str,
            error_msg: String,
            current_status: Option<&crds::NetBoxVRFStatus>,
        ) {
            if let Some(status) = current_status {
                if status.state == ResourceState::Failed && status.error.as_ref() == Some(&error_msg) {
                    debug!("NetBoxVRF {}/{} already has this error in status, skipping update", namespace, name);
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
                error!("Failed to update NetBoxVRF {}/{} error status: {}", namespace, name, e);
            } else {
                info!("Updated NetBoxVRF {}/{} status with error", namespace, name);
            }
        }
        
        // Validate status and check for drift
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                vrf_crd.status.as_ref(),
                "NetBoxVRF",
                namespace,
                name,
                |netbox_id: u64| async move {
                    netbox_client_ref.get_vrf(VrfId(netbox_id)).await
                },
            ).await?
        };
        
        // Resolve Route Target IDs for import/export targets
        let import_target_ids = Self::resolve_route_target_ids(
            &*self.netbox_route_target_api,
            &vrf_crd.spec.import_targets,
            name,
        ).await;
        
        let export_target_ids = Self::resolve_route_target_ids(
            &*self.netbox_route_target_api,
            &vrf_crd.spec.export_targets,
            name,
        ).await;
        
        // Convert RouteTargetId to u64 for comparison (clone first to avoid move issues)
        let import_target_ids_clone = import_target_ids.clone();
        let export_target_ids_clone = export_target_ids.clone();
        let import_target_ids_u64: Option<Vec<u64>> = import_target_ids_clone.as_ref().map(|ids| ids.iter().map(|rt_id| rt_id.0).collect());
        let export_target_ids_u64: Option<Vec<u64>> = export_target_ids_clone.as_ref().map(|ids| ids.iter().map(|rt_id| rt_id.0).collect());
        
        let netbox_vrf = match drift_result {
            DriftCheckResult::UseExisting(existing_vrf) => {
                // Resource exists - check if it needs updating
                // Resolve dependencies for comparison
                let tenant_id: Option<u64> = if let Some(tenant_ref) = &vrf_crd.spec.tenant {
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
                    &vrf_crd.spec.tags,
                    namespace,
                    name,
                    None,
                    "NetBoxVRF",
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
                if Self::vrf_needs_update(
                    &vrf_crd.spec,
                    &existing_vrf,
                    tenant_id,
                    &import_target_ids_u64,
                    &export_target_ids_u64,
                ) {
                    // Update the VRF
                    debug!("Updating VRF {} with tenant_id: {:?}, import_targets: {:?}, export_targets: {:?}, tags: {:?}", 
                        existing_vrf.id, tenant_id, import_target_ids, export_target_ids, resolved_tags);
                    
                    match netbox_client.update_vrf(
                        VrfId(existing_vrf.id),
                        Some(&vrf_crd.spec.name),
                        vrf_crd.spec.rd.as_deref(),
                        Some(vrf_crd.spec.enforce_unique),
                        tenant_id.map(netbox_client::TenantId),
                        vrf_crd.spec.description.clone(),
                        vrf_crd.spec.comments.clone(),
                        import_target_ids.clone(),
                        export_target_ids.clone(),
                        resolved_tags,
                    ).await {
                        Ok(updated_vrf) => {
                            // Update successful
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::UPDATED,
                                &format!("Updated VRF {} in NetBox (ID: {})", updated_vrf.name, updated_vrf.id),
                                vrf_crd,
                            ).await;
                            Some(updated_vrf)
                        }
                        Err(e) => {
                            error!("Failed to update NetBoxVRF {}/{} in NetBox: {}", namespace, name, e);
                            use crate::events::reasons;
                            self.record_event_warning(
                                reasons::RECONCILIATION_FAILED,
                                &format!("Failed to update NetBoxVRF {}/{} in NetBox: {}", namespace, name, e),
                                vrf_crd,
                            ).await;
                            update_status_error(&*self.netbox_vrf_api, name, namespace, format!("{}", e), vrf_crd.status.as_ref()).await;
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                } else {
                    // No changes needed
                    Some(existing_vrf)
                }
            }
            DriftCheckResult::StatusCleared { message } => {
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxVRF {}/{} drift detected: {}", namespace, name, message),
                    vrf_crd,
                ).await;
                
                let status_patch = Self::create_resource_status_patch(
                    0,
                    String::new(),
                    ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_vrf_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxVRF status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        // If we have a NetBox VRF, update status and return
        if let Some(vrf) = netbox_vrf {
            let status_patch = Self::create_resource_status_patch(
                vrf.id,
                vrf.url.clone(),
                ResourceState::Created,
                None,
            );
            update_resource_status(
                &*self.netbox_vrf_api,
                name,
                namespace,
                &status_patch,
                "NetBoxVRF",
                vrf.id,
            ).await?;
            return Ok(());
        }
        
        // Need to create the VRF
        // Resolve dependencies
        let tenant_id: Option<netbox_client::TenantId> = if let Some(tenant_ref) = &vrf_crd.spec.tenant {
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
        
        // Check for existing VRF by name (idempotency)
        let existing_vrf_opt = match netbox_client.get_vrf_by_name(&vrf_crd.spec.name).await {
            Ok(Some(existing)) => {
                // VRF already exists - use it
                info!("VRF '{}' already exists in NetBox (ID: {}), using it", 
                    existing.name, existing.id);
                Some(existing)
            }
            Ok(None) => None,
            Err(_) => None, // If query fails, proceed with creation
        };
        
        // Resolve tags for creation
        let resolved_tags_json = self.resolve_tag_references(
            netbox_client.as_ref(),
            &vrf_crd.spec.tags,
            namespace,
            name,
            None,
            "NetBoxVRF",
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
        
        debug!("Creating VRF {} with tenant_id: {:?}, import_targets: {:?}, export_targets: {:?}, tags: {:?}", 
            vrf_crd.spec.name, tenant_id, import_target_ids, export_target_ids, resolved_tags);
        
        let netbox_vrf = if let Some(existing) = existing_vrf_opt {
            // VRF was found in pre-check
            existing
        } else {
            // Create the VRF
            match netbox_client.create_vrf(
                &vrf_crd.spec.name,
                vrf_crd.spec.rd.as_deref(),
                Some(vrf_crd.spec.enforce_unique),
                tenant_id,
                vrf_crd.spec.description.clone(),
                vrf_crd.spec.comments.clone(),
                import_target_ids.clone(),
                export_target_ids.clone(),
                resolved_tags,
            ).await {
                Ok(created_vrf) => {
                    // Creation successful
                    use crate::events::reasons;
                    self.record_event_normal(
                        reasons::CREATED,
                        &format!("Created VRF {} in NetBox (ID: {})", created_vrf.name, created_vrf.id),
                        vrf_crd,
                    ).await;
                    created_vrf
                }
                Err(e) => {
                    error!("Failed to create NetBoxVRF {}/{} in NetBox: {}", namespace, name, e);
                    use crate::events::reasons;
                    self.record_event_warning(
                        reasons::RECONCILIATION_FAILED,
                        &format!("Failed to create NetBoxVRF {}/{} in NetBox: {}", namespace, name, e),
                        vrf_crd,
                    ).await;
                    
                    if is_conflict_error(&e) {
                        // Conflict - resource may have been created concurrently
                        // Try to fetch it by name
                        match netbox_client.get_vrf_by_name(&vrf_crd.spec.name).await {
                            Ok(Some(existing)) => {
                                info!("VRF '{}' was created concurrently, using existing (ID: {})", 
                                    existing.name, existing.id);
                                existing
                            }
                            _ => {
                                update_status_error(&*self.netbox_vrf_api, name, namespace, format!("{}", e), vrf_crd.status.as_ref()).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    } else {
                        update_status_error(&*self.netbox_vrf_api, name, namespace, format!("{}", e), vrf_crd.status.as_ref()).await;
                        return Err(ControllerError::NetBox(e));
                    }
                }
            }
        };
        
        // Update status with NetBox VRF ID
        let status_patch = Self::create_resource_status_patch(
            netbox_vrf.id,
            netbox_vrf.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_vrf_api,
            name,
            namespace,
            &status_patch,
            "NetBoxVRF",
            netbox_vrf.id,
        ).await?;
        
        Ok(())
    }
}

