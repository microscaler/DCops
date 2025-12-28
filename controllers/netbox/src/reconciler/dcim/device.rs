//! NetBoxDevice reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxDevice, ResourceState};
use netbox_client::{NetBoxClientTrait, DeviceId, DeviceTypeId, DeviceRoleId, SiteId, TenantId, PlatformId, LocationId, IpAddressId};

impl Reconciler {
    pub async fn reconcile_netbox_device(&self, device_crd: &NetBoxDevice) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::extract_name_and_namespace;
        let (name, namespace) = extract_name_and_namespace(device_crd, "NetBoxDevice")?;
        let tenant_ref = &device_crd.spec.tenant;
        
        // SINGLE POINT: Get tenant-specific client
        let netbox_client = self.token_resolver
            .create_client_for_tenant(namespace, tenant_ref)
            .await?;
        
        info!("Reconciling NetBoxDevice {}/{}", namespace, name);
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                device_crd.status.as_ref(),
                "NetBoxDevice",
                namespace,
                name,
                |netbox_id| async move {
                    netbox_client_ref.get_device(DeviceId(netbox_id)).await
                },
            ).await?
        };
        
        let netbox_device = match drift_result {
            DriftCheckResult::UseExisting(device) => {
                // Resource exists and is up-to-date
                Some(device)
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
                if let Err(update_err) = self.netbox_device_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxDevice status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        // Handle existing device (from helper) or create new
        let netbox_device = match netbox_device {
            Some(device) => {
                // Resource exists and is up-to-date - only update status if it changed
                // Use trait-based helper to check if status needs updating
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
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
                        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
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
                // Need to create device - resolve dependencies first using helpers
                use crate::reconcile_helpers::{validate_reference_kind, resolve_required_dependency_id, resolve_optional_dependency_id};
                
                // Validate and resolve DeviceType (required)
                validate_reference_kind(&device_crd.spec.device_type, "NetBoxDeviceType", "device_type", name)?;
                let device_type_id = resolve_required_dependency_id(
                    &*self.netbox_device_type_api,
                    &device_crd.spec.device_type.name,
                    "DeviceType",
                    name,
                    |crd| crd.status.as_ref(),
                ).await?;
                
                // Validate and resolve DeviceRole (required)
                validate_reference_kind(&device_crd.spec.device_role, "NetBoxDeviceRole", "device_role", name)?;
                let device_role_id = resolve_required_dependency_id(
                    &*self.netbox_device_role_api,
                    &device_crd.spec.device_role.name,
                    "DeviceRole",
                    name,
                    |crd| crd.status.as_ref(),
                ).await?;
                
                // Validate and resolve Site (required)
                validate_reference_kind(&device_crd.spec.site, "NetBoxSite", "site", name)?;
                let site_id = resolve_required_dependency_id(
                    &*self.netbox_site_api,
                    &device_crd.spec.site.name,
                    "Site",
                    name,
                    |crd| crd.status.as_ref(),
                ).await?;
                
                // Validate and resolve Tenant (required)
                validate_reference_kind(&device_crd.spec.tenant, "NetBoxTenant", "tenant", name)?;
                let tenant_id = resolve_required_dependency_id(
                    &*self.netbox_tenant_api,
                    &device_crd.spec.tenant.name,
                    "Tenant",
                    name,
                    |crd| crd.status.as_ref(),
                ).await?;
                
                // Resolve optional dependencies using helper
                let platform_id = resolve_optional_dependency_id(
                    &*self.netbox_platform_api,
                    device_crd.spec.platform.as_ref(),
                    "NetBoxPlatform",
                    "platform",
                    name,
                    |crd| crd.status.as_ref(),
                ).await;
                
                let location_id = resolve_optional_dependency_id(
                    &*self.netbox_location_api,
                    device_crd.spec.location.as_ref(),
                    "NetBoxLocation",
                    "location",
                    name,
                    |crd| crd.status.as_ref(),
                ).await;
                
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
                    } else if let Some(ip_addr) = &ip_ref.ip_address {
                        // Query NetBox by IP address (fallback)
                        match netbox_client.query_ip_addresses(&[("address", ip_addr)], false).await {
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
                    } else {
                        None
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
                    } else if let Some(ip_addr) = &ip_ref.ip_address {
                        // Query NetBox by IP address (fallback)
                        match netbox_client.query_ip_addresses(&[("address", ip_addr)], false).await {
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
                    } else {
                        None
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
                let existing_device = match netbox_client.query_devices(
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
                    match netbox_client.create_device(
                        DeviceTypeId(device_type_id),
                        DeviceRoleId(device_role_id),
                        SiteId(site_id),
                        Some(device_name),
                        Some(TenantId(tenant_id)), // tenant is now required
                        platform_id.map(PlatformId),
                        location_id.map(LocationId),
                        device_crd.spec.serial.as_deref(),
                        device_crd.spec.asset_tag.as_deref(),
                        Some(status_str),
                        primary_ip4_id.map(IpAddressId),
                        primary_ip6_id.map(IpAddressId),
                        device_crd.spec.description.clone(),
                        device_crd.spec.comments.clone(),
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
                                    match netbox_client.query_devices(&[("asset_tag", asset_tag)], false).await {
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
                                    match netbox_client.query_devices(
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
                                    match netbox_client.query_devices(&[], true).await {
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
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
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
