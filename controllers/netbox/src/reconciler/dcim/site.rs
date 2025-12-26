//! NetBoxSite reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use kube::Api;
use tracing::{info, error, debug, warn};
use crds::{NetBoxSite, NetBoxSiteStatus, ResourceState};
use netbox_client;

impl Reconciler {
    fn site_needs_update(
        spec: &crds::NetBoxSiteSpec,
        existing: &netbox_client::Site,
        desired_tenant_id: Option<u64>,
        desired_region_id: Option<u64>,
        desired_site_group_id: Option<u64>,
        desired_status: &str,
    ) -> bool {
        // Compare name
        if spec.name != existing.name {
            debug!("Site name changed: '{}' -> '{}'", existing.name, spec.name);
            return true;
        }
        
        // Compare slug
        if let Some(slug) = &spec.slug {
            if slug != &existing.slug {
                debug!("Site slug changed: '{}' -> '{}'", existing.slug, slug);
                return true;
            }
        }
        
        // Compare description
        if spec.description.as_deref() != existing.description.as_deref() {
            debug!("Site description changed");
            return true;
        }
        
        // Compare physical_address
        if spec.physical_address.as_deref() != existing.physical_address.as_deref() {
            debug!("Site physical_address changed");
            return true;
        }
        
        // Compare shipping_address
        if spec.shipping_address.as_deref() != existing.shipping_address.as_deref() {
            debug!("Site shipping_address changed");
            return true;
        }
        
        // Compare latitude
        if spec.latitude != existing.latitude {
            debug!("Site latitude changed: {:?} -> {:?}", existing.latitude, spec.latitude);
            return true;
        }
        
        // Compare longitude
        if spec.longitude != existing.longitude {
            debug!("Site longitude changed: {:?} -> {:?}", existing.longitude, spec.longitude);
            return true;
        }
        
        // Compare tenant
        let existing_tenant_id = existing.tenant.as_ref().map(|t| t.id);
        if desired_tenant_id != existing_tenant_id {
            debug!("Site tenant changed: {:?} -> {:?}", existing_tenant_id, desired_tenant_id);
            return true;
        }
        
        // Compare region
        let existing_region_id = existing.region.as_ref().map(|r| r.id);
        if desired_region_id != existing_region_id {
            debug!("Site region changed: {:?} -> {:?}", existing_region_id, desired_region_id);
            return true;
        }
        
        // Compare site_group
        let existing_site_group_id = existing.site_group.as_ref().map(|sg| sg.id);
        if desired_site_group_id != existing_site_group_id {
            debug!("Site site_group changed: {:?} -> {:?}", existing_site_group_id, desired_site_group_id);
            return true;
        }
        
        // Compare status
        let existing_status = match existing.status {
            netbox_client::SiteStatus::Active => "active",
            netbox_client::SiteStatus::Planned => "planned",
            netbox_client::SiteStatus::Retired => "retired",
            netbox_client::SiteStatus::Staging => "staging",
        };
        if desired_status != existing_status {
            debug!("Site status changed: '{}' -> '{}'", existing_status, desired_status);
            return true;
        }
        
        // Compare facility
        if spec.facility.as_deref() != existing.facility.as_deref() {
            debug!("Site facility changed");
            return true;
        }
        
        // Compare time_zone
        if spec.time_zone.as_deref() != existing.time_zone.as_deref() {
            debug!("Site time_zone changed");
            return true;
        }
        
        // Compare comments
        if spec.comments.as_deref() != existing.comments.as_deref() {
            debug!("Site comments changed");
            return true;
        }
        
        false // No changes needed
    }

    // DCIM reconciler functions

