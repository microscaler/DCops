//! NetBoxVLAN reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers;
use tracing::{info, error, debug, warn};
use crds::{NetBoxVLAN, ResourceState};

impl Reconciler {
    pub async fn reconcile_netbox_vlan(&self, vlan_crd: &NetBoxVLAN) -> Result<(), ControllerError> {
        let name = vlan_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxVLAN missing name".to_string()))?;
        let namespace = vlan_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxVLAN {}/{}", namespace, name);
        
        // Check if already created - use helper for drift detection
        let netbox_vlan = if let Some(status) = &vlan_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Use simple helper function for drift detection (no update logic)
                    match reconcile_helpers::check_existing(
                        self.netbox_client.as_ref(),
                        netbox_id,
                        &format!("NetBoxVLAN {}/{}", namespace, name),
                        self.netbox_client.get_vlan(netbox_id),
                    ).await {
                        Ok(Some(resource)) => {
                            // Resource exists and is up-to-date
                            Some(resource)
                        }
                        Ok(None) => {
                            // Drift detected - resource was deleted, clear status and recreate
                            warn!("NetBoxVLAN {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_resource_status_patch(
                                0, // Clear netbox_id
                                String::new(), // Clear URL
                                ResourceState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_vlan_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                .await
                            {
                                warn!("Failed to clear NetBoxVLAN status after drift detection: {}", e);
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
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
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
                
                // Resolve tenant ID if tenant reference provided
                let tenant_id = if let Some(tenant_ref) = &vlan_crd.spec.tenant {
                    if tenant_ref.kind != "NetBoxTenant" {
                        warn!("Invalid kind '{}' for tenant reference in VLAN {}, expected 'NetBoxTenant'", tenant_ref.kind, name);
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
                
                // Resolve role ID if role reference provided
                let role_id = if let Some(role_ref) = &vlan_crd.spec.role {
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
                let existing_vlan = match self.netbox_client.query_vlans(
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
                    match self.netbox_client.create_vlan(
                        site_id_value,
                        vlan_crd.spec.vid as u32,
                        &vlan_crd.spec.name,
                        status_str,
                        vlan_crd.spec.description.as_deref(),
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
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
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
