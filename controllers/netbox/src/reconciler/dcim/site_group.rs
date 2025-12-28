//! NetBoxSiteGroup reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxSiteGroup, ResourceState};
use netbox_client::{NetBoxClientTrait, SiteGroupId};

impl Reconciler {
    pub async fn reconcile_netbox_site_group(&self, site_group_crd: &NetBoxSiteGroup) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::{extract_name_and_namespace, resolve_optional_dependency_id};
        let (name, namespace) = extract_name_and_namespace(site_group_crd, "NetBoxSiteGroup")?;
        
        info!("Reconciling NetBoxSiteGroup {}/{}", namespace, name);
        
        // Get client for shared resource (finds tenant from referencing Sites)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxSiteGroup", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Resolve optional parent site group ID using helper
        let parent_id: Option<u64> = resolve_optional_dependency_id(
            &*self.netbox_site_group_api,
            site_group_crd.spec.parent.as_ref(),
            "NetBoxSiteGroup",
            "parent",
            name,
            |crd| crd.status.as_ref(),
        ).await;
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                site_group_crd.status.as_ref(),
                "NetBoxSiteGroup",
                namespace,
                name,
                |netbox_id: u64| async move {
                    netbox_client_ref.get_site_group(SiteGroupId(netbox_id)).await
                },
            ).await?
        };
        
        let netbox_site_group = match drift_result {
            DriftCheckResult::UseExisting(site_group) => {
                // Resource exists and is up-to-date
                Some(site_group)
            }
            DriftCheckResult::StatusCleared { message } => {
                // Status was cleared - update it to Pending
                let status_patch = Self::create_typed_site_group_status_patch(
                    0, // Clear netbox_id
                    String::new(), // Clear URL
                    ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_site_group_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxSiteGroup status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        // Handle existing site group (from helper) or create new
        let netbox_site_group = match netbox_site_group {
            Some(site_group) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    site_group_crd.status.as_ref(),
                    site_group.id,
                    &site_group.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    use crate::reconcile_helpers::update_resource_status;
                    let status_patch = Self::create_typed_site_group_status_patch(
                        site_group.id,
                        site_group.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    update_resource_status(
                        &*self.netbox_site_group_api,
                        name,
                        namespace,
                        &status_patch,
                        "NetBoxSiteGroup",
                        site_group.id,
                    ).await?;
                    debug!("Updated NetBoxSiteGroup {}/{} status: NetBox ID {}", namespace, name, site_group.id);
                    return Ok(());
                } else {
                    debug!("NetBoxSiteGroup {}/{} already has correct status (ID: {}), skipping update", namespace, name, site_group.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create site group - try to find existing by name (idempotency fallback)
                let existing_site_group = match netbox_client.get_site_group_by_name(&site_group_crd.spec.name).await {
                    Ok(Some(sg)) => {
                        info!("SiteGroup {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", site_group_crd.spec.name, sg.id);
                        Some(sg)
                    }
                    Ok(None) => None,
                    Err(e) => {
                        warn!("Failed to query site group by name: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(existing) = existing_site_group {
                    existing
                } else {
                    // Create site group
                    debug!("Attempting to create site group {} in NetBox", site_group_crd.spec.name);
                    match netbox_client.create_site_group(
                        &site_group_crd.spec.name,
                        site_group_crd.spec.slug.as_deref(),
                        parent_id.map(SiteGroupId),
                        site_group_crd.spec.description.clone(),
                        None, // comments - not in CRD spec yet
                    ).await {
                        Ok(created) => {
                            info!("Created site group {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            use crate::reconcile_helpers::is_conflict_error;

                            if is_conflict_error(&e) {
                                warn!("SiteGroup {} creation conflicted, attempting idempotent lookup", site_group_crd.spec.name);

                                // Strategy 1: by name
                                let mut found_site_group = match netbox_client.get_site_group_by_name(&site_group_crd.spec.name).await {
                                    Ok(Some(sg)) => Some(sg),
                                    _ => None,
                                };

                                // Strategy 2: by slug if provided
                                if found_site_group.is_none() {
                                    if let Some(slug) = &site_group_crd.spec.slug {
                                        if let Ok(site_groups) = netbox_client.query_site_groups(&[("slug", slug)], false).await {
                                            if let Some(sg) = site_groups.first() {
                                                info!("Found existing site group by slug '{}' in NetBox (ID: {}) after conflict", slug, sg.id);
                                                found_site_group = Some(sg.clone());
                                            }
                                        }
                                    }
                                }

                                // Strategy 3: fallback query all and filter
                                if found_site_group.is_none() {
                                    if let Ok(all_site_groups) = netbox_client.query_site_groups(&[], true).await {
                                        if let Some(sg) = all_site_groups.iter().find(|sg| {
                                            let slug_match = site_group_crd
                                                .spec
                                                .slug
                                                .as_ref()
                                                .map(|spec_slug| sg.slug == *spec_slug)
                                                .unwrap_or(false);
                                            sg.name == site_group_crd.spec.name || slug_match
                                        }) {
                                            info!("Found existing site group in NetBox (ID: {}) via fallback query", sg.id);
                                            found_site_group = Some(sg.clone());
                                        }
                                    }
                                }

                                if let Some(found) = found_site_group {
                                    found
                                } else {
                                    let error_msg = format!("SiteGroup {} already exists in NetBox but could not retrieve it: {}", site_group_crd.spec.name, e);
                                    error!("{}", error_msg);
                                    return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                                }
                            } else {
                                let error_msg = format!("Failed to create site group in NetBox: {}", e);
                                error!("{}", error_msg);
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                }
            }
        };
        
        // Update status using helper
        use crate::reconcile_helpers::update_resource_status;
        let status_patch = Self::create_typed_site_group_status_patch(
            netbox_site_group.id,
            netbox_site_group.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_site_group_api,
            name,
            namespace,
            &status_patch,
            "NetBoxSiteGroup",
            netbox_site_group.id,
        ).await?;
        info!("Updated NetBoxSiteGroup {}/{} status: NetBox ID {}", namespace, name, netbox_site_group.id);
        Ok(())
    }
}
