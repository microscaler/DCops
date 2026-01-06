//! NetBoxDevice reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxDevice, ResourceState};
use netbox_client::{NetBoxClientTrait, DeviceId, DeviceTypeId, DeviceRoleId, SiteId, TenantId, PlatformId, LocationId, IpAddressId};

impl Reconciler {
    fn device_needs_update(
        spec: &crds::NetBoxDeviceSpec,
        existing: &netbox_client::Device,
        desired_device_type_id: u64,
        desired_device_role_id: u64,
        desired_site_id: u64,
        desired_location_id: Option<u64>,
        desired_tenant_id: u64,
        desired_platform_id: Option<u64>,
        desired_primary_ip4_id: Option<u64>,
        desired_primary_ip6_id: Option<u64>,
    ) -> bool {
        use crate::reconcile_helpers::{
            compare_optional_string_field,
            compare_optional_dependency_id,
            compare_enum_field,
        };
        
        let existing_device_type_id = existing.device_type.id;
        let existing_device_role_id = existing.device_role.as_ref().map(|r| r.id);
        let existing_site_id = existing.site.as_ref().map(|s| s.id);
        let existing_location_id = existing.location.as_ref().map(|l| l.id);
        let existing_tenant_id = existing.tenant.as_ref().map(|t| t.id);
        let existing_platform_id = existing.platform.as_ref().map(|p| p.id);
        let existing_primary_ip4_id = existing.primary_ip4.as_ref().map(|ip| ip.id);
        let existing_primary_ip6_id = existing.primary_ip6.as_ref().map(|ip| ip.id);
        
        // Convert DeviceStatus enum for comparison
        let existing_status = match existing.status {
            netbox_client::DeviceStatus::Active => crds::DeviceStatus::Active,
            netbox_client::DeviceStatus::Offline => crds::DeviceStatus::Offline,
            netbox_client::DeviceStatus::Planned => crds::DeviceStatus::Planned,
            netbox_client::DeviceStatus::Staged => crds::DeviceStatus::Staged,
            netbox_client::DeviceStatus::Failed => crds::DeviceStatus::Failed,
            netbox_client::DeviceStatus::Inventory => crds::DeviceStatus::Inventory,
            netbox_client::DeviceStatus::Decommissioning => crds::DeviceStatus::Decommissioning,
        };
        
        // Evaluate all comparisons to log all field differences (no short-circuit)
        let name_diff = compare_optional_string_field(&spec.name, &existing.name);
        let device_type_diff = existing_device_type_id != desired_device_type_id;
        let device_role_diff = compare_optional_dependency_id(Some(desired_device_role_id), existing_device_role_id);
        let site_diff = compare_optional_dependency_id(Some(desired_site_id), existing_site_id);
        let location_diff = compare_optional_dependency_id(desired_location_id, existing_location_id);
        let tenant_diff = compare_optional_dependency_id(Some(desired_tenant_id), existing_tenant_id);
        let platform_diff = compare_optional_dependency_id(desired_platform_id, existing_platform_id);
        let serial_diff = compare_optional_string_field(&spec.serial, &existing.serial);
        let asset_tag_diff = compare_optional_string_field(&spec.asset_tag, &existing.asset_tag);
        let status_diff = compare_enum_field(&spec.status, &existing_status);
        let primary_ip4_diff = compare_optional_dependency_id(desired_primary_ip4_id, existing_primary_ip4_id);
        let primary_ip6_diff = compare_optional_dependency_id(desired_primary_ip6_id, existing_primary_ip6_id);
        let description_diff = compare_optional_string_field(&spec.description, &existing.description);
        let comments_diff = compare_optional_string_field(&spec.comments, &existing.comments);
        // Tags are handled separately
        
        name_diff || device_type_diff || device_role_diff || site_diff || location_diff || tenant_diff || platform_diff || serial_diff || asset_tag_diff || status_diff || primary_ip4_diff || primary_ip6_diff || description_diff || comments_diff
    }

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
        
        // Resolve dependencies once at the top level (needed for both drift detection and creation)
        use crate::reconcile_helpers::{validate_reference_kind, resolve_required_dependency_id, resolve_optional_dependency_id};
        
