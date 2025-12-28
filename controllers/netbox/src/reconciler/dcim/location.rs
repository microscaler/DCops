//! NetBoxLocation reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxLocation, ResourceState};
use netbox_client::{NetBoxClientTrait, LocationId, SiteId, TenantId};

impl Reconciler {
    pub async fn reconcile_netbox_location(&self, location_crd: &NetBoxLocation) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::extract_name_and_namespace;
        let (name, namespace) = extract_name_and_namespace(location_crd, "NetBoxLocation")?;
        let tenant_ref = &location_crd.spec.tenant;
        
        // SINGLE POINT: Get tenant-specific client
        let netbox_client = self.token_resolver
            .create_client_for_tenant(namespace, tenant_ref)
            .await?;
        
        info!("Reconciling NetBoxLocation {}/{}", namespace, name);
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                location_crd.status.as_ref(),
                "NetBoxLocation",
                namespace,
                name,
                |netbox_id| async move {
                    netbox_client_ref.get_location(LocationId(netbox_id)).await
                },
            ).await?
        };
        
        let netbox_location = match drift_result {
            DriftCheckResult::UseExisting(location) => {
                // Resource exists and is up-to-date
                Some(location)
            }
            DriftCheckResult::StatusCleared { message } => {
                // Status was cleared - update it to Pending
                let status_patch = Self::create_resource_status_patch(
                    0, // Clear netbox_id
                    String::new(), // Clear URL
                    ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_location_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxLocation status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        // Handle existing location (from helper) or create new
        let netbox_location = match netbox_location {
            Some(location) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    location_crd.status.as_ref(),
                    location.id,
                    &location.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        location.id,
                        location.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_location_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxLocation {}/{} status: NetBox ID {}", namespace, name, location.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxLocation status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxLocation {}/{} already has correct status (ID: {}), skipping update", namespace, name, location.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create location - resolve dependencies first using helpers
                use crate::reconcile_helpers::{validate_reference_kind, resolve_required_dependency_id, resolve_optional_dependency_id};
                
                // Validate and resolve site ID (required)
                validate_reference_kind(&location_crd.spec.site, "NetBoxSite", "site", name)?;
                let site_id = resolve_required_dependency_id(
                    &*self.netbox_site_api,
                    &location_crd.spec.site.name,
                    "Site",
                    name,
                    |crd| crd.status.as_ref(),
                ).await?;
                
                // Resolve optional parent location ID
                let parent_id = resolve_optional_dependency_id(
                    &*self.netbox_location_api,
                    location_crd.spec.parent.as_ref(),
                    "NetBoxLocation",
                    "parent",
                    name,
                    |crd| crd.status.as_ref(),
                ).await;
                
                // Validate and resolve tenant ID (required)
                validate_reference_kind(&location_crd.spec.tenant, "NetBoxTenant", "tenant", name)?;
                let tenant_id = resolve_required_dependency_id(
                    &*self.netbox_tenant_api,
                    &location_crd.spec.tenant.name,
                    "Tenant",
                    name,
                    |crd| crd.status.as_ref(),
                ).await?;
                
                // Try to find existing location by name and site
                let existing_location = match netbox_client.query_locations(
                    &[("site_id", &site_id.to_string()), ("name", &location_crd.spec.name)],
                    false,
                ).await {
                    Ok(locations) => locations.first().cloned(),
                    Err(_) => None
                };
                
                let netbox_location = if let Some(existing) = existing_location {
                    info!("Location {} already exists in NetBox (ID: {})", location_crd.spec.name, existing.id);
                    existing
                } else {
                    match netbox_client.create_location(
                        SiteId(site_id),
                        &location_crd.spec.name,
                        location_crd.spec.slug.as_deref(),
                        parent_id.map(LocationId),
                        Some(TenantId(tenant_id)),
                        location_crd.spec.facility.as_deref(),
                        location_crd.spec.description.clone(),
                        None, // comments not in spec
                    ).await {
                        Ok(created) => {
                            info!("Created location {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create location in NetBox: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                };
                
                netbox_location
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_location.id,
            netbox_location.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_location_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxLocation {}/{} status: NetBox ID {}", namespace, name, netbox_location.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxLocation status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
