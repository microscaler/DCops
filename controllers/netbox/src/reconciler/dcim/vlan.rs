//! NetBoxVLAN reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxVLAN, ResourceState};
use netbox_client::{NetBoxClientTrait, VlanId, SiteId, TenantId};

impl Reconciler {
    pub async fn reconcile_netbox_vlan(&self, vlan_crd: &NetBoxVLAN) -> Result<(), ControllerError> {
        // Extract namespace and tenant reference
        let namespace = vlan_crd.metadata.namespace.as_deref().unwrap_or("default");
        let tenant_ref = &vlan_crd.spec.tenant;
        
        // SINGLE POINT: Get tenant-specific client
        let netbox_client = self.token_resolver
            .create_client_for_tenant(namespace, tenant_ref)
            .await?;
        
        let name = vlan_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxVLAN missing name".to_string()))?;
        
        info!("Reconciling NetBoxVLAN {}/{}", namespace, name);
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                vlan_crd.status.as_ref(),
                "NetBoxVLAN",
                namespace,
                name,
                |netbox_id| async move {
                    netbox_client_ref.get_vlan(VlanId(netbox_id as u32)).await
                },
            ).await?
        };
        
        let netbox_vlan = match drift_result {
            DriftCheckResult::UseExisting(vlan) => {
                // Resource exists and is up-to-date
                Some(vlan)
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
                if let Err(update_err) = self.netbox_vlan_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxVLAN status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        // Handle existing VLAN (from helper) or create new
        let netbox_vlan = match netbox_vlan {
            Some(vlan) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    vlan_crd.status.as_ref(),
                    vlan.id,
                    &vlan.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        vlan.id,
                        vlan.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_vlan_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxVLAN {}/{} status: NetBox ID {}", namespace, name, vlan.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxVLAN status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxVLAN {}/{} already has correct status (ID: {}), skipping update", namespace, name, vlan.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create VLAN - resolve dependencies first
                // Resolve site ID if site reference provided
                let site_id = if let Some(site_ref) = &vlan_crd.spec.site {
                    if site_ref.kind != "NetBoxSite" {
                        warn!("Invalid kind '{}' for site reference in VLAN {}, expected 'NetBoxSite'", site_ref.kind, name);
                        None
                    } else {
                        match self.netbox_site_api.get(&site_ref.name).await {
                            Ok(site_crd) => {
                                site_crd.status
                                    .as_ref()
                                    .and_then(|s| s.netbox_id)
                            }
                            Err(_) => None
                        }
                    }
                } else {
                    None
                };
                
                // Resolve tenant ID (required)
                if vlan_crd.spec.tenant.kind != "NetBoxTenant" {
                    return Err(ControllerError::InvalidConfig(
                        format!("Invalid kind '{}' for tenant reference in VLAN {}, expected 'NetBoxTenant'", vlan_crd.spec.tenant.kind, name)
                    ));
                }
                let tenant_id = match self.netbox_tenant_api.get(&vlan_crd.spec.tenant.name).await {
                    Ok(tenant_crd) => {
                        tenant_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                            .ok_or_else(|| ControllerError::InvalidConfig(
                                format!("Tenant '{}' has not been created in NetBox yet (no netbox_id in status)", vlan_crd.spec.tenant.name)
                            ))?
                    }
                    Err(_) => {
                        return Err(ControllerError::InvalidConfig(
                            format!("Tenant CRD '{}' not found for VLAN {}", vlan_crd.spec.tenant.name, name)
                        ));
                    }
                };
                
                // Resolve role ID if role reference provided
                let _role_id = if let Some(role_ref) = &vlan_crd.spec.role {
                    if role_ref.kind != "NetBoxRole" {
                        warn!("Invalid kind '{}' for role reference in VLAN {}, expected 'NetBoxRole'", role_ref.kind, name);
                        None
                    } else {
                        match self.netbox_role_api.get(&role_ref.name).await {
                            Ok(role_crd) => {
                                role_crd.status
                                    .as_ref()
                                    .and_then(|s| s.netbox_id)
                            }
                            Err(_) => None
                        }
                    }
                } else {
                    None
                };
                
                // Convert status enum to string
                let status_str = match vlan_crd.spec.status {
                    crds::VlanStatus::Active => Some("active"),
                    crds::VlanStatus::Reserved => Some("reserved"),
                    crds::VlanStatus::Deprecated => Some("deprecated"),
                };
                
                // Try to find existing VLAN by VID
                let existing_vlan = match netbox_client.query_vlans(
                    &[("vid", &vlan_crd.spec.vid.to_string())],
                    false,
                ).await {
                    Ok(vlans) => vlans.first().cloned(),
                    Err(_) => None
                };
                
                let netbox_vlan = if let Some(existing) = existing_vlan {
                    info!("VLAN {} already exists in NetBox (ID: {})", vlan_crd.spec.vid, existing.id);
                    existing
                } else {
                    let site_id_value = site_id.ok_or_else(|| {
                        ControllerError::InvalidConfig("Site ID is required for VLAN".to_string())
                    })?;
                    match netbox_client.create_vlan(
                        vlan_crd.spec.vid,
                        &vlan_crd.spec.name,
                        Some(SiteId(site_id_value)),
                        None, // group_id
                        Some(TenantId(tenant_id)),
                        None, // role_id
                        status_str, // status_str is already Option<&str>
                        vlan_crd.spec.description.clone(),
                        None, // comments
                    ).await {
                        Ok(created) => {
                            info!("Created VLAN {} ({}) in NetBox (ID: {})", created.vid, created.name, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create VLAN in NetBox: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                        }
                    }
                };
                
                netbox_vlan
            }
        };
        
        // Update status (use PascalCase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_vlan.id,
            netbox_vlan.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_vlan_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxVLAN {}/{} status: NetBox ID {}", namespace, name, netbox_vlan.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxVLAN status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