        // Validate and resolve DeviceType (required)
        validate_reference_kind(&device_crd.spec.device_type, "NetBoxDeviceType", "device_type", name)?;
        let device_type_id = match resolve_required_dependency_id(
            &*self.netbox_device_type_api,
            &device_crd.spec.device_type.name,
            "DeviceType",
            name,
            |crd| crd.status.as_ref(),
        ).await {
            Ok(id) => id,
            Err(e) => {
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DEPENDENCY_NOT_FOUND,
                    &format!("DeviceType '{}' not found or not ready: {}", device_crd.spec.device_type.name, e),
                    device_crd,
                ).await;
                return Err(e);
            }
        };
        
        // Validate and resolve DeviceRole (required)
        validate_reference_kind(&device_crd.spec.device_role, "NetBoxDeviceRole", "device_role", name)?;
        let device_role_id = match resolve_required_dependency_id(
            &*self.netbox_device_role_api,
            &device_crd.spec.device_role.name,
            "DeviceRole",
            name,
            |crd| crd.status.as_ref(),
        ).await {
            Ok(id) => id,
            Err(e) => {
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DEPENDENCY_NOT_FOUND,
                    &format!("DeviceRole '{}' not found or not ready: {}", device_crd.spec.device_role.name, e),
                    device_crd,
                ).await;
                return Err(e);
            }
        };
        
        // Validate and resolve Site (required)
        validate_reference_kind(&device_crd.spec.site, "NetBoxSite", "site", name)?;
        let site_id = match resolve_required_dependency_id(
            &*self.netbox_site_api,
            &device_crd.spec.site.name,
            "Site",
            name,
            |crd| crd.status.as_ref(),
        ).await {
            Ok(id) => id,
            Err(e) => {
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DEPENDENCY_NOT_FOUND,
                    &format!("Site '{}' not found or not ready: {}", device_crd.spec.site.name, e),
                    device_crd,
                ).await;
                return Err(e);
            }
        };
        
        // Validate and resolve Tenant (required)
        validate_reference_kind(&device_crd.spec.tenant, "NetBoxTenant", "tenant", name)?;
        let tenant_id = match resolve_required_dependency_id(
            &*self.netbox_tenant_api,
            &device_crd.spec.tenant.name,
            "Tenant",
            name,
            |crd| crd.status.as_ref(),
        ).await {
            Ok(id) => id,
            Err(e) => {
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DEPENDENCY_NOT_FOUND,
                    &format!("Tenant '{}' not found or not ready: {}", device_crd.spec.tenant.name, e),
                    device_crd,
                ).await;
                return Err(e);
            }
        };
        
        // Resolve optional dependencies
        let platform_id: Option<u64> = resolve_optional_dependency_id(
            &*self.netbox_platform_api,
            device_crd.spec.platform.as_ref(),
            "NetBoxPlatform",
            "platform",
            name,
            |crd| crd.status.as_ref(),
        ).await;
        
        let location_id: Option<u64> = resolve_optional_dependency_id(
            &*self.netbox_location_api,
            device_crd.spec.location.as_ref(),
            "NetBoxLocation",
            "location",
            name,
            |crd| crd.status.as_ref(),
        ).await;
        
        // Resolve primary IP addresses (if specified)
        let primary_ip4_id = if let Some(ip_ref) = &device_crd.spec.primary_ip4 {
            if let Some(ip_addr) = &ip_ref.ip_address {
                match netbox_client.query_ip_addresses(&[("address", ip_addr)], false).await {
                    Ok(ips) => ips.first().map(|ip| ip.id),
                    Err(_) => None,
                }
            } else {
                None
            }
        } else {
            None
        };
        
        let primary_ip6_id = if let Some(ip_ref) = &device_crd.spec.primary_ip6 {
            if let Some(ip_addr) = &ip_ref.ip_address {
                match netbox_client.query_ip_addresses(&[("address", ip_addr)], false).await {
                    Ok(ips) => ips.first().map(|ip| ip.id),
                    Err(_) => None,
                }
            } else {
                None
            }
        } else {
            None
        };
        
        // Check if already created - use shared helper for drift detection and status validation
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                device_crd.status.as_ref(),
                "NetBoxDevice",
                namespace,
                name,
                |netbox_id: u64| async move {
                    netbox_client_ref.get_device(DeviceId(netbox_id)).await
                },
            ).await?
        };
        
        // Check if drift detection is enabled (defaults to true)
        let drift_detection_enabled = device_crd.spec.drift_detection.unwrap_or(true);
        
        let netbox_device = match drift_result {
            DriftCheckResult::UseExisting(device) => {
                // Check for field drift if enabled
                if drift_detection_enabled {
                    if Self::device_needs_update(
                        &device_crd.spec,
                        &device,
                        device_type_id,
                        device_role_id,
                        site_id,
                        location_id,
                        tenant_id,
                        platform_id,
                        primary_ip4_id,
                        primary_ip6_id,
                    ) {
                        // Field drift detected - update NetBox to match CRD (Git is source of truth)
                        warn!("NetBoxDevice {}/{} fields differ from CRD spec, updating to match Git", namespace, name);
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DRIFT_DETECTED,
                            &format!("NetBoxDevice {}/{} fields differ from CRD, updating to match Git", namespace, name),
                            device_crd,
                        ).await;
                        
                        // Resolve tags for update
                        let resolved_tags_json = self.resolve_tag_references(
                            netbox_client.as_ref(),
                            &device_crd.spec.tags,
                            namespace,
                            name,
                            Some(device.id),
                            "NetBoxDevice",
                        ).await;
                        let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                        
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
                        
                        match netbox_client.update_device(
                            DeviceId(device.id),
                            device_crd.spec.name.as_deref(),
                            Some(TenantId(tenant_id)),
                            platform_id.map(PlatformId),
                            location_id.map(LocationId),
                            device_crd.spec.serial.as_deref(),
                            device_crd.spec.asset_tag.as_deref(),
                            Some(status_str),
                            primary_ip4_id.map(IpAddressId),
                            primary_ip6_id.map(IpAddressId),
                            device_crd.spec.description.clone(),
                            device_crd.spec.comments.clone(),
                            resolved_tags,
                        ).await {
                            Ok(updated) => {
                                use crate::events::reasons;
                                self.record_event_normal(
                                    reasons::UPDATED,
                                    &format!("Updated NetBoxDevice {}/{} in NetBox to match CRD (ID: {})", namespace, name, updated.id),
                                    device_crd,
                                ).await;
                                // Tags are already updated via update_device call above, so we can skip the separate tag reconciliation here
                                // The later tag reconciliation step (line ~424) will handle any tag-only changes
                                Some(updated)
                            }
                            Err(e) => {
                                error!("Failed to update NetBoxDevice {}/{} in NetBox: {}", namespace, name, e);
                                Some(device) // Use existing if update fails
                            }
                        }
                    } else {
                        // No drift - use existing
                        Some(device)
                    }
                } else {
                    // Drift detection disabled - use existing
                    Some(device)
                }
            }
            DriftCheckResult::StatusCleared { message } => {
                // Status was cleared - drift detected
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxDevice {}/{} drift detected: {}", namespace, name, message),
                    device_crd,
                ).await;
                
                // Status was cleared - update it to Pending
                let status_patch = Self::create_resource_status_patch(
                    0,
                    String::new(),
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
                // Update tags if they differ (tags are handled separately from field drift)
                let device_id = device.id;
                let device_clone = device.clone();
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &device_crd.spec.tags,
                    namespace,
                    name,
                    Some(device_id),
                    "NetBoxDevice",
                ).await;
                let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                
                let device = match crate::reconcile_helpers::update_tags_if_differ(
                    device,
                    &device_crd.spec.tags,
                    resolved_tags.clone(),
                    |tags| {
                        let device_id_clone = device_id;
                        let name_clone = device_crd.spec.name.clone();
                        let tenant_id_clone = tenant_id;
                        let platform_id_clone = platform_id;
                        let location_id_clone = location_id;
                        let serial_clone = device_crd.spec.serial.clone();
                        let asset_tag_clone = device_crd.spec.asset_tag.clone();
                        let status_str = match device_crd.spec.status {
                            crds::DeviceStatus::Active => "active",
                            crds::DeviceStatus::Offline => "offline",
                            crds::DeviceStatus::Planned => "planned",
                            crds::DeviceStatus::Staged => "staged",
                            crds::DeviceStatus::Failed => "failed",
                            crds::DeviceStatus::Inventory => "inventory",
                            crds::DeviceStatus::Decommissioning => "decommissioning",
                        };
                        let primary_ip4_id_clone = primary_ip4_id;
                        let primary_ip6_id_clone = primary_ip6_id;
                        let description_clone = device_crd.spec.description.clone();
                        let comments_clone = device_crd.spec.comments.clone();
                        async move {
                            netbox_client.update_device(
                                DeviceId(device_id_clone),
                                name_clone.as_deref(),
                                Some(TenantId(tenant_id_clone)),
                                platform_id_clone.map(PlatformId),
                                location_id_clone.map(LocationId),
                                serial_clone.as_deref(),
                                asset_tag_clone.as_deref(),
                                Some(status_str),
                                primary_ip4_id_clone.map(IpAddressId),
                                primary_ip6_id_clone.map(IpAddressId),
                                description_clone,
                                comments_clone,
                                tags,
                            ).await
                        }
                    },
                    &format!("NetBoxDevice {}/{}", namespace, name),
                ).await {
                    Ok(Some(updated)) => {
                        use crate::events::reasons;
                        self.record_event_normal(
                            reasons::UPDATED,
                            &format!("Updated NetBoxDevice {}/{} tags in NetBox", namespace, name),
                            device_crd,
                        ).await;
                        updated
                    }
                    Ok(None) => device_clone, // Tags are up-to-date
                    Err(e) => {
                        warn!("Failed to update NetBoxDevice {}/{} tags: {}", namespace, name, e);
                        device_clone // Use existing if update fails
                    }
                };
                
                // Update status if needed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    device_crd.status.as_ref(),
                    device.id,
                    &device.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    use crate::reconcile_helpers::update_resource_status;
                    let status_patch = Self::create_resource_status_patch(
                        device.id,
                        device.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    update_resource_status(
                        &*self.netbox_device_api,
                        name,
                        namespace,
                        &status_patch,
                        "NetBoxDevice",
                        device.id,
                    ).await?;
                    debug!("Updated NetBoxDevice {}/{} status: NetBox ID {}", namespace, name, device.id);
                }
                return Ok(());
            }
            None => {
                // Need to create device - dependencies already resolved above
                
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
                            // Emit event for successful creation
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::CREATED,
                                &format!("Created device {} in NetBox (ID: {})", device_crd.spec.name.as_deref().unwrap_or("<unnamed>"), created.id),
                                device_crd,
                            ).await;
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
