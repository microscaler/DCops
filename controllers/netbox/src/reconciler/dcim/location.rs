//! NetBoxLocation reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers;
use tracing::{info, error, debug, warn};
use crds::{NetBoxLocation, ResourceState};
use netbox_client::{NetBoxClientTrait, LocationId, SiteId, TenantId};

impl Reconciler {
    pub async fn reconcile_netbox_location(&self, location_crd: &NetBoxLocation) -> Result<(), ControllerError> {
        // Extract namespace and tenant reference
        let namespace = location_crd.metadata.namespace.as_deref().unwrap_or("default");
        let tenant_ref = &location_crd.spec.tenant;
        
        // SINGLE POINT: Get tenant-specific client
        let netbox_client = self.token_resolver
            .create_client_for_tenant(namespace, tenant_ref)
            .await?;
        
        let name = location_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxLocation missing name".to_string()))?;
        
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
                // Need to create location - resolve dependencies first
                // Resolve site ID (required)
                if location_crd.spec.site.kind != "NetBoxSite" {
                    return Err(ControllerError::InvalidConfig(
                        format!("Invalid kind '{}' for site reference in location {}, expected 'NetBoxSite'", location_crd.spec.site.kind, name)
                    ));
                }
                let site_id = match self.netbox_site_api.get(&location_crd.spec.site.name).await {
                    Ok(site_crd) => {
                        site_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                            .ok_or_else(|| ControllerError::InvalidConfig(
                                format!("Site '{}' has not been created in NetBox yet (no netbox_id in status)", location_crd.spec.site.name)
                            ))?
                    }
                    Err(_) => {
                        return Err(ControllerError::InvalidConfig(
                            format!("Site CRD '{}' not found for location {}", location_crd.spec.site.name, name)
                        ));
                    }
                };
                
                // Resolve parent location ID if parent reference provided
                let parent_id = if let Some(parent_ref) = &location_crd.spec.parent {
            if parent_ref.kind != "NetBoxLocation" {
                warn!("Invalid kind '{}' for parent location reference in location {}, expected 'NetBoxLocation'", parent_ref.kind, name);
                None
            } else {
                match self.netbox_location_api.get(&parent_ref.name).await {
                    Ok(parent_crd) => {
                        parent_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                    }
                    Err(_) => {
                        warn!("Parent location CRD '{}' not found for location {}", parent_ref.name, name);
                        None
                    }
                }
            }
                } else {
                    None
                };
                
                // Resolve tenant ID (required)
                if location_crd.spec.tenant.kind != "NetBoxTenant" {
                    return Err(ControllerError::InvalidConfig(
                        format!("Invalid kind '{}' for tenant reference in location {}, expected 'NetBoxTenant'", location_crd.spec.tenant.kind, name)
                    ));
                }
                let tenant_id = match self.netbox_tenant_api.get(&location_crd.spec.tenant.name).await {
                    Ok(tenant_crd) => {
                        tenant_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                            .ok_or_else(|| ControllerError::InvalidConfig(
                                format!("Tenant '{}' has not been created in NetBox yet (no netbox_id in status)", location_crd.spec.tenant.name)
                            ))?
                    }
                    Err(_) => {
                        return Err(ControllerError::InvalidConfig(
                            format!("Tenant CRD '{}' not found for location {}", location_crd.spec.tenant.name, name)
                        ));
                    }
                };
                
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
