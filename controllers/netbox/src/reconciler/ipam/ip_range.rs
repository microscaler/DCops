//! NetBoxIPRange reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers::{extract_name_and_namespace, validate_status_and_drift, DriftCheckResult, update_resource_status, resolve_required_dependency_id, resolve_optional_dependency_id, validate_reference_kind, is_conflict_error};
use crate::kube_api_trait::KubeApiTrait;
use tracing::{info, error, debug, warn};
use crds::{NetBoxIPRange, ResourceState};
use netbox_client::{IPRangeId, IPRangeStatus};
use std::str::FromStr;
use ipnet::IpNet;

impl Reconciler {
    /// Check if IP range needs updating by comparing spec with existing NetBox resource
    fn ip_range_needs_update(
        spec: &crds::NetBoxIPRangeSpec,
        existing: &netbox_client::IPRange,
        desired_tenant_id: u64,
        desired_status: &str,
        desired_role_id: Option<u64>,
        desired_vrf_id: Option<u64>,
    ) -> bool {
        // Compare tenant
        let existing_tenant_id = existing.tenant.as_ref().map(|t| t.id);
        if Some(desired_tenant_id) != existing_tenant_id {
            debug!("IP range tenant changed: {:?} -> {}", existing_tenant_id, desired_tenant_id);
            return true;
        }
        
        // Compare role
        let existing_role_id = existing.role.as_ref().map(|r| r.id);
        if desired_role_id != existing_role_id {
            debug!("IP range role changed: {:?} -> {:?}", existing_role_id, desired_role_id);
            return true;
        }
        
        // Compare VRF
        let existing_vrf_id = existing.vrf.as_ref().map(|v| v.id);
        if desired_vrf_id != existing_vrf_id {
            debug!("IP range VRF changed: {:?} -> {:?}", existing_vrf_id, desired_vrf_id);
            return true;
        }
        
        // Compare status
        let existing_status = match existing.status {
            netbox_client::IPRangeStatus::Active => "active",
            netbox_client::IPRangeStatus::Reserved => "reserved",
            netbox_client::IPRangeStatus::Deprecated => "deprecated",
        };
        if desired_status != existing_status {
            debug!("IP range status changed: '{}' -> '{}'", existing_status, desired_status);
            return true;
        }
        
        // Compare description
        let spec_desc = spec.description.as_deref().unwrap_or("");
        if spec_desc != existing.description {
            debug!("IP range description changed: '{}' -> '{}'", existing.description, spec_desc);
            return true;
        }
        
        // Compare mark_utilized
        if spec.mark_utilized != existing.mark_utilized {
            debug!("IP range mark_utilized changed: {} -> {}", existing.mark_utilized, spec.mark_utilized);
            return true;
        }
        
        // Compare mark_populated
        if spec.mark_populated != existing.mark_populated {
            debug!("IP range mark_populated changed: {} -> {}", existing.mark_populated, spec.mark_populated);
            return true;
        }
        
        // Compare tags using helper function
        if crate::reconcile_helpers::tags_differ(&existing.tags, &spec.tags) {
            return true;
        }
        
        false // No changes needed
    }

