//! NetBoxSite reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::kube_api_trait::KubeApiTrait;
use tracing::{info, error, debug, warn};
use crds::{NetBoxSite, NetBoxSiteStatus, ResourceState};
use netbox_client::{NetBoxClientTrait, SiteId, TenantId, RegionId, SiteGroupId};

impl Reconciler {
    fn site_needs_update(
        spec: &crds::NetBoxSiteSpec,
        existing: &netbox_client::Site,
        desired_tenant_id: u64, // tenant is now required
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
        if Some(desired_tenant_id) != existing_tenant_id {
            debug!("Site tenant changed: {:?} -> {}", existing_tenant_id, desired_tenant_id);
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
            api: &dyn KubeApiTrait<NetBoxSite>,
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
            if let Err(e) = api.patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone())).await {
                error!("Failed to update NetBoxSite {}/{} error status: {}", namespace, name, e);
            } else {
                info!("Updated NetBoxSite {}/{} status with error", namespace, name);
            }
        }
        
        // Extract name and namespace using helper
        use crate::reconcile_helpers::{extract_name_and_namespace, validate_reference_kind, resolve_required_dependency_id};
        let (name, namespace) = extract_name_and_namespace(site_crd, "NetBoxSite")?;
        
        info!("Reconciling NetBoxSite {}/{}", namespace, name);
        
        // Validate tenant reference using helper
        validate_reference_kind(&site_crd.spec.tenant, "NetBoxTenant", "tenant", name)?;
        
        // SINGLE POINT OF DEPENDENCY INJECTION: Get tenant-specific NetBoxClient
        let netbox_client = self.token_resolver
            .create_client_for_tenant(namespace, &site_crd.spec.tenant)
            .await?;
        
        // Resolve tenant ID (required) using helper
        let tenant_id = resolve_required_dependency_id(
            &*self.netbox_tenant_api,
            &site_crd.spec.tenant.name,
            "Tenant",
            name,
            |crd| crd.status.as_ref(),
        ).await?;
        
        // Resolve region ID if region reference provided (optional) using helper
        use crate::reconcile_helpers::resolve_optional_dependency_id;
        let region_id = resolve_optional_dependency_id(
            &*self.netbox_region_api,
            site_crd.spec.region.as_ref(),
            "NetBoxRegion",
            "region",
            name,
            |crd| crd.status.as_ref(),
        ).await;
        
        // Resolve site group ID if site group reference provided (optional) using helper
        let site_group_id = resolve_optional_dependency_id(
            &*self.netbox_site_group_api,
            site_crd.spec.site_group.as_ref(),
            "NetBoxSiteGroup",
            "site_group",
            name,
            |crd| crd.status.as_ref(),
        ).await;
        
        // Convert status enum to string
        let status_str = match site_crd.spec.status {
            crds::SiteStatus::Active => "active",
            crds::SiteStatus::Planned => "planned",
            crds::SiteStatus::Retired => "retired",
            crds::SiteStatus::Staging => "staging",
        };
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                site_crd.status.as_ref(),
                "NetBoxSite",
                namespace,
                name,
                |netbox_id| async move {
                    netbox_client_ref.get_site(SiteId(netbox_id)).await
                },
            ).await?
        };
        
        let netbox_site = match drift_result {
            DriftCheckResult::UseExisting(existing_site) => {
                // Resource exists - check if it needs updating
                let existing_tenant_id = existing_site.tenant.as_ref().map(|t| t.id);
                let _existing_region_id = existing_site.region.as_ref().map(|r| r.id);
                let _existing_site_group_id = existing_site.site_group.as_ref().map(|sg| sg.id);
                
                // ALWAYS include tenant/region/site_group in PATCH requests
                // NetBox 4.0 seems to require these fields to be present even if unchanged
                let update_tenant_id = tenant_id; // Always include if we have a tenant_id
                let update_region_id = region_id; // Always include if we have a region_id
                let update_site_group_id = site_group_id; // Always include if we have a site_group_id
                
                if Some(tenant_id) != existing_tenant_id {
                    warn!("Site tenant changed: existing={:?}, desired={}", existing_tenant_id, tenant_id);
                } else {
                    debug!("Site tenant unchanged: {}, but will include in update", tenant_id);
                }
                
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
                    match netbox_client.update_site(
                        SiteId(existing_site.id),
                        Some(&site_crd.spec.name),
                        site_crd.spec.slug.as_deref(),
                        site_crd.spec.description.clone(),
                        site_crd.spec.physical_address.clone(),
                        site_crd.spec.shipping_address.clone(),
                        site_crd.spec.latitude,
                        site_crd.spec.longitude,
                        Some(TenantId(update_tenant_id)), // tenant is now required
                        update_region_id.map(RegionId),
                        update_site_group_id.map(SiteGroupId),
                        Some(status_str),
                        site_crd.spec.facility.clone(),
                        site_crd.spec.time_zone.clone(),
                        site_crd.spec.comments.clone(),
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
            DriftCheckResult::StatusCleared { message } => {
                // Status was cleared - update it to Pending
                let status_patch = Self::create_resource_status_patch(
                    0, // Clear netbox_id
                    String::new(), // Clear URL
                    ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_site_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxSite status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
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
                        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
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
                let existing_site = match netbox_client.query_sites(
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
                    // NOTE: Parameter order matches trait signature:
                    // name, slug, status, region_id, site_group_id, tenant_id, facility, time_zone, description, comments
                    match netbox_client.create_site(
                        &site_crd.spec.name,
                        site_crd.spec.slug.as_deref(),
                        site_crd.spec.description.clone(),
                        site_crd.spec.physical_address.clone(),
                        site_crd.spec.shipping_address.clone(),
                        site_crd.spec.latitude,
                        site_crd.spec.longitude,
                        Some(TenantId(tenant_id)), // tenant is now required
                        region_id.map(RegionId),
                        site_group_id.map(SiteGroupId),
                        Some(status_str),
                        site_crd.spec.facility.clone(),
                        site_crd.spec.time_zone.clone(),
                        site_crd.spec.comments.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created site {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            // Handle CREATE conflicts using shared helper (GitOps idempotency)
                            use crate::reconcile_helpers::is_conflict_error;
                            
                            if is_conflict_error(&e) {
                                warn!("Site {} creation failed with conflict, attempting to retrieve existing site (idempotency)", site_crd.spec.name);
                                
                                // Try multiple query strategies
                                let mut found_site = None;
                                
                                // Strategy 1: Query by name
                                match netbox_client.query_sites(&[("name", &site_crd.spec.name)], false).await {
                                    Ok(sites) => {
                                        if let Some(site) = sites.first() {
                                            info!("Found existing site by name '{}' in NetBox (ID: {}) after conflict", site_crd.spec.name, site.id);
                                            found_site = Some(site.clone());
                                        }
                                    }
                                    Err(_) => {}
                                }
                                
                                // Strategy 2: Query by slug if not found
                                if found_site.is_none() {
                                    if let Some(slug) = &site_crd.spec.slug {
                                        match netbox_client.query_sites(&[("slug", slug)], false).await {
                                            Ok(sites) => {
                                                if let Some(site) = sites.first() {
                                                    info!("Found existing site by slug '{}' in NetBox (ID: {}) after conflict", slug, site.id);
                                                    found_site = Some(site.clone());
                                                }
                                            }
                                            Err(_) => {}
                                        }
                                    }
                                }
                                
                                // Strategy 3: Fallback - query all and filter
                                if found_site.is_none() {
                                    match netbox_client.query_sites(&[], true).await {
                                        Ok(all_sites) => {
                                            if let Some(site) = all_sites.iter().find(|s| {
                                                s.name == site_crd.spec.name ||
                                                site_crd.spec.slug.as_ref().map(|slug| s.slug == *slug).unwrap_or(false)
                                            }) {
                                                info!("Found existing site in NetBox (ID: {}) via fallback query", site.id);
                                                found_site = Some(site.clone());
                                            }
                                        }
                                        Err(_) => {}
                                    }
                                }
                                
                                if let Some(found) = found_site {
                                    info!("Found existing site {} in NetBox (ID: {}) via conflict resolution (idempotency)", found.name, found.id);
                                    found
                                } else {
                                    let error_msg = format!("Site {} already exists in NetBox but could not retrieve it: {}", site_crd.spec.name, e);
                                    error!("{}", error_msg);
                                    update_status_error(&*self.netbox_site_api, name, namespace, error_msg.clone(), site_crd.status.as_ref()).await;
                                    return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                                }
                            } else {
                                // Not a conflict, return original error
                                let error_msg = format!("Failed to create site in NetBox: {}", e);
                                error!("{}", error_msg);
                                update_status_error(&*self.netbox_site_api, name, namespace, error_msg.clone(), site_crd.status.as_ref()).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                }
            }
        };
        
        // Update status using helper
        use crate::reconcile_helpers::update_resource_status;
        let status_patch = Self::create_resource_status_patch(
            netbox_site.id,
            netbox_site.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_site_api,
            name,
            namespace,
            &status_patch,
            "NetBoxSite",
            netbox_site.id,
        ).await?;
        info!("Updated NetBoxSite {}/{} status: NetBox ID {}", namespace, name, netbox_site.id);
        Ok(())
    }
}