    pub async fn reconcile_netbox_site(&self, site_crd: &NetBoxSite) -> Result<(), ControllerError> {
        // Helper function to update status with error
        async fn update_status_error(
            api: &Api<NetBoxSite>,
            name: &str,
            namespace: &str,
            error_msg: String,
            current_status: Option<&NetBoxSiteStatus>,
        ) {
            if let Some(status) = current_status {
                if status.state == ResourceState::Failed && status.error.as_ref() == Some(&error_msg) {
                    debug!("NetBoxSite {}/{} already has this error in status, skipping update", namespace, name);
                    return;
                }
            }
            
            // Update status with error (use lowercase state to match CRD validation schema)
            let status_patch = Reconciler::create_resource_status_patch(
                0, // No netbox_id on error
                String::new(), // No URL on error
                ResourceState::Failed,
                Some(error_msg.clone()),
            );
            let pp = kube::api::PatchParams::default();
            if let Err(e) = api.patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch)).await {
                error!("Failed to update NetBoxSite {}/{} error status: {}", namespace, name, e);
            } else {
                info!("Updated NetBoxSite {}/{} status with error", namespace, name);
            }
        }
        
        let name = site_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxSite missing name".to_string()))?;
        let namespace = site_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxSite {}/{}", namespace, name);
        
        // Resolve tenant ID if tenant reference provided (needed for diffing and creation)
        let tenant_id = if let Some(tenant_ref) = &site_crd.spec.tenant {
            if tenant_ref.kind != "NetBoxTenant" {
                warn!("Invalid kind '{}' for tenant reference in site {}, expected 'NetBoxTenant'", tenant_ref.kind, name);
                None
            } else {
                match self.netbox_tenant_api.get(&tenant_ref.name).await {
                    Ok(tenant_crd) => {
                        tenant_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                    }
                    Err(_) => None
                }
            }
        } else {
            None
        };
        
        // Resolve region ID if region reference provided
        let region_id = if let Some(region_ref) = &site_crd.spec.region {
            if region_ref.kind != "NetBoxRegion" {
                warn!("Invalid kind '{}' for region reference in site {}, expected 'NetBoxRegion'", region_ref.kind, name);
                None
            } else {
                match self.netbox_region_api.get(&region_ref.name).await {
                    Ok(region_crd) => {
                        region_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                    }
                    Err(_) => {
                        warn!("Region CRD '{}' not found for site {}, skipping region reference", region_ref.name, name);
                        None
                    }
                }
            }
        } else {
            None
        };
        
        // Resolve site group ID if site group reference provided
        let site_group_id = if let Some(site_group_ref) = &site_crd.spec.site_group {
            if site_group_ref.kind != "NetBoxSiteGroup" {
                warn!("Invalid kind '{}' for site group reference in site {}, expected 'NetBoxSiteGroup'", site_group_ref.kind, name);
                None
            } else {
                match self.netbox_site_group_api.get(&site_group_ref.name).await {
                    Ok(site_group_crd) => {
                        site_group_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                    }
                    Err(_) => {
                        warn!("SiteGroup CRD '{}' not found for site {}, skipping site group reference", site_group_ref.name, name);
                        None
                    }
                }
            }
        } else {
            None
        };
        
        // Convert status enum to string
        let status_str = match site_crd.spec.status {
            crds::SiteStatus::Active => "active",
            crds::SiteStatus::Planned => "planned",
            crds::SiteStatus::Retired => "retired",
            crds::SiteStatus::Staging => "staging",
        };
        
        // Check if already created - use helper for drift detection and diffing
        let netbox_site = if let Some(status) = &site_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Use helper function for drift detection, diffing, and updating
                    // Only pass tenant_id/region_id/site_group_id if they're different from existing
                    // This prevents NetBox validation errors when sending unchanged nested objects
                    match self.netbox_client.get_site(netbox_id).await {
                        Ok(existing_site) => {
                            // Check which nested fields actually changed (independent of other field changes)
                            // This is critical: NetBox 4.0 validates nested objects strictly, so we only
                            // include them in the update if they've actually changed
                            let existing_tenant_id = existing_site.tenant.as_ref().map(|t| t.id);
                            let existing_region_id = existing_site.region.as_ref().map(|r| r.id);
                            let existing_site_group_id = existing_site.site_group.as_ref().map(|sg| sg.id);
                            
                            // Only include tenant/region/site_group if they've changed
                            // If unchanged, we pass None to exclude them from the update body (PATCH semantics)
                            let update_tenant_id = if tenant_id != existing_tenant_id {
                                tenant_id // Only include if changed
                            } else {
                                None // Don't include if unchanged (avoids NetBox validation errors)
                            };
                            
                            let update_region_id = if region_id != existing_region_id {
                                region_id
                            } else {
                                None
                            };
                            
                            let update_site_group_id = if site_group_id != existing_site_group_id {
                                site_group_id
                            } else {
                                None
                            };
                            
                            // Check if any field (including nested) changed
                            if Self::site_needs_update(
                                &site_crd.spec,
                                &existing_site,
                                tenant_id,
                                region_id,
                                site_group_id,
                                &status_str,
                            ) {
                                // Update the site with only changed fields
                                match self.netbox_client.update_site(
                                    netbox_id,
                                    Some(&site_crd.spec.name),
                                    site_crd.spec.slug.as_deref(),
                                    Some(status_str),
                                    update_region_id, // Only include if changed
                                    update_site_group_id, // Only include if changed
                                    update_tenant_id, // Only include if changed
                                    site_crd.spec.facility.as_deref(),
                                    site_crd.spec.time_zone.as_deref(),
                                    site_crd.spec.description.as_deref(),
                                    site_crd.spec.comments.as_deref(),
                                ).await {
                                    Ok(updated_site) => {
                                        // Update successful
                                        Some(updated_site)
                                    }
                                    Err(e) => {
                                        error!("Failed to update NetBoxSite {}/{} in NetBox: {}", namespace, name, e);
                                        return Err(ControllerError::NetBox(e));
                                    }
                                }
                            } else {
                                // No changes needed
                                Some(existing_site)
                            }
                        }
                        Err(e) => {
                            // Error getting existing site - treat as drift
                            warn!("Failed to get existing site {} from NetBox: {}, treating as deleted", netbox_id, e);
                            // Clear status and recreate
                            let status_patch = Self::create_resource_status_patch(
                                0, // Clear netbox_id
                                String::new(), // Clear URL
                                ResourceState::Pending,
                                Some(format!("Resource was deleted in NetBox, will recreate: {}", e)),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(update_err) = self.netbox_site_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                .await
                            {
                                warn!("Failed to clear NetBoxSite status after drift detection: {}", update_err);
                            }
                            // Fall through to creation
                            None
                        }
                    }
                } else {
                    None // No netbox_id, need to create
                }
            } else {
                None // Not in Created state, need to create
            }
        } else {
            None // No status, need to create
        };
        
        // Handle existing site (from helper) or create new
        let netbox_site = match netbox_site {
            Some(site) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    site_crd.status.as_ref(),
                    site.id,
                    &site.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        site.id,
                        site.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_site_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxSite {}/{} status: NetBox ID {}", namespace, name, site.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxSite status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxSite {}/{} already has correct status (ID: {}), skipping update", namespace, name, site.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create site - try to find existing by name (idempotency fallback)
                let existing_site = match self.netbox_client.query_sites(
                    &[("name", &site_crd.spec.name)],
                    false,
                ).await {
                    Ok(sites) => sites.first().cloned(),
                    Err(_) => None
                };
                
                if let Some(existing) = existing_site {
                    info!("Site {} already exists in NetBox (ID: {})", site_crd.spec.name, existing.id);
                    existing
                } else {
                    // Create new site
                    match self.netbox_client.create_site(
                        &site_crd.spec.name,
                        site_crd.spec.slug.as_deref(),
                        status_str,
                        region_id,
                        site_group_id,
                        tenant_id,
                        site_crd.spec.facility.as_deref(),
                        site_crd.spec.time_zone.as_deref(),
                        site_crd.spec.description.as_deref(),
                        site_crd.spec.comments.as_deref(),
                    ).await {
                        Ok(created) => {
                            info!("Created site {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create site in NetBox: {}", e);
                            error!("{}", error_msg);
                            update_status_error(&self.netbox_site_api, name, namespace, error_msg.clone(), site_crd.status.as_ref()).await;
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_site.id,
            netbox_site.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_site_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxSite {}/{} status: NetBox ID {}", namespace, name, netbox_site.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxSite status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
