//! NetBoxRegion reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxRegion, ResourceState};
use netbox_client::{NetBoxClientTrait, RegionId};

impl Reconciler {
    pub async fn reconcile_netbox_region(&self, region_crd: &NetBoxRegion) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::{extract_name_and_namespace, resolve_optional_dependency_id};
        let (name, namespace) = extract_name_and_namespace(region_crd, "NetBoxRegion")?;
        
        info!("Reconciling NetBoxRegion {}/{}", namespace, name);
        
        // Get client for shared resource (finds tenant from referencing Sites)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxRegion", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Resolve optional parent region ID using helper
        let parent_id = resolve_optional_dependency_id(
            &*self.netbox_region_api,
            region_crd.spec.parent.as_ref(),
            "NetBoxRegion",
            "parent",
            name,
            |crd| crd.status.as_ref(),
        ).await;
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                region_crd.status.as_ref(),
                "NetBoxRegion",
                namespace,
                name,
                |netbox_id| async move {
                    netbox_client_ref.get_region(RegionId(netbox_id)).await
                },
            ).await?
        };
        
        let netbox_region = match drift_result {
            DriftCheckResult::UseExisting(region) => {
                // Resource exists and is up-to-date
                Some(region)
            }
            DriftCheckResult::StatusCleared { message } => {
                // Status was cleared - update it to Pending
                let status_patch = Self::create_typed_region_status_patch(
                    0, // Clear netbox_id
                    String::new(), // Clear URL
                    ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_region_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxRegion status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        // Handle existing region (from helper) or create new
        let netbox_region = match netbox_region {
            Some(region) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    region_crd.status.as_ref(),
                    region.id,
                    &region.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    use crate::reconcile_helpers::update_resource_status;
                    let status_patch = Self::create_typed_region_status_patch(
                        region.id,
                        region.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    update_resource_status(
                        &*self.netbox_region_api,
                        name,
                        namespace,
                        &status_patch,
                        "NetBoxRegion",
                        region.id,
                    ).await?;
                    debug!("Updated NetBoxRegion {}/{} status: NetBox ID {}", namespace, name, region.id);
                    return Ok(());
                } else {
                    debug!("NetBoxRegion {}/{} already has correct status (ID: {}), skipping update", namespace, name, region.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create region - try to find existing by name (idempotency fallback)
                let existing_region = match netbox_client.get_region_by_name(&region_crd.spec.name).await {
                    Ok(Some(region)) => {
                        info!("Region {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", region_crd.spec.name, region.id);
                        Some(region)
                    }
                    Ok(None) => None,
                    Err(e) => {
                        warn!("Failed to query region by name: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(existing) = existing_region {
                    existing
                } else {
                    // Create region
                    info!("Creating region {} in NetBox", region_crd.spec.name);
                    match netbox_client.create_region(
                        &region_crd.spec.name,
                        region_crd.spec.slug.as_deref(),
                        parent_id.map(RegionId),
                        region_crd.spec.description.clone(),
                        None, // comments - not in CRD spec yet
                    ).await {
                        Ok(created) => {
                            info!("Created region {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create region in NetBox: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        // Update status using helper
        use crate::reconcile_helpers::update_resource_status;
        let status_patch = Self::create_typed_region_status_patch(
            netbox_region.id,
            netbox_region.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_region_api,
            name,
            namespace,
            &status_patch,
            "NetBoxRegion",
            netbox_region.id,
        ).await?;
        info!("Updated NetBoxRegion {}/{} status: NetBox ID {}", namespace, name, netbox_region.id);
        Ok(())
    }
}
