//! NetBoxDevice reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers;
use tracing::{info, error, debug, warn};
use crds::{NetBoxDevice, ResourceState};

impl Reconciler {
    pub async fn reconcile_netbox_device(&self, device_crd: &NetBoxDevice) -> Result<(), ControllerError> {
        let name = device_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxDevice missing name".to_string()))?;
        let namespace = device_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxDevice {}/{}", namespace, name);
        
        // Check if already created - use helper for drift detection
        let netbox_device = if let Some(status) = &device_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Use simple helper function for drift detection (no update logic)
                    match reconcile_helpers::check_existing(
                        self.netbox_client.as_ref(),
                        netbox_id,
                        &format!("NetBoxDevice {}/{}", namespace, name),
                        self.netbox_client.get_device(netbox_id),
                    ).await {
                        Ok(Some(resource)) => {
                            // Resource exists and is up-to-date
                            Some(resource)
                        }
                        Ok(None) => {
                            // Drift detected - resource was deleted, clear status and recreate
                            warn!("NetBoxDevice {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_resource_status_patch(
                                0, // Clear netbox_id
                                String::new(), // Clear URL
                                ResourceState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_device_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                .await
                            {
                                warn!("Failed to clear NetBoxDevice status after drift detection: {}", e);
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
        
        // Handle existing device (from helper) or create new
        let netbox_device = match netbox_device {
            Some(device) => {
                // Resource exists and is up-to-date - only update status if it changed
                // Use trait-based helper to check if status needs updating
                let needs_status_update = reconcile_helpers::status_needs_update(
                    device_crd.status.as_ref(),
                    device.id,
                    &device.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        device.id,
                        device.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_device_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxDevice {}/{} status: NetBox ID {}", namespace, name, device.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxDevice status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    // Status is already correct - no update needed, skip reconciliation
                    debug!("NetBoxDevice {}/{} already has correct status (ID: {}), skipping update", namespace, name, device.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create device - resolve dependencies first
                // Validate and resolve dependencies
                if device_crd.spec.device_type.kind != "NetBoxDeviceType" {
                    return Err(ControllerError::InvalidConfig(
                        format!("Invalid kind '{}' for device_type reference in device {}, expected 'NetBoxDeviceType'", device_crd.spec.device_type.kind, name)
                    ));
                }
                let device_type_id = match self.netbox_device_type_api.get(&device_crd.spec.device_type.name).await {
                    Ok(device_type_crd) => {
                        device_type_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                            .ok_or_else(|| ControllerError::InvalidConfig(
                                format!("DeviceType '{}' has not been created in NetBox yet (no netbox_id in status)", device_crd.spec.device_type.name)
                            ))?
                    }
                    Err(_) => {
                        return Err(ControllerError::InvalidConfig(
                            format!("DeviceType CRD '{}' not found for device {}", device_crd.spec.device_type.name, name)
                        ));
                    }
                };
                
                if device_crd.spec.device_role.kind != "NetBoxDeviceRole" {
                    return Err(ControllerError::InvalidConfig(
                        format!("Invalid kind '{}' for device_role reference in device {}, expected 'NetBoxDeviceRole'", device_crd.spec.device_role.kind, name)
                    ));
                }
                let device_role_id = match self.netbox_device_role_api.get(&device_crd.spec.device_role.name).await {
                    Ok(role_crd) => {
                        role_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                            .ok_or_else(|| ControllerError::InvalidConfig(
                                format!("DeviceRole '{}' has not been created in NetBox yet (no netbox_id in status)", device_crd.spec.device_role.name)
                            ))?
                    }
                    Err(_) => {
                        return Err(ControllerError::InvalidConfig(
                            format!("DeviceRole CRD '{}' not found for device {}", device_crd.spec.device_role.name, name)
                        ));
                    }
                };
                
                if device_crd.spec.site.kind != "NetBoxSite" {
                    return Err(ControllerError::InvalidConfig(
                        format!("Invalid kind '{}' for site reference in device {}, expected 'NetBoxSite'", device_crd.spec.site.kind, name)
                    ));
                }
                let site_id = match self.netbox_site_api.get(&device_crd.spec.site.name).await {
                    Ok(site_crd) => {
                        site_crd.status
                            .as_ref()
                            .and_then(|s| s.netbox_id)
                            .ok_or_else(|| ControllerError::InvalidConfig(
                                format!("Site '{}' has not been created in NetBox yet (no netbox_id in status)", device_crd.spec.site.name)
                            ))?
                    }
                    Err(_) => {
                        return Err(ControllerError::InvalidConfig(
                            format!("Site CRD '{}' not found for device {}", device_crd.spec.site.name, name)
                        ));
                    }
                };
                
                let tenant_id = if let Some(tenant_ref) = &device_crd.spec.tenant {
                    if tenant_ref.kind != "NetBoxTenant" {
                        warn!("Invalid kind '{}' for tenant reference in device {}, expected 'NetBoxTenant'", tenant_ref.kind, name);
                        None
                    } else {
                        match self.netbox_tenant_api.get(&tenant_ref.name).await {
                            Ok(tenant_crd) => tenant_crd.status.as_ref().and_then(|s| s.netbox_id),
                            Err(_) => {
                                warn!("Tenant CRD '{}' not found for device {}", tenant_ref.name, name);
                                None
                            }
                        }
                    }
                } else {
                    None
                };
                
                let platform_id = if let Some(platform_ref) = &device_crd.spec.platform {
                    if platform_ref.kind != "NetBoxPlatform" {
                        warn!("Invalid kind '{}' for platform reference in device {}, expected 'NetBoxPlatform'", platform_ref.kind, name);
                        None
                    } else {
                        match self.netbox_platform_api.get(&platform_ref.name).await {
                            Ok(platform_crd) => platform_crd.status.as_ref().and_then(|s| s.netbox_id),
                            Err(_) => {
                                warn!("Platform CRD '{}' not found for device {}", platform_ref.name, name);
                                None
                            }
                        }
                    }
                } else {
                    None
                };
                
                let location_id = if let Some(location_ref) = &device_crd.spec.location {
                    if location_ref.kind != "NetBoxLocation" {
                        warn!("Invalid kind '{}' for location reference in device {}, expected 'NetBoxLocation'", location_ref.kind, name);
                        None
                    } else {
                        match self.netbox_location_api.get(&location_ref.name).await {
                            Ok(location_crd) => location_crd.status.as_ref().and_then(|s| s.netbox_id),
                            Err(_) => {
                                warn!("Location CRD '{}' not found for device {}", location_ref.name, name);
                                None
                            }
                        }
                    }
                } else {
                    None
                };
                
                // Resolve primary IP addresses (if specified)
                let primary_ip4_id = if let Some(ip_ref) = &device_crd.spec.primary_ip4 {
                    if let Some(claim_ref) = &ip_ref.ip_claim_ref {
                            // Resolve IPClaim CRD reference to get NetBox IP address ID
                            if claim_ref.kind != "IPClaim" {
                                warn!("Invalid kind '{}' for primary_ip4 IPClaim reference in device {}, expected 'IPClaim'", claim_ref.kind, name);
                                None
                            } else {
                                let claim_namespace = claim_ref.namespace.as_deref()
                                    .unwrap_or_else(|| device_crd.metadata.namespace.as_deref().unwrap_or("default"));
                                
                                match self.ip_claim_api.get(&claim_ref.name).await {
                                    Ok(claim_crd) => {
                                        // Get the NetBox IP address ID from the claim's status
                                        // The claim's netbox_ip_ref contains the URL, we need to extract the ID
                                        if let Some(status) = &claim_crd.status {
                                            if let Some(ip_url) = &status.netbox_ip_ref {
                                                // Extract ID from URL (e.g., "http://netbox/api/ipam/ip-addresses/123/")
                                                if let Some(id_str) = ip_url.split('/').nth_back(1) {
                                                    if let Ok(id) = id_str.parse::<u64>() {
                                                        debug!("Resolved primary_ip4 from IPClaim {}/{} to NetBox IP ID {}", claim_namespace, claim_ref.name, id);
                                                        Some(id)
                                                    } else {
                                                        warn!("Failed to parse IP ID from IPClaim netbox_ip_ref URL: {}", ip_url);
                                                        None
                                                    }
                                                } else {
                                                    warn!("Failed to extract IP ID from IPClaim netbox_ip_ref URL: {}", ip_url);
                                                    None
                                                }
                                            } else {
                                                warn!("IPClaim {}/{} has no netbox_ip_ref in status (not allocated yet)", claim_namespace, claim_ref.name);
                                                None
                                            }
                                        } else {
                                            warn!("IPClaim {}/{} has no status (not allocated yet)", claim_namespace, claim_ref.name);
                                            None
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to get IPClaim {}/{} for primary_ip4: {}", claim_namespace, claim_ref.name, e);
                                        None
                                    }
                                }
                            }
                        }
                        crds::PrimaryIPReference::IPAddress(ip_addr) => {
                            // Query NetBox by IP address (fallback)
                            match self.netbox_client.query_ip_addresses(&[("address", ip_addr)], false).await {
                                Ok(ips) => {
                                    if let Some(ip) = ips.first() {
                                        debug!("Resolved primary_ip4 from IP address {} to NetBox IP ID {}", ip_addr, ip.id);
                                        Some(ip.id)
                                    } else {
                                        warn!("IP address {} not found in NetBox", ip_addr);
                                        None
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to query IP address {} in NetBox: {}", ip_addr, e);
                                    None
                                }
                            }
                        }
                    }
                } else {
                    None
                };
                
                let primary_ip6_id = if let Some(ip_ref) = &device_crd.spec.primary_ip6 {
                    if let Some(claim_ref) = &ip_ref.ip_claim_ref {
                            // Resolve IPClaim CRD reference to get NetBox IP address ID
                            if claim_ref.kind != "IPClaim" {
                                warn!("Invalid kind '{}' for primary_ip6 IPClaim reference in device {}, expected 'IPClaim'", claim_ref.kind, name);
                                None
                            } else {
                                let claim_namespace = claim_ref.namespace.as_deref()
                                    .unwrap_or_else(|| device_crd.metadata.namespace.as_deref().unwrap_or("default"));
                                
                                match self.ip_claim_api.get(&claim_ref.name).await {
                                    Ok(claim_crd) => {
                                        // Get the NetBox IP address ID from the claim's status
                                        if let Some(status) = &claim_crd.status {
                                            if let Some(ip_url) = &status.netbox_ip_ref {
                                                // Extract ID from URL
                                                if let Some(id_str) = ip_url.split('/').nth_back(1) {
                                                    if let Ok(id) = id_str.parse::<u64>() {
                                                        debug!("Resolved primary_ip6 from IPClaim {}/{} to NetBox IP ID {}", claim_namespace, claim_ref.name, id);
                                                        Some(id)
                                                    } else {
                                                        warn!("Failed to parse IP ID from IPClaim netbox_ip_ref URL: {}", ip_url);
                                                        None
                                                    }
                                                } else {
                                                    warn!("Failed to extract IP ID from IPClaim netbox_ip_ref URL: {}", ip_url);
                                                    None
                                                }
                                            } else {
                                                warn!("IPClaim {}/{} has no netbox_ip_ref in status (not allocated yet)", claim_namespace, claim_ref.name);
                                                None
                                            }
                                        } else {
                                            warn!("IPClaim {}/{} has no status (not allocated yet)", claim_namespace, claim_ref.name);
                                            None
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to get IPClaim {}/{} for primary_ip6: {}", claim_namespace, claim_ref.name, e);
                                        None
                                    }
                                }
                            }
                        }
                        crds::PrimaryIPReference::IPAddress(ip_addr) => {
                            // Query NetBox by IP address (fallback)
                            match self.netbox_client.query_ip_addresses(&[("address", ip_addr)], false).await {
                                Ok(ips) => {
                                    if let Some(ip) = ips.first() {
                                        debug!("Resolved primary_ip6 from IP address {} to NetBox IP ID {}", ip_addr, ip.id);
                                        Some(ip.id)
                                    } else {
                                        warn!("IP address {} not found in NetBox", ip_addr);
                                        None
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to query IP address {} in NetBox: {}", ip_addr, e);
                                    None
                                }
                            }
                        }
                    }
                } else {
                    None
                };
                
                // Convert device status to NetBox format
                let status_str = match device_crd.spec.status {
                    crds::DeviceStatus::Active => "active",
                    crds::DeviceStatus::Offline => "offline",
                    crds::DeviceStatus::Planned => "planned",
                    crds::DeviceStatus::Staged => "staged",
                    crds::DeviceStatus::Failed => "failed",
                    crds::DeviceStatus::Inventory => "inventory",
                    crds::DeviceStatus::Decommissioning => "decommissioning",
                };
                
                // Try to find existing device by name
                let existing_device = match self.netbox_client.query_devices(
                    &[("name", device_crd.spec.name.as_deref().unwrap_or(name))],
                    false,
                ).await {
                    Ok(devices) => devices.first().cloned(),
                    Err(_) => None
                };
                
                let netbox_device = if let Some(existing) = existing_device {
                    info!("Device {} already exists in NetBox (ID: {})", device_crd.spec.name.as_deref().unwrap_or(name), existing.id);
                    existing
                } else {
                    let device_name = device_crd.spec.name.as_deref().ok_or_else(|| {
                        ControllerError::InvalidConfig("Device name is required".to_string())
                    })?;
                    match self.netbox_client.create_device(
                        device_name,
                        device_type_id,
                        device_role_id,
                        site_id,
                        location_id,
                        tenant_id,
                        platform_id,
                        device_crd.spec.serial.as_deref(),
                        device_crd.spec.asset_tag.as_deref(),
                        status_str,
                        primary_ip4_id,
                        primary_ip6_id,
                        device_crd.spec.description.as_deref(),
                        device_crd.spec.comments.as_deref(),
                    ).await {
                        Ok(created) => {
                            info!("Created device {} in NetBox (ID: {})", device_crd.spec.name.as_deref().unwrap_or("<unnamed>"), created.id);
                            created
                        }
                        Err(e) => {
                            // Check if device already exists (idempotency)
                            let error_str = format!("{}", e);
                            if error_str.contains("already exists") || error_str.contains("asset tag") {
                                warn!("Device {} already exists in NetBox, attempting to retrieve it (idempotency)", device_crd.spec.name.as_deref().unwrap_or(name));
                                
                                // Try to find existing device by asset_tag or name
                                let mut found_device = None;
                                
                                // First try: query by asset_tag
                                if let Some(asset_tag) = &device_crd.spec.asset_tag {
                                    match self.netbox_client.query_devices(&[("asset_tag", asset_tag)], false).await {
                                        Ok(devices) => {
                                            if let Some(device) = devices.first() {
                                                info!("Found existing device by asset_tag '{}' in NetBox (ID: {})", asset_tag, device.id);
                                                found_device = Some(device.clone());
                                            } else {
                                                warn!("Query by asset_tag '{}' returned no devices", asset_tag);
                                            }
                                        }
                                        Err(query_err) => {
                                            warn!("Query by asset_tag '{}' failed: {}, trying fallback", asset_tag, query_err);
                                        }
                                    }
                                }
                                
                                // Second try: query by name if not found by asset_tag
                                if found_device.is_none() {
                                    let device_name = device_crd.spec.name.as_deref().unwrap_or(name);
                                    match self.netbox_client.query_devices(
                                        &[("name", device_name)],
                                        false,
                                    ).await {
                                        Ok(devices) => {
                                            if let Some(device) = devices.first() {
                                                info!("Found existing device by name '{}' in NetBox (ID: {})", device_name, device.id);
                                                found_device = Some(device.clone());
                                            } else {
                                                warn!("Query by name '{}' returned no devices, trying fallback: query all devices", device_name);
                                            }
                                        }
                                        Err(query_err) => {
                                            warn!("Query by name '{}' failed: {}, trying fallback: query all devices", device_name, query_err);
                                        }
                                    }
                                }
                                
                                // Third try: fallback - query all devices and filter
                                if found_device.is_none() {
                                    warn!("Fallback: querying all devices to find existing device");
                                    match self.netbox_client.query_devices(&[], true).await {
                                        Ok(all_devices) => {
                                            // Try to match by asset_tag first, then by name
                                            let matched = if let Some(asset_tag) = &device_crd.spec.asset_tag {
                                                all_devices.iter().find(|d| {
                                                    d.asset_tag.as_ref().map(|at| at == asset_tag).unwrap_or(false)
                                                })
                                            } else {
                                                None
                                            };
                                            
                                            let matched = matched.or_else(|| {
                                                let device_name = device_crd.spec.name.as_deref().unwrap_or(name);
                                                all_devices.iter().find(|d| {
                                                    d.name.as_deref().map(|n| n == device_name).unwrap_or(false)
                                                })
                                            });
                                            
                                            if let Some(device) = matched {
                                                info!("Found existing device in NetBox (ID: {}) via fallback query", device.id);
                                                found_device = Some(device.clone());
                                            } else {
                                                warn!("Fallback query returned {} devices but none matched asset_tag '{:?}' or name '{}'", 
                                                    all_devices.len(), 
                                                    device_crd.spec.asset_tag, 
                                                    device_crd.spec.name.as_deref().unwrap_or(name)
                                                );
                                            }
                                        }
                                        Err(fallback_err) => {
                                            warn!("Fallback query for all devices failed: {}", fallback_err);
                                        }
                                    }
                                }
                                
                                if let Some(found) = found_device {
                                    info!("Found existing device {} in NetBox (ID: {}) via idempotency query", found.name.as_deref().unwrap_or("<unnamed>"), found.id);
                                    found
                                } else {
                                    let error_msg = format!("Device {} already exists in NetBox but could not retrieve it: {}", device_crd.spec.name.as_deref().unwrap_or(name), e);
                                    error!("{}", error_msg);
                                    return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                                }
                            } else {
                                let error_msg = format!("Failed to create device in NetBox: {}", e);
                                error!("{}", error_msg);
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                };
                
                netbox_device
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_device.id,
            netbox_device.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_device_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxDevice {}/{} status: NetBox ID {}", namespace, name, netbox_device.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxDevice status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
