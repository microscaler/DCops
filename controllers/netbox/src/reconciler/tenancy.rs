//! Tenancy reconcilers
//! 
//! Handles: NetBoxTenant

use super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers;
use crds::{NetBoxTenant, ResourceState};
use kube::Api;
use tracing::{info, error, debug, warn};

impl Reconciler {
    /// Reconciles a NetBoxTenant resource.
    pub async fn reconcile_netbox_tenant(&self, tenant_crd: &NetBoxTenant) -> Result<(), ControllerError> {
        // Helper function to update status with error
        async fn update_status_error(
            api: &Api<NetBoxTenant>,
            name: &str,
            namespace: &str,
            error_msg: String,
            current_status: Option<&crds::NetBoxTenantStatus>,
        ) {
            if let Some(status) = current_status {
                if status.state == ResourceState::Failed && status.error.as_ref() == Some(&error_msg) {
                    debug!("NetBoxTenant {}/{} already has this error in status, skipping update", namespace, name);
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
                error!("Failed to update NetBoxTenant {}/{} error status: {}", namespace, name, e);
            } else {
                info!("Updated NetBoxTenant {}/{} status with error", namespace, name);
            }
        }
        
        let name = tenant_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxTenant missing name".to_string()))?;
        let namespace = tenant_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxTenant {}/{}", namespace, name);
        
        // Check if already created - use helper for drift detection
        // Note: Tenants don't have update logic yet, so we use the simple check_existing helper
        let netbox_tenant = if let Some(status) = &tenant_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Use simple helper function for drift detection (no update logic)
                    match reconcile_helpers::check_existing(
                        self.netbox_client.as_ref(),
                        netbox_id,
                        &format!("NetBoxTenant {}/{}", namespace, name),
                        self.netbox_client.get_tenant(netbox_id),
                    ).await {
                        Ok(Some(resource)) => {
                            // Resource exists and is up-to-date
                            Some(resource)
                        }
                        Ok(None) => {
                            // Drift detected - resource was deleted, clear status and recreate
                            warn!("NetBoxTenant {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_resource_status_patch(
                                0, // Clear netbox_id
                                String::new(), // Clear URL
                                ResourceState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_tenant_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                .await
                            {
                                warn!("Failed to clear NetBoxTenant status after drift detection: {}", e);
                            }
                            // Fall through to creation
                            None
                        }
                        Err(e) => {
                            // Error during drift detection - return to retry
                            return Err(e);
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
        
        // Handle existing tenant (from helper) or create new
        let netbox_tenant = match netbox_tenant {
            Some(tenant) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    tenant_crd.status.as_ref(),
                    tenant.id,
                    &tenant.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        tenant.id,
                        tenant.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_tenant_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxTenant {}/{} status: NetBox ID {}", namespace, name, tenant.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxTenant status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxTenant {}/{} already has correct status (ID: {}), skipping update", namespace, name, tenant.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create tenant - try to find existing by name (idempotency fallback)
                let existing_tenant = match self.netbox_client.query_tenants(
                    &[("name", &tenant_crd.spec.name)],
                    false,
                ).await {
                    Ok(tenants) => tenants.first().cloned(),
                    Err(e) => {
                        warn!("Failed to query tenants: {}, will try to create", e);
                        None
                    }
                };
                
                // Resolve tenant group ID if group reference provided
                // If no group is specified and NetBox requires one, create a default group
                info!("Resolving tenant group for tenant {}/{}", namespace, name);
                let group_id = if let Some(group_ref) = &tenant_crd.spec.group {
                    // Validate kind (NetBoxTenantGroup CRD not yet implemented, but we can validate)
                    if group_ref.kind != "NetBoxTenantGroup" {
                        warn!("Invalid kind '{}' for group reference in tenant {}, expected 'NetBoxTenantGroup'", group_ref.kind, name);
                        None
                    } else {
                        info!("Tenant group specified in CRD: '{}'", group_ref.name);
                        match self.netbox_client.get_tenant_group_by_name(&group_ref.name).await {
                            Ok(Some(group)) => {
                                info!("Resolved tenant group '{}' to ID {}", group_ref.name, group.id);
                                Some(group.id)
                            }
                            Ok(None) => {
                                warn!("Tenant group '{}' not found, will try to create default group", group_ref.name);
                                None
                            }
                            Err(e) => {
                                warn!("Failed to resolve tenant group '{}': {}, will try to create default group", group_ref.name, e);
                                None
                            }
                        }
                    }
                } else {
                    info!("No tenant group specified in CRD, checking for existing tenant groups");
                    // Check if any tenant groups exist, if not create a default one
                    match self.netbox_client.query_tenant_groups(&[], false).await {
                        Ok(groups) if !groups.is_empty() => {
                            // Use the first available tenant group
                            let group = &groups[0];
                            info!("Using existing tenant group '{}' (ID: {}) for tenant", group.name, group.id);
                            Some(group.id)
                        }
                        _ => {
                            // Create a default tenant group
                            info!("No tenant groups found, creating default tenant group 'Default'");
                            match self.netbox_client.create_tenant_group(
                                "Default",
                                "default", // Slug is required
                                Some("Default tenant group for DCops"),
                            ).await {
                                Ok(group) => {
                                    info!("Created default tenant group '{}' (ID: {})", group.name, group.id);
                                    Some(group.id)
                                }
                                Err(e) => {
                                    warn!("Failed to create default tenant group: {}, will try without group", e);
                                    None
                                }
                            }
                        }
                    }
                };
                
                let netbox_tenant = if let Some(existing) = existing_tenant {
                    info!("Tenant {} already exists in NetBox (ID: {})", tenant_crd.spec.name, existing.id);
                    existing
                } else {
                    // Create tenant
                    info!("Creating tenant {} in NetBox", tenant_crd.spec.name);
                    let slug = tenant_crd.spec.slug.as_deref().map(|s| s.to_string())
                        .unwrap_or_else(|| tenant_crd.spec.name.to_lowercase().replace(' ', "-"));
                    match self.netbox_client.create_tenant(
                        &tenant_crd.spec.name,
                        &slug,
                        group_id,
                        tenant_crd.spec.description.as_deref(),
                        tenant_crd.spec.comments.as_deref(),
                    ).await {
                        Ok(created) => {
                            info!("Created tenant {} in NetBox (ID: {})", created.name, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create tenant in NetBox: {}", e);
                            error!("{}", error_msg);
                            update_status_error(&self.netbox_tenant_api, name, namespace, error_msg.clone(), tenant_crd.status.as_ref()).await;
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                };
                
                netbox_tenant
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_tenant.id,
            netbox_tenant.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_tenant_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxTenant {}/{} status: NetBox ID {}", namespace, name, netbox_tenant.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxTenant status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