    pub async fn reconcile_netbox_ip_range(&self, ip_range_crd: &NetBoxIPRange) -> Result<(), ControllerError> {
        let (name, namespace) = extract_name_and_namespace(ip_range_crd, "NetBoxIPRange")?;
        let tenant_ref = &ip_range_crd.spec.tenant;
        
        info!("Reconciling NetBoxIPRange {}/{}", namespace, name);
        
        // Get tenant-specific client
        let netbox_client = self.token_resolver
            .create_client_for_tenant(namespace, tenant_ref)
            .await?;
        
        // Helper function to update status with error
        async fn update_status_error(
            api: &dyn KubeApiTrait<NetBoxIPRange>,
            name: &str,
            namespace: &str,
            error_msg: String,
            current_status: Option<&crds::NetBoxIPRangeStatus>,
        ) {
            if let Some(status) = current_status {
                if status.state == ResourceState::Failed && status.error.as_ref() == Some(&error_msg) {
                    debug!("NetBoxIPRange {}/{} already has this error in status, skipping update", namespace, name);
                    return;
                }
            }
            
            let status_patch = Reconciler::create_resource_status_patch(
                0,
                String::new(),
                ResourceState::Failed,
                Some(error_msg.clone()),
            );
            let pp = kube::api::PatchParams::default();
            if let Err(e) = api.patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone())).await {
                error!("Failed to update NetBoxIPRange {}/{} error status: {}", namespace, name, e);
            } else {
                info!("Updated NetBoxIPRange {}/{} status with error", namespace, name);
            }
        }
        
        // Parse IP addresses from spec
        let start_ip_net = IpNet::from_str(&ip_range_crd.spec.start_address)
            .map_err(|e| ControllerError::InvalidInput(format!("Invalid start IP address format '{}': {}", ip_range_crd.spec.start_address, e)))?;
        let end_ip_net = IpNet::from_str(&ip_range_crd.spec.end_address)
            .map_err(|e| ControllerError::InvalidInput(format!("Invalid end IP address format '{}': {}", ip_range_crd.spec.end_address, e)))?;
        
        // Validate that start and end are in the same family
        if start_ip_net.addr().is_ipv4() != end_ip_net.addr().is_ipv4() {
            let error_msg = format!("Start and end addresses must be in the same family (both IPv4 or both IPv6)");
            update_status_error(&*self.netbox_ip_range_api, name, namespace, error_msg.clone(), ip_range_crd.status.as_ref()).await;
            return Err(ControllerError::InvalidInput(error_msg));
        }
        
        // Validate status and check for drift
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                ip_range_crd.status.as_ref(),
                "NetBoxIPRange",
                namespace,
                name,
                |netbox_id: u64| async move {
                    netbox_client_ref.get_ip_range(IPRangeId(netbox_id)).await
                },
            ).await?
        };
        
        let netbox_ip_range = match drift_result {
            DriftCheckResult::UseExisting(existing_range) => {
                // Resource exists - check if it needs updating
                // Resolve dependencies for comparison
                validate_reference_kind(&ip_range_crd.spec.tenant, "NetBoxTenant", "tenant", name)?;
                let tenant_id = resolve_required_dependency_id(
                    &*self.netbox_tenant_api,
                    &ip_range_crd.spec.tenant.name,
                    "NetBoxTenant",
                    name,
                    |crd| crd.status.as_ref(),
                ).await?;
                
                let role_id: Option<netbox_client::RoleId> = if let Some(role_ref) = &ip_range_crd.spec.role {
                    validate_reference_kind(role_ref, "NetBoxRole", "role", name)?;
                    resolve_optional_dependency_id(
                        &*self.netbox_role_api,
                        Some(role_ref),
                        "NetBoxRole",
                        "role",
                        name,
                        |crd| crd.status.as_ref(),
                    ).await.map(|id| netbox_client::RoleId(id))
                } else {
                    None
                };
                
                // Resolve VRF if specified
                let vrf_id: Option<u64> = if let Some(vrf_ref) = &ip_range_crd.spec.vrf {
                    validate_reference_kind(vrf_ref, "NetBoxVRF", "vrf", name)?;
                    Some(resolve_optional_dependency_id(
                        &*self.netbox_vrf_api,
                        Some(vrf_ref),
                        "NetBoxVRF",
                        "vrf",
                        name,
                        |crd| crd.status.as_ref(),
                    ).await.ok_or_else(|| ControllerError::InvalidConfig(format!("VRF '{}' not found", vrf_ref.name)))?)
                } else {
                    None
                };
                
                // Convert status enum to string
                let status_str = match ip_range_crd.spec.status {
                    crds::IPRangeStatus::Active => "active",
                    crds::IPRangeStatus::Reserved => "reserved",
                    crds::IPRangeStatus::Deprecated => "deprecated",
                };
                
                // Always resolve tags (even if nothing else changed, tags might need updating)
                let resolved_tags_json = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &ip_range_crd.spec.tags,
                    namespace,
                    name,
                ).await;
                
                // Convert resolved tags from Vec<serde_json::Value> to Vec<String>
                // IPRange client expects Vec<String> (tag IDs as strings)
                let resolved_tags: Option<Vec<String>> = resolved_tags_json.map(|tags| {
                    tags.into_iter()
                        .filter_map(|tag_value| {
                            // Extract numeric ID from serde_json::Value
                            if let Some(id) = tag_value.as_u64() {
                                Some(id.to_string())
                            } else {
                                // If it's an object (slug format), we can't use it with IPRange's current API
                                warn!("Tag resolved to non-numeric format, skipping (IPRange requires tag IDs as strings)");
                                None
                            }
                        })
                        .collect()
                });
                
                // Check if any field changed (including tags)
                let role_id_value = role_id.map(|id| id.0);
                if Self::ip_range_needs_update(
                    &ip_range_crd.spec,
                    &existing_range,
                    tenant_id,
                    status_str,
                    role_id_value,
                    vrf_id,
                ) {
                    // Update the IP range
                    let status_enum = match ip_range_crd.spec.status {
                        crds::IPRangeStatus::Active => Some(IPRangeStatus::Active),
                        crds::IPRangeStatus::Reserved => Some(IPRangeStatus::Reserved),
                        crds::IPRangeStatus::Deprecated => Some(IPRangeStatus::Deprecated),
                    };
                    
                    debug!("Updating IP range {} with tenant_id: {}, tags: {:?}", existing_range.id, tenant_id, resolved_tags);
                    
                    match netbox_client.update_ip_range(
                        IPRangeId(existing_range.id),
                        Some(&start_ip_net),
                        Some(&end_ip_net),
                        vrf_id, // VRF support (currently None, will be resolved when NetBoxVRF CRD exists)
                        Some(netbox_client::TenantId(tenant_id)),
                        role_id,
                        status_enum,
                        ip_range_crd.spec.description.clone(),
                        ip_range_crd.spec.comments.clone(),
                        Some(ip_range_crd.spec.mark_utilized),
                        Some(ip_range_crd.spec.mark_populated),
                        resolved_tags,
                    ).await {
                        Ok(updated_range) => {
                            // Update successful
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::UPDATED,
                                &format!("Updated IP range {} - {} in NetBox (ID: {})", updated_range.start_address, updated_range.end_address, updated_range.id),
                                ip_range_crd,
                            ).await;
                            Some(updated_range)
                        }
                        Err(e) => {
                            error!("Failed to update NetBoxIPRange {}/{} in NetBox: {}", namespace, name, e);
                            use crate::events::reasons;
                            self.record_event_warning(
                                reasons::RECONCILIATION_FAILED,
                                &format!("Failed to update NetBoxIPRange {}/{} in NetBox: {}", namespace, name, e),
                                ip_range_crd,
                            ).await;
                            update_status_error(&*self.netbox_ip_range_api, name, namespace, format!("{}", e), ip_range_crd.status.as_ref()).await;
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                } else {
                    // No changes needed
                    Some(existing_range)
                }
            }
            DriftCheckResult::StatusCleared { message } => {
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxIPRange {}/{} drift detected: {}", namespace, name, message),
                    ip_range_crd,
                ).await;
                
                let status_patch = Self::create_resource_status_patch(
                    0,
                    String::new(),
                    ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_ip_range_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxIPRange status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        // If we have a NetBox IP range, update status and return
        if let Some(range) = netbox_ip_range {
            let status_patch = Self::create_resource_status_patch(
                range.id,
                range.url.clone(),
                ResourceState::Created,
                None,
            );
            update_resource_status(
                &*self.netbox_ip_range_api,
                name,
                namespace,
                &status_patch,
                "NetBoxIPRange",
                range.id,
            ).await?;
            return Ok(());
        }
        
        // Need to create the IP range
        // Resolve dependencies
        validate_reference_kind(&ip_range_crd.spec.tenant, "NetBoxTenant", "tenant", name)?;
        let tenant_id = resolve_required_dependency_id(
            &*self.netbox_tenant_api,
            &ip_range_crd.spec.tenant.name,
            "NetBoxTenant",
            name,
            |crd| crd.status.as_ref(),
        ).await?;
        
        let role_id: Option<netbox_client::RoleId> = if let Some(role_ref) = &ip_range_crd.spec.role {
            validate_reference_kind(role_ref, "NetBoxRole", "role", name)?;
            resolve_optional_dependency_id(
                &*self.netbox_role_api,
                Some(role_ref),
                "NetBoxRole",
                "role",
                name,
                |crd| crd.status.as_ref(),
            ).await.map(|id| netbox_client::RoleId(id))
        } else {
            None
        };
        
        // Resolve VRF if specified
        let vrf_id: Option<u64> = if let Some(vrf_ref) = &ip_range_crd.spec.vrf {
            validate_reference_kind(vrf_ref, "NetBoxVRF", "vrf", name)?;
            Some(resolve_optional_dependency_id(
                &*self.netbox_vrf_api,
                Some(vrf_ref),
                "NetBoxVRF",
                "vrf",
                name,
                |crd| crd.status.as_ref(),
            ).await.ok_or_else(|| ControllerError::InvalidConfig(format!("VRF '{}' not found", vrf_ref.name)))?)
        } else {
            None
        };
        
        // Convert status enum
        let status_enum = match ip_range_crd.spec.status {
            crds::IPRangeStatus::Active => Some(IPRangeStatus::Active),
            crds::IPRangeStatus::Reserved => Some(IPRangeStatus::Reserved),
            crds::IPRangeStatus::Deprecated => Some(IPRangeStatus::Deprecated),
        };
        
        // Check for existing IP range before creating (idempotency)
        // Use fetch_all=true to ensure we check ALL ranges for conflicts
        let filters: Vec<(&str, &str)> = vec![
            ("start_address", &ip_range_crd.spec.start_address),
            ("end_address", &ip_range_crd.spec.end_address),
        ];
        let (existing_range_opt, was_pre_existing) = match netbox_client.query_ip_ranges(&filters, true).await {
            Ok(ranges) => {
                if let Some(existing) = ranges.into_iter()
                    .find(|r| r.start_address == start_ip_net && r.end_address == end_ip_net) {
                    // Range already exists - use it
                    info!("IP range {} - {} already exists in NetBox (ID: {}), using it", 
                        existing.start_address, existing.end_address, existing.id);
                    (Some(existing), true)
                } else {
                    (None, false)
                }
            }
            Err(_) => (None, false), // If query fails, proceed with creation
        };
        
        // Resolve tags for creation
        let resolved_tags_json = self.resolve_tag_references(
            netbox_client.as_ref(),
            &ip_range_crd.spec.tags,
            namespace,
            name,
        ).await;
        
        // Convert resolved tags from Vec<serde_json::Value> to Vec<String>
        // IPRange client expects Vec<String> (tag IDs as strings)
        let resolved_tags: Option<Vec<String>> = resolved_tags_json.map(|tags| {
            tags.into_iter()
                .filter_map(|tag_value| {
                    // Extract numeric ID from serde_json::Value
                    if let Some(id) = tag_value.as_u64() {
                        Some(id.to_string())
                    } else {
                        // If it's an object (slug format), we can't use it with IPRange's current API
                        warn!("Tag resolved to non-numeric format, skipping (IPRange requires tag IDs as strings)");
                        None
                    }
                })
                .collect()
        });
        
        debug!("Creating IP range {} - {} with tenant_id: {}, tags: {:?}", start_ip_net, end_ip_net, tenant_id, resolved_tags);
        
        let netbox_ip_range = if let Some(existing) = existing_range_opt {
            // Range was found in pre-check
            existing
        } else {
            // Create the IP range
            match netbox_client.create_ip_range(
                &start_ip_net,
                &end_ip_net,
                vrf_id, // VRF support (currently None, will be resolved when NetBoxVRF CRD exists)
                Some(netbox_client::TenantId(tenant_id)),
                role_id,
                status_enum,
                ip_range_crd.spec.description.clone(),
                ip_range_crd.spec.comments.clone(),
                Some(ip_range_crd.spec.mark_utilized),
                Some(ip_range_crd.spec.mark_populated),
                resolved_tags,
            ).await {
                Ok(created_range) => {
                    // Creation successful
                    created_range
                }
                Err(e) if is_conflict_error(&e) => {
                    // Conflict detected - try to find existing range
                    warn!("NetBoxIPRange {}/{} creation conflict - querying for existing", namespace, name);
                    // Use fetch_all=true to ensure we check ALL ranges for conflicts
                    match netbox_client.query_ip_ranges(&filters, true).await {
                        Ok(existing_ranges) => {
                            if let Some(existing) = existing_ranges.into_iter()
                                .find(|r| r.start_address == start_ip_net && r.end_address == end_ip_net) {
                                info!("Found existing NetBoxIPRange {}/{} in NetBox (ID: {}) after conflict", namespace, name, existing.id);
                                existing
                            } else {
                                let error_msg = format!("Conflict on create but no matching IP range found: {}", e);
                                error!("{}", error_msg);
                                update_status_error(&*self.netbox_ip_range_api, name, namespace, error_msg.clone(), ip_range_crd.status.as_ref()).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                        Err(query_err) => {
                            let error_msg = format!("Failed to query existing IP ranges after conflict: {}", query_err);
                            error!("{}", error_msg);
                            update_status_error(&*self.netbox_ip_range_api, name, namespace, error_msg.clone(), ip_range_crd.status.as_ref()).await;
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to create NetBoxIPRange {}/{} in NetBox: {}", namespace, name, e);
                    error!("{}", error_msg);
                    use crate::events::reasons;
                    self.record_event_warning(
                        reasons::RECONCILIATION_FAILED,
                        &error_msg,
                        ip_range_crd,
                    ).await;
                    update_status_error(&*self.netbox_ip_range_api, name, namespace, error_msg.clone(), ip_range_crd.status.as_ref()).await;
                    return Err(ControllerError::NetBox(e));
                }
            }
        };
        
        // Update status with the IP range (either existing or newly created)
        use crate::events::reasons;
        if !was_pre_existing {
            // Range was newly created - emit CREATED event
            self.record_event_normal(
                reasons::CREATED,
                &format!("Created IP range {} - {} in NetBox (ID: {})", 
                    netbox_ip_range.start_address, netbox_ip_range.end_address, netbox_ip_range.id),
                ip_range_crd,
            ).await;
        }
        
        let status_patch = Self::create_resource_status_patch(
            netbox_ip_range.id,
            netbox_ip_range.url.clone(),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_ip_range_api,
            name,
            namespace,
            &status_patch,
            "NetBoxIPRange",
            netbox_ip_range.id,
        ).await?;
        
        Ok(())
    }
}

