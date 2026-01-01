//! NetBoxIPAddress reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers::{extract_name_and_namespace, validate_status_and_drift, DriftCheckResult, update_resource_status, resolve_required_dependency_id, resolve_optional_dependency_id, validate_reference_kind, is_conflict_error};
use crate::kube_api_trait::KubeApiTrait;
use tracing::{info, error, debug, warn};
use crds::{NetBoxIPAddress, ResourceState};
use netbox_client::{NetBoxClientTrait, IpAddressId};
use std::str::FromStr;
use ipnet::IpNet;

impl Reconciler {
    /// Resolve tag references to NetBox tag IDs or dictionaries
    /// 
    /// For each NetBoxResourceReference in the tags list:
    /// 1. Get the NetBoxTag CRD
    /// 2. Extract the NetBox tag ID from the CRD status
    /// 3. If ID not found, query NetBox by name/slug
    /// 4. Return a list of tag IDs or dictionaries for the NetBox API
    pub async fn resolve_tag_references(
        &self,
        netbox_client: &dyn NetBoxClientTrait,
        tag_refs: &Option<Vec<crds::NetBoxResourceReference>>,
        namespace: &str,
        resource_name: &str,
    ) -> Option<Vec<serde_json::Value>> {
        let tag_refs = match tag_refs {
            Some(refs) if !refs.is_empty() => refs,
            _ => return None,
        };
        
        let mut resolved_tags = Vec::new();
        
        for tag_ref in tag_refs {
            validate_reference_kind(tag_ref, "NetBoxTag", "tag", resource_name).ok()?;
            
            let tag_namespace = tag_ref.namespace.as_deref().unwrap_or(namespace);
            
            // Try to get the NetBoxTag CRD and extract the NetBox ID
            let tag_id = match self.netbox_tag_api.get(&tag_ref.name).await {
                Ok(tag_crd) => {
                    if let Some(status) = &tag_crd.status {
                        if let Some(id) = status.netbox_id {
                            debug!("Resolved tag {}/{} to NetBox ID {} from CRD status", tag_namespace, tag_ref.name, id);
                            Some(id)
                        } else {
                            warn!("NetBoxTag CRD {}/{} exists but has no netbox_id in status, querying NetBox", tag_namespace, tag_ref.name);
                            None
                        }
                    } else {
                        warn!("NetBoxTag CRD {}/{} exists but has no status, querying NetBox", tag_namespace, tag_ref.name);
                        None
                    }
                }
                Err(_) => {
                    debug!("NetBoxTag CRD {}/{} not found, querying NetBox directly", tag_namespace, tag_ref.name);
                    None
                }
            };
            
            // If we got an ID from the CRD, use it
            if let Some(id) = tag_id {
                resolved_tags.push(serde_json::json!(id));
                continue;
            }
            
            // Fallback: Query NetBox directly by name
            match netbox_client.query_tags(&[("name", &tag_ref.name)], false).await {
                Ok(tags) => {
                    if let Some(tag) = tags.first() {
                        resolved_tags.push(serde_json::json!(tag.id));
                        debug!("Resolved tag '{}' to NetBox ID {} via query", tag_ref.name, tag.id);
                    } else {
                        // Tag doesn't exist in NetBox - skip it
                        // NetBox requires tags to exist before they can be referenced
                        // TODO: Consider creating missing tags automatically in the future
                        warn!("Tag '{}' not found in NetBox, skipping (tags must exist in NetBox before they can be assigned)", tag_ref.name);
                    }
                }
                Err(e) => {
                    // Query failed - skip this tag
                    warn!("Failed to query tag '{}' from NetBox: {}, skipping (tags must exist in NetBox before they can be assigned)", tag_ref.name, e);
                }
            }
        }
        
        if resolved_tags.is_empty() {
            None
        } else {
            Some(resolved_tags)
        }
    }
    
    /// Detect and remediate duplicate IP addresses in NetBox
    /// 
    /// This function:
    /// 1. Queries for all IP addresses with the same address
    /// 2. If multiple found, selects the best match (prefers one matching CRD's netbox_id if status exists)
    /// 3. Deletes all duplicates except the selected one
    /// 4. Returns the selected IP address
    /// 
    /// This handles both:
    /// - Duplicates created by human error
    /// - Duplicates created by reconciler bugs
    async fn detect_and_remediate_duplicate_ips(
        &self,
        netbox_client: &dyn NetBoxClientTrait,
        ip_address_crd: &NetBoxIPAddress,
        address: &str,
    ) -> Result<netbox_client::IPAddress, ControllerError> {
        // Query for all IPs with this address (fetch_all to get all pages)
        let mut all_ips = match netbox_client.query_ip_addresses(
            &[("address", address)],
            true, // fetch_all to check all pages
        ).await {
            Ok(ips) => {
                // Filter to exact matches (NetBox API filter might be fuzzy)
                ips.into_iter()
                    .filter(|ip| ip.address.to_string() == address)
                    .collect::<Vec<_>>()
            },
            Err(e) => {
                warn!("Failed to query for duplicate IP addresses {}: {}", address, e);
                return Err(ControllerError::NetBox(e));
            }
        };

        if all_ips.is_empty() {
            // No IPs found - this is fine, will create new one
            return Err(ControllerError::NetBox(netbox_client::NetBoxError::NotFound(
                format!("No IP address found for {}", address)
            )));
        }

        if all_ips.len() == 1 {
            // Only one IP found - no duplicates, return it
            return Ok(all_ips.into_iter().next().unwrap());
        }

        // Multiple duplicates found - need to remediate
        warn!("Found {} duplicate IP addresses for {} in NetBox, remediating", all_ips.len(), address);
        
        // Select the best match using timestamps for proper deduplication:
        // 1. Prefer one that matches CRD's netbox_id (if status exists) - this is the one we're managing
        // 2. Otherwise, prefer the oldest one by created timestamp (first created is the original)
        // 3. If created timestamps are equal, use last_updated (older is better)
        // 4. Fallback to lowest ID if timestamps are unavailable
        let (selected_ip, duplicates) = if let Some(status) = &ip_address_crd.status {
            if let Some(expected_id) = status.netbox_id {
                // Try to find one matching the expected ID (this is the one we're managing)
                if let Some(pos) = all_ips.iter().position(|ip| ip.id == expected_id) {
                    let selected = all_ips.remove(pos);
                    (selected, all_ips)
                } else {
                    // Expected ID not found, pick oldest by created timestamp
                    let mut sorted = all_ips;
                    sorted.sort_by(|a, b| {
                        // Parse created timestamps (RFC3339 format)
                        let a_created = chrono::DateTime::parse_from_rfc3339(&a.created)
                            .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                        let b_created = chrono::DateTime::parse_from_rfc3339(&b.created)
                            .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                        
                        // Primary sort: by created timestamp (oldest first)
                        match a_created.cmp(&b_created) {
                            std::cmp::Ordering::Equal => {
                                // Secondary sort: by last_updated (older is better)
                                let a_updated = chrono::DateTime::parse_from_rfc3339(&a.last_updated)
                                    .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                                let b_updated = chrono::DateTime::parse_from_rfc3339(&b.last_updated)
                                    .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                                a_updated.cmp(&b_updated)
                            }
                            other => other,
                        }
                    });
                    let selected = sorted.remove(0);
                    (selected, sorted)
                }
            } else {
                // No expected ID, pick oldest by created timestamp
                let mut sorted = all_ips;
                sorted.sort_by(|a, b| {
                    let a_created = chrono::DateTime::parse_from_rfc3339(&a.created)
                        .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                    let b_created = chrono::DateTime::parse_from_rfc3339(&b.created)
                        .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                    
                    match a_created.cmp(&b_created) {
                        std::cmp::Ordering::Equal => {
                            let a_updated = chrono::DateTime::parse_from_rfc3339(&a.last_updated)
                                .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                            let b_updated = chrono::DateTime::parse_from_rfc3339(&b.last_updated)
                                .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                            a_updated.cmp(&b_updated)
                        }
                        other => other,
                    }
                });
                let selected = sorted.remove(0);
                (selected, sorted)
            }
        } else {
            // No status, pick oldest by created timestamp
            let mut sorted = all_ips;
            sorted.sort_by(|a, b| {
                let a_created = chrono::DateTime::parse_from_rfc3339(&a.created)
                    .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                let b_created = chrono::DateTime::parse_from_rfc3339(&b.created)
                    .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                
                match a_created.cmp(&b_created) {
                    std::cmp::Ordering::Equal => {
                        let a_updated = chrono::DateTime::parse_from_rfc3339(&a.last_updated)
                            .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                        let b_updated = chrono::DateTime::parse_from_rfc3339(&b.last_updated)
                            .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                        a_updated.cmp(&b_updated)
                    }
                    other => other,
                }
            });
            let selected = sorted.remove(0);
            (selected, sorted)
        };

        info!("Selected IP address {} (ID: {}, created: {}) as the canonical one (oldest), will delete {} duplicates", 
            selected_ip.address, selected_ip.id, selected_ip.created, duplicates.len());

        // Delete all duplicates (sorted by creation time for logging)
        let mut deleted_count = 0;
        let mut failed_deletes = Vec::new();
        for duplicate in duplicates {
            match netbox_client.delete_ip_address(IpAddressId(duplicate.id)).await {
                Ok(_) => {
                    deleted_count += 1;
                    info!("Deleted duplicate IP address {} (ID: {}, created: {})", 
                        duplicate.address, duplicate.id, duplicate.created);
                }
                Err(e) => {
                    warn!("Failed to delete duplicate IP address {} (ID: {}, created: {}): {}", 
                        duplicate.address, duplicate.id, duplicate.created, e);
                    failed_deletes.push((duplicate.id, e));
                }
            }
        }

        // Emit event about remediation
        use crate::events::reasons;
        let total_count = deleted_count + 1; // +1 for the selected IP
        if deleted_count > 0 {
            self.record_event_warning(
                reasons::DRIFT_DETECTED,
                &format!("Remediated {} duplicate IP addresses for {}: deleted {} duplicates (newest), kept ID {} (oldest, created: {})", 
                    total_count, address, deleted_count, selected_ip.id, selected_ip.created),
                ip_address_crd,
            ).await;
        }

        if !failed_deletes.is_empty() {
            warn!("Failed to delete {} duplicate IP addresses, but continuing with selected IP", failed_deletes.len());
        }

        Ok(selected_ip)
    }

    /// Check if IP address needs updating by comparing spec with existing NetBox resource
    fn ip_address_needs_update(
        spec: &crds::NetBoxIPAddressSpec,
        existing: &netbox_client::IPAddress,
        desired_tenant_id: u64,
        _desired_vlan_id: Option<u32>,
        desired_status: &str,
    ) -> bool {
        // Compare tenant
        let existing_tenant_id = existing.tenant.as_ref().map(|t| t.id);
        if Some(desired_tenant_id) != existing_tenant_id {
            debug!("IP address tenant changed: {:?} -> {}", existing_tenant_id, desired_tenant_id);
            return true;
        }
        
        // Compare vlan
        // Note: IPAddress model doesn't have a vlan field in the response,
        // but vlan can be set via API. We can't compare vlan from existing resource,
        // so we'll update if other fields changed. Vlan updates will be handled by
        // always including vlan_id in update calls when provided.
        
        // Compare status
        let existing_status = match existing.status {
            netbox_client::IPAddressStatus::Active => "active",
            netbox_client::IPAddressStatus::Reserved => "reserved",
            netbox_client::IPAddressStatus::Deprecated => "deprecated",
            netbox_client::IPAddressStatus::Dhcp => "dhcp",
            netbox_client::IPAddressStatus::Slaac => "slaac",
        };
        if desired_status != existing_status {
            debug!("IP address status changed: '{}' -> '{}'", existing_status, desired_status);
            return true;
        }
        
        // Compare role
        let existing_role = existing.role.as_deref();
        let desired_role = spec.role.as_deref();
        if desired_role != existing_role {
            debug!("IP address role changed: {:?} -> {:?}", existing_role, desired_role);
            return true;
        }
        
        // Compare dns_name
        let existing_dns_name = existing.dns_name.as_str();
        let desired_dns_name = spec.dns_name.as_deref().unwrap_or("");
        if desired_dns_name != existing_dns_name {
            debug!("IP address dns_name changed: '{}' -> '{}'", existing_dns_name, desired_dns_name);
            return true;
        }
        
        // Compare description
        let spec_desc = spec.description.as_deref().unwrap_or("");
        if spec_desc != existing.description {
            debug!("IP address description changed: '{}' -> '{}'", existing.description, spec_desc);
            return true;
        }
        
        // Compare tags using helper function
        if crate::reconcile_helpers::tags_differ(&existing.tags, &spec.tags) {
            return true;
        }
        
        false // No changes needed
    }

    pub async fn reconcile_netbox_ip_address(&self, ip_address_crd: &NetBoxIPAddress) -> Result<(), ControllerError> {
        let (name, namespace) = extract_name_and_namespace(ip_address_crd, "NetBoxIPAddress")?;
        let tenant_ref = &ip_address_crd.spec.tenant;
        
        info!("Reconciling NetBoxIPAddress {}/{}", namespace, name);
        
        // Get tenant-specific client
        let netbox_client = self.token_resolver
            .create_client_for_tenant(namespace, tenant_ref)
            .await?;
        
        // Resolve interface ID for static DHCP reservations (early, so available in all code paths)
        // Priority: 1. spec.interface (explicit reference), 2. spec.macAddress (query by MAC)
        let interface_id: Option<u64> = if let Some(interface_ref) = &ip_address_crd.spec.interface {
            // Explicit interface reference takes precedence
            validate_reference_kind(interface_ref, "NetBoxInterface", "interface", name)?;
            let id = resolve_optional_dependency_id(
                &*self.netbox_interface_api,
                Some(interface_ref),
                "NetBoxInterface",
                "interface",
                name,
                |crd| crd.status.as_ref(),
            ).await;
            if let Some(id) = id {
                info!("Resolved interface reference '{}' to NetBox ID {}", interface_ref.name, id);
                Some(id)
            } else {
                warn!("Interface reference '{}' not found, will skip interface assignment", interface_ref.name);
                None
            }
        } else if let Some(mac_address) = &ip_address_crd.spec.mac_address {
            // Query interfaces by MAC address
            info!("Resolving interface by MAC address: {}", mac_address);
            match netbox_client.query_interfaces(&[("mac_address", mac_address.as_str())], false).await {
                Ok(interfaces) => {
                    if let Some(interface) = interfaces.first() {
                        info!("Found interface {} (ID: {}) with MAC address {}", interface.name, interface.id, mac_address);
                        Some(interface.id)
                    } else {
                        warn!("No interface found with MAC address {}, will skip interface assignment", mac_address);
                        None
                    }
                }
                Err(e) => {
                    warn!("Failed to query interfaces by MAC address {}: {}, will skip interface assignment", mac_address, e);
                    None
                }
            }
        } else {
            None
        };
        
        // Helper function to update status with error
        async fn update_status_error(
            api: &dyn KubeApiTrait<NetBoxIPAddress>,
            name: &str,
            namespace: &str,
            error_msg: String,
            current_status: Option<&crds::NetBoxIPAddressStatus>,
        ) {
            if let Some(status) = current_status {
                if status.state == ResourceState::Failed && status.error.as_ref() == Some(&error_msg) {
                    debug!("NetBoxIPAddress {}/{} already has this error in status, skipping update", namespace, name);
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
                error!("Failed to update NetBoxIPAddress {}/{} error status: {}", namespace, name, e);
            } else {
                info!("Updated NetBoxIPAddress {}/{} status with error", namespace, name);
            }
        }
        
        // Validate that at least one of address or ip_range is provided
        if ip_address_crd.spec.address.is_none() && ip_address_crd.spec.ip_range.is_none() {
            let error_msg = "Either 'address' or 'ipRange' (or both) must be specified for NetBoxIPAddress".to_string();
            error!("NetBoxIPAddress {}/{}: {}", namespace, name, error_msg);
            update_status_error(&*self.netbox_ip_address_api, name, namespace, error_msg.clone(), ip_address_crd.status.as_ref()).await;
            return Err(ControllerError::InvalidInput(error_msg));
        }
        
        // Validate DHCP scenario requirements
        if ip_address_crd.spec.status == crds::IPAddressStatus::Dhcp {
            if ip_address_crd.spec.address.is_some() {
                // Static reservation: require macAddress or interface
                if ip_address_crd.spec.mac_address.is_none() && ip_address_crd.spec.interface.is_none() {
                    let error_msg = "For static DHCP reservations (status: dhcp with address specified), either 'macAddress' or 'interface' must be provided".to_string();
                    error!("NetBoxIPAddress {}/{}: {}", namespace, name, error_msg);
                    update_status_error(&*self.netbox_ip_address_api, name, namespace, error_msg.clone(), ip_address_crd.status.as_ref()).await;
                    return Err(ControllerError::InvalidInput(error_msg));
                }
                
                // Validate MAC address format if provided
                if let Some(mac) = &ip_address_crd.spec.mac_address {
                    use crate::reconcile_helpers::is_valid_mac_address;
                    if !is_valid_mac_address(mac) {
                        let error_msg = format!("Invalid MAC address format '{}'. Expected format: 'aa:bb:cc:dd:ee:ff' or 'aa-bb-cc-dd-ee-ff'", mac);
                        error!("NetBoxIPAddress {}/{}: {}", namespace, name, error_msg);
                        update_status_error(&*self.netbox_ip_address_api, name, namespace, error_msg.clone(), ip_address_crd.status.as_ref()).await;
                        return Err(ControllerError::InvalidInput(error_msg));
                    }
                }
            } else {
                // Random allocation: require ipRange, no address
                if ip_address_crd.spec.ip_range.is_none() {
                    let error_msg = "For random DHCP allocation (status: dhcp without address), 'ipRange' must be specified".to_string();
                    error!("NetBoxIPAddress {}/{}: {}", namespace, name, error_msg);
                    update_status_error(&*self.netbox_ip_address_api, name, namespace, error_msg.clone(), ip_address_crd.status.as_ref()).await;
                    return Err(ControllerError::InvalidInput(error_msg));
                }
            }
        }
        
        // Resolve IP range reference if provided
        let ip_range_id: Option<u64> = if let Some(ip_range_ref) = &ip_address_crd.spec.ip_range {
            validate_reference_kind(ip_range_ref, "NetBoxIPRange", "ipRange", name)?;
            match resolve_optional_dependency_id(
                &*self.netbox_ip_range_api,
                Some(ip_range_ref),
                "NetBoxIPRange",
                "ipRange",
                name,
                |crd| crd.status.as_ref(),
            ).await {
                Some(id) => {
                    info!("Resolved IP range reference '{}' to NetBox ID {}", ip_range_ref.name, id);
                    Some(id)
                }
                None => {
                    let error_msg = format!("IP range '{}' not found or not ready", ip_range_ref.name);
                    use crate::events::reasons;
                    self.record_event_warning(
                        reasons::DEPENDENCY_NOT_FOUND,
                        &error_msg,
                        ip_address_crd,
                    ).await;
                    update_status_error(&*self.netbox_ip_address_api, name, namespace, error_msg.clone(), ip_address_crd.status.as_ref()).await;
                    return Err(ControllerError::InvalidInput(error_msg));
                }
            }
        } else {
            None
        };
        
        // Parse IP address from spec or status (for DHCP IPs, address may be in status)
        let ip_net = if let Some(address) = &ip_address_crd.spec.address {
            // Use address from spec (static IPs)
            IpNet::from_str(address)
                .map_err(|e| ControllerError::InvalidInput(format!("Invalid IP address format '{}': {}", address, e)))?
        } else if let Some(status) = &ip_address_crd.status {
            // For DHCP IPs, try to get address from status (set after previous reconciliation)
            if let Some(status_address) = &status.address {
                IpNet::from_str(status_address)
                    .map_err(|e| ControllerError::InvalidInput(format!("Invalid IP address format in status '{}': {}", status_address, e)))?
            } else {
                // No address in spec or status - for DHCP with ipRange, we need the address
                let error_msg = "IP address must be specified in either spec.address or status.address. For DHCP IPs, the address will be stored in status.address after reconciliation.".to_string();
                error!("NetBoxIPAddress {}/{}: {}", namespace, name, error_msg);
                update_status_error(&*self.netbox_ip_address_api, name, namespace, error_msg.clone(), ip_address_crd.status.as_ref()).await;
                return Err(ControllerError::InvalidInput(error_msg));
            }
        } else {
            // No address in spec or status
            let error_msg = "IP address must be specified in either spec.address or status.address. For DHCP IPs, the address will be stored in status.address after reconciliation.".to_string();
            error!("NetBoxIPAddress {}/{}: {}", namespace, name, error_msg);
            update_status_error(&*self.netbox_ip_address_api, name, namespace, error_msg.clone(), ip_address_crd.status.as_ref()).await;
            return Err(ControllerError::InvalidInput(error_msg));
        };
        
        // If ip_range is provided, validate address (from spec or status) is within range
        if let Some(range_id) = ip_range_id {
            // Get the IP range to validate address is within it
            match netbox_client.get_ip_range(netbox_client::IPRangeId(range_id)).await {
                Ok(range) => {
                    let address_ip = ip_net.addr();
                    let range_start = range.start_address.addr();
                    let range_end = range.end_address.addr();
                    
                    // Get address string for error messages and logging
                    let address_str = ip_address_crd.spec.address.as_ref()
                        .or_else(|| ip_address_crd.status.as_ref().and_then(|s| s.address.as_ref()))
                        .map(|s| s.as_str())
                        .unwrap_or("unknown");
                    
                    // Check if address is within range
                    if address_ip < range_start || address_ip > range_end {
                        let error_msg = format!(
                            "IP address {} is not within the specified IP range {} - {}",
                            address_str, range.start_address, range.end_address
                        );
                        warn!("NetBoxIPAddress {}/{}: {}", namespace, name, error_msg);
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::RECONCILIATION_FAILED,
                            &error_msg,
                            ip_address_crd,
                        ).await;
                        update_status_error(&*self.netbox_ip_address_api, name, namespace, error_msg.clone(), ip_address_crd.status.as_ref()).await;
                        return Err(ControllerError::InvalidInput(error_msg));
                    }
                    debug!("Validated IP address {} is within range {} - {}", address_str, range.start_address, range.end_address);
                }
                Err(e) => {
                    warn!("Failed to validate IP address against range (ID: {}): {}", range_id, e);
                    // Continue anyway - range validation is best-effort
                }
            }
        }
        
        // Validate status and check for drift
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                ip_address_crd.status.as_ref(),
                "NetBoxIPAddress",
                namespace,
                name,
                |netbox_id: u64| async move {
                    netbox_client_ref.get_ip_address(IpAddressId(netbox_id)).await
                },
            ).await?
        };
        
        let netbox_ip_address: Option<netbox_client::IPAddress> = match drift_result {
            DriftCheckResult::UseExisting(existing_ip) => {
                // Resource exists - first, check for and remediate any duplicates
                // This ensures we always clean up duplicates, even for existing resources
                let address_str = ip_address_crd.spec.address.as_ref()
                    .ok_or_else(|| ControllerError::InvalidInput("Address is required for IP address reconciliation".to_string()))?;
                
                let remediated_ip = match self.detect_and_remediate_duplicate_ips(
                    netbox_client.as_ref(),
                    ip_address_crd,
                    address_str,
                ).await {
                    Ok(ip) => {
                        // If the remediated IP is different from the existing one, log it
                        if ip.id != existing_ip.id {
                            warn!("NetBoxIPAddress {}/{}: Duplicate remediation selected different IP (ID: {} -> {}, created: {})", 
                                namespace, name, existing_ip.id, ip.id, ip.created);
                            // Update the status to reflect the new ID
                            let status_patch = Self::create_typed_ip_address_status_patch(
                                ip.id,
                                ip.url.clone(),
                                Some(ip.address.to_string()),
                                ResourceState::Created,
                                Some(format!("Remediated duplicates, using IP ID {} (created: {})", ip.id, ip.created)),
                            );
                            update_resource_status(
                                &*self.netbox_ip_address_api,
                                name,
                                namespace,
                                &status_patch,
                                "NetBoxIPAddress",
                                ip.id,
                            ).await?;
                            // Return early since we've updated status
                            return Ok(());
                        }
                        ip
                    }
                    Err(ControllerError::NetBox(netbox_client::NetBoxError::NotFound(_))) => {
                        // No duplicates found, use existing
                        existing_ip
                    }
                    Err(e) => {
                        // Error during duplicate detection - log but continue with existing IP
                        warn!("Failed to check for duplicates for NetBoxIPAddress {}/{}: {}, continuing with existing IP", namespace, name, e);
                        existing_ip
                    }
                };
                
                // Resource exists - check if it needs updating
                // Resolve dependencies for comparison
                validate_reference_kind(&ip_address_crd.spec.tenant, "NetBoxTenant", "tenant", name)?;
                let tenant_id = resolve_required_dependency_id(
                    &*self.netbox_tenant_api,
                    &ip_address_crd.spec.tenant.name,
                    "NetBoxTenant",
                    name,
                    |crd| crd.status.as_ref(),
                ).await?;
                
                let vlan_id: Option<u32> = if let Some(vlan_ref) = &ip_address_crd.spec.vlan {
                    validate_reference_kind(vlan_ref, "NetBoxVLAN", "vlan", name)?;
                    resolve_optional_dependency_id(
                        &*self.netbox_vlan_api,
                        Some(vlan_ref),
                        "NetBoxVLAN",
                        "vlan",
                        name,
                        |crd| crd.status.as_ref(),
                    ).await.map(|id| id as u32)
                } else {
                    None
                };
                
                // Convert status enum to string
                let status_str = match ip_address_crd.spec.status {
                    crds::IPAddressStatus::Active => "active",
                    crds::IPAddressStatus::Reserved => "reserved",
                    crds::IPAddressStatus::Deprecated => "deprecated",
                    crds::IPAddressStatus::Dhcp => "dhcp",
                    crds::IPAddressStatus::Slaac => "slaac",
                };
                
                // Always resolve tags (even if nothing else changed, tags might need updating)
                let resolved_tags = self.resolve_tag_references(
                    netbox_client.as_ref(),
                    &ip_address_crd.spec.tags,
                    namespace,
                    name,
                ).await;
                
                // Check if any field changed (including tags)
                let needs_update = Self::ip_address_needs_update(
                    &ip_address_crd.spec,
                    &remediated_ip,
                    tenant_id,
                    vlan_id, // Note: vlan_id comparison not implemented in needs_update yet
                    status_str,
                );
                
                if needs_update {
                    info!("IP address {}/{} needs update (tenant: {:?} -> {}, existing tags: {}, desired tags: {:?})", 
                        namespace, name, 
                        remediated_ip.tenant.as_ref().map(|t| t.id), 
                        tenant_id,
                        remediated_ip.tags.len(),
                        resolved_tags.as_ref().map(|t| t.len()).unwrap_or(0));
                    
                    debug!("Updating IP address {} with tenant_id: {}, tags: {:?}", remediated_ip.id, tenant_id, resolved_tags);
                    
                    // Update the IP address
                    use netbox_client::AllocateIPRequest;
                    let update_request = AllocateIPRequest {
                        address: None, // Address cannot be changed
                        description: ip_address_crd.spec.description.clone(),
                        status: Some(match ip_address_crd.spec.status {
                            crds::IPAddressStatus::Active => netbox_client::IPAddressStatus::Active,
                            crds::IPAddressStatus::Reserved => netbox_client::IPAddressStatus::Reserved,
                            crds::IPAddressStatus::Deprecated => netbox_client::IPAddressStatus::Deprecated,
                            crds::IPAddressStatus::Dhcp => netbox_client::IPAddressStatus::Dhcp,
                            crds::IPAddressStatus::Slaac => netbox_client::IPAddressStatus::Slaac,
                        }),
                        role: ip_address_crd.spec.role.clone(),
                        dns_name: ip_address_crd.spec.dns_name.clone(),
                        tenant: Some(tenant_id),
                        tags: resolved_tags,
                        assigned_object_type: interface_id.map(|_| "dcim.interface".to_string()),
                        assigned_object_id: interface_id,
                    };
                    
                    match netbox_client.update_ip_address(IpAddressId(remediated_ip.id), update_request).await {
                        Ok(updated_ip) => {
                            // Update successful
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::UPDATED,
                                &format!("Updated IP address {} in NetBox (ID: {})", updated_ip.address, updated_ip.id),
                                ip_address_crd,
                            ).await;
                            // Update status with the updated IP
                            let status_patch = Self::create_typed_ip_address_status_patch(
                                updated_ip.id,
                                updated_ip.url.clone(),
                                Some(updated_ip.address.to_string()),
                                ResourceState::Created,
                                None,
                            );
                            update_resource_status(
                                &*self.netbox_ip_address_api,
                                name,
                                namespace,
                                &status_patch,
                                "NetBoxIPAddress",
                                updated_ip.id,
                            ).await?;
                            return Ok(());
                        }
                        Err(e) => {
                            error!("Failed to update NetBoxIPAddress {}/{} in NetBox: {}", namespace, name, e);
                            use crate::events::reasons;
                            self.record_event_warning(
                                reasons::RECONCILIATION_FAILED,
                                &format!("Failed to update NetBoxIPAddress {}/{} in NetBox: {}", namespace, name, e),
                                ip_address_crd,
                            ).await;
                            update_status_error(&*self.netbox_ip_address_api, name, namespace, format!("{}", e), ip_address_crd.status.as_ref()).await;
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                } else {
                    // No changes needed - only update status if it changed
                    use crate::reconcile_helpers::status_needs_update;
                    let needs_status_update = status_needs_update(
                        ip_address_crd.status.as_ref(),
                        remediated_ip.id,
                        &remediated_ip.url,
                        "Created",
                        None,
                    );
                    
                    if needs_status_update {
                        let status_patch = Self::create_typed_ip_address_status_patch(
                            remediated_ip.id,
                            remediated_ip.url.clone(),
                            Some(remediated_ip.address.to_string()),
                            ResourceState::Created,
                            None,
                        );
                        update_resource_status(
                            &*self.netbox_ip_address_api,
                            name,
                            namespace,
                            &status_patch,
                            "NetBoxIPAddress",
                            remediated_ip.id,
                        ).await?;
                        debug!("Updated NetBoxIPAddress {}/{} status: NetBox ID {}", namespace, name, remediated_ip.id);
                        return Ok(());
                    } else {
                        debug!("NetBoxIPAddress {}/{} already has correct status (ID: {}), skipping update", namespace, name, remediated_ip.id);
                        return Ok(());
                    }
                }
            }
            DriftCheckResult::StatusCleared { message } => {
                // Status was cleared (invalid netbox_id) - query by ADDRESS to find existing IP
                // This is critical: don't trust status, always query NetBox directly by address
                let address_str = ip_address_crd.spec.address.as_ref()
                    .ok_or_else(|| ControllerError::InvalidInput("Address is required for IP address reconciliation".to_string()))?;
                
                // Query by address to find existing IP (even though status says it doesn't exist)
                match self.detect_and_remediate_duplicate_ips(
                    netbox_client.as_ref(),
                    ip_address_crd,
                    address_str,
                ).await {
                    Ok(existing_ip) => {
                        // Found existing IP! Status was wrong - use it and update status
                        info!("NetBoxIPAddress {}/{}: Found existing IP {} (ID: {}) despite invalid status, updating status", 
                            namespace, name, address_str, existing_ip.id);
                        
                        // Emit event for drift detection
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DRIFT_DETECTED,
                            &format!("NetBoxIPAddress {}/{} drift detected: {} - Found existing IP (ID: {})", 
                                namespace, name, message, existing_ip.id),
                            ip_address_crd,
                        ).await;
                        
                        // Update status with correct netbox_id
                        let status_patch = Self::create_typed_ip_address_status_patch(
                            existing_ip.id,
                            existing_ip.url.clone(),
                            Some(existing_ip.address.to_string()),
                            ResourceState::Created,
                            Some(format!("Recovered from invalid status: {}", message)),
                        );
                        update_resource_status(
                            &*self.netbox_ip_address_api,
                            name,
                            namespace,
                            &status_patch,
                            "NetBoxIPAddress",
                            existing_ip.id,
                        ).await?;
                        // Return early - we've found and updated the existing IP
                        return Ok(());
                    }
                    Err(ControllerError::NetBox(netbox_client::NetBoxError::NotFound(_))) => {
                        // No IP found - status was correct, need to create
                        // Emit event for drift detection
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DRIFT_DETECTED,
                            &format!("NetBoxIPAddress {}/{} drift detected: {}", namespace, name, message),
                            ip_address_crd,
                        ).await;
                        
                        let status_patch = Self::create_typed_ip_address_status_patch(
                            0,
                            String::new(),
                            None, // No address yet
                            ResourceState::Pending,
                            Some(message),
                        );
                        let pp = kube::api::PatchParams::default();
                        if let Err(update_err) = self.netbox_ip_address_api
                            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                            .await
                        {
                            warn!("Failed to clear NetBoxIPAddress status: {}", update_err);
                        }
                        // Fall through to creation
                        None
                    }
                    Err(e) => {
                        // Error during query - log and fall through to creation
                        warn!("Failed to query for existing IP address {} during status clear: {}, will attempt creation", address_str, e);
                        // Emit event for drift detection
                        use crate::events::reasons;
                        self.record_event_warning(
                            reasons::DRIFT_DETECTED,
                            &format!("NetBoxIPAddress {}/{} drift detected: {}", namespace, name, message),
                            ip_address_crd,
                        ).await;
                        
                        let status_patch = Self::create_typed_ip_address_status_patch(
                            0,
                            String::new(),
                            None, // No address yet
                            ResourceState::Pending,
                            Some(message),
                        );
                        let pp = kube::api::PatchParams::default();
                        if let Err(update_err) = self.netbox_ip_address_api
                            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                            .await
                        {
                            warn!("Failed to clear NetBoxIPAddress status: {}", update_err);
                        }
                        // Fall through to creation
                        None
                    }
                }
            }
            DriftCheckResult::Recreate => {
                // Need to create - but first query by ADDRESS to ensure it doesn't exist
                let address_str = ip_address_crd.spec.address.as_ref()
                    .ok_or_else(|| ControllerError::InvalidInput("Address is required for IP address reconciliation".to_string()))?;
                
                // Query by address to find existing IP (critical: don't create if it exists)
                match self.detect_and_remediate_duplicate_ips(
                    netbox_client.as_ref(),
                    ip_address_crd,
                    address_str,
                ).await {
                    Ok(existing_ip) => {
                        // Found existing IP! Use it and update status
                        info!("NetBoxIPAddress {}/{}: Found existing IP {} (ID: {}) during recreate, updating status", 
                            namespace, name, address_str, existing_ip.id);
                        
                        let status_patch = Self::create_typed_ip_address_status_patch(
                            existing_ip.id,
                            existing_ip.url.clone(),
                            Some(existing_ip.address.to_string()),
                            ResourceState::Created,
                            Some("Found existing IP during recreate".to_string()),
                        );
                        update_resource_status(
                            &*self.netbox_ip_address_api,
                            name,
                            namespace,
                            &status_patch,
                            "NetBoxIPAddress",
                            existing_ip.id,
                        ).await?;
                        // Return early - we've found and updated the existing IP
                        return Ok(());
                    }
                    Err(ControllerError::NetBox(netbox_client::NetBoxError::NotFound(_))) => {
                        // No IP found - proceed with creation
                        None
                    }
                    Err(e) => {
                        // Error during query - log but proceed with creation
                        warn!("Failed to query for existing IP address {} during recreate: {}, will attempt creation", address_str, e);
                        None
                    }
                }
            }
        };
        
        // If we reach here, we need to create the IP address
        // (all existing IP cases should have returned early above)
        
        // Need to create IP address - resolve dependencies first
        validate_reference_kind(&ip_address_crd.spec.tenant, "NetBoxTenant", "tenant", name)?;
    let tenant_id = match resolve_required_dependency_id(
        &*self.netbox_tenant_api,
        &ip_address_crd.spec.tenant.name,
        "NetBoxTenant",
        name,
        |crd| crd.status.as_ref(),
    ).await {
        Ok(id) => id,
        Err(e) => {
            use crate::events::reasons;
            self.record_event_warning(
                reasons::DEPENDENCY_NOT_FOUND,
                &format!("Tenant '{}' not found or not ready: {}", ip_address_crd.spec.tenant.name, e),
                ip_address_crd,
            ).await;
            return Err(e);
        }
    };
    
    // Resolve optional vlan ID (for future use in AllocateIPRequest)
    let _vlan_id: Option<u32> = if let Some(vlan_ref) = &ip_address_crd.spec.vlan {
        validate_reference_kind(vlan_ref, "NetBoxVLAN", "vlan", name)?;
        resolve_optional_dependency_id(
            &*self.netbox_vlan_api,
            Some(vlan_ref),
            "NetBoxVLAN",
            "vlan",
            name,
            |crd| crd.status.as_ref(),
        ).await.map(|id| id as u32)
    } else {
        None
    };
    
    // Convert status enum to NetBox status
    let netbox_status = match ip_address_crd.spec.status {
        crds::IPAddressStatus::Active => netbox_client::IPAddressStatus::Active,
        crds::IPAddressStatus::Reserved => netbox_client::IPAddressStatus::Reserved,
        crds::IPAddressStatus::Deprecated => netbox_client::IPAddressStatus::Deprecated,
        crds::IPAddressStatus::Dhcp => netbox_client::IPAddressStatus::Dhcp,
        crds::IPAddressStatus::Slaac => netbox_client::IPAddressStatus::Slaac,
    };
    
    // CRITICAL: Before creating, query ALL IPs by address to ensure it doesn't exist
    // This prevents duplicate creation due to race conditions or query failures
    let address_str = ip_address_crd.spec.address.as_ref()
        .ok_or_else(|| ControllerError::InvalidInput("Address is required for IP address reconciliation".to_string()))?;
    
    // Query ALL IPs (no filters) and filter client-side for exact match
    // This is more reliable than using NetBox API filters which might be inaccurate
    info!("Pre-creation check: Querying ALL IP addresses to find exact match for {}", address_str);
    let all_ips = match netbox_client.query_ip_addresses(&[], true).await {
        Ok(ips) => {
            // Filter client-side for exact address match
            ips.into_iter()
                .filter(|ip| ip.address.to_string() == address_str.as_str())
                .collect::<Vec<_>>()
        }
        Err(e) => {
            warn!("Failed to query all IP addresses for pre-creation check: {}, will use filtered query", e);
            // Fallback to filtered query
            match netbox_client.query_ip_addresses(&[("address", address_str.as_str())], true).await {
                Ok(ips) => {
                    ips.into_iter()
                        .filter(|ip| ip.address.to_string() == address_str.as_str())
                        .collect::<Vec<_>>()
                }
                Err(e2) => {
                    error!("Failed to query IP addresses (both global and filtered): {}, proceeding with creation (may create duplicate)", e2);
                    Vec::new()
                }
            }
        }
    };
    
    let netbox_ip_address = if !all_ips.is_empty() {
        // Found existing IP(s) - use duplicate detection to select best one
        warn!("Pre-creation check found {} existing IP(s) for {}, using duplicate detection", all_ips.len(), address_str);
        match self.detect_and_remediate_duplicate_ips(
            netbox_client.as_ref(),
            ip_address_crd,
            address_str.as_str(),
        ).await {
            Ok(existing) => {
                info!("IP address {} already exists in NetBox (ID: {}), using it (pre-creation check prevented duplicate)", address_str, existing.id);
                existing
            }
            Err(e) => {
                // Should not happen since we found IPs, but handle gracefully
                error!("Pre-creation check found IPs but duplicate detection failed: {}, proceeding with creation (may create duplicate)", e);
                // Fall through to creation - this is a bug but we don't want to block reconciliation
                return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(
                    format!("Pre-creation check found existing IPs but duplicate detection failed: {}", e)
                )));
            }
        }
    } else {
        // No IP found - safe to create
        info!("Pre-creation check: No existing IP found for {}, creating new one", address_str);
        
        // Resolve tags from NetBoxResourceReference to NetBox tag IDs
        let resolved_tags = self.resolve_tag_references(
            netbox_client.as_ref(),
            &ip_address_crd.spec.tags,
            namespace,
            name,
        ).await;
        
        debug!("Creating IP address {} with tenant_id: {}, tags: {:?}", address_str, tenant_id, resolved_tags);
        
        // Create IP address
        use netbox_client::AllocateIPRequest;
        let create_request = AllocateIPRequest {
            address: Some(ip_net), // Specify the exact IP address
            description: ip_address_crd.spec.description.clone(),
            status: Some(netbox_status),
            role: ip_address_crd.spec.role.clone(),
            dns_name: ip_address_crd.spec.dns_name.clone(),
            tenant: Some(tenant_id),
            tags: resolved_tags,
            assigned_object_type: interface_id.map(|_| "dcim.interface".to_string()),
            assigned_object_id: interface_id,
        };
        
        match netbox_client.create_ip_address(&ip_net, Some(create_request)).await {
            Ok(created_ip) => {
                info!("Created IP address {} in NetBox (ID: {})", created_ip.address, created_ip.id);
                use crate::events::reasons;
                self.record_event_normal(
                    reasons::CREATED,
                    &format!("Created IP address {} in NetBox (ID: {})", created_ip.address, created_ip.id),
                    ip_address_crd,
                ).await;
                created_ip
            }
            Err(e) => {
                if is_conflict_error(&e) {
                    warn!("IP address {} creation conflicted, attempting duplicate detection and remediation", address_str);
                    
                    // Use duplicate detection to find and remediate
                    match self.detect_and_remediate_duplicate_ips(
                        netbox_client.as_ref(),
                        ip_address_crd,
                        address_str.as_str(),
                    ).await {
                        Ok(found) => {
                            info!("Found existing IP address {} in NetBox (ID: {}) after conflict, duplicates remediated", found.address, found.id);
                            found
                        }
                        Err(ControllerError::NetBox(netbox_client::NetBoxError::NotFound(_))) => {
                            // Still not found after remediation - this shouldn't happen after conflict
                            let error_msg = format!("IP address {} creation conflicted but could not find it after remediation: {}", address_str, e);
                            error!("{}", error_msg);
                            update_status_error(&*self.netbox_ip_address_api, name, namespace, error_msg.clone(), ip_address_crd.status.as_ref()).await;
                            return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                        }
                        Err(e) => {
                            let error_msg = format!("IP address {} creation conflicted and duplicate remediation failed: {}", address_str, e);
                            error!("{}", error_msg);
                            update_status_error(&*self.netbox_ip_address_api, name, namespace, error_msg.clone(), ip_address_crd.status.as_ref()).await;
                            return Err(e);
                        }
                    }
                } else {
                    let error_msg = format!("Failed to create IP address in NetBox: {}", e);
                    error!("{}", error_msg);
                    use crate::events::reasons;
                    self.record_event_warning(
                        reasons::RECONCILIATION_FAILED,
                        &error_msg,
                        ip_address_crd,
                    ).await;
                    update_status_error(&*self.netbox_ip_address_api, name, namespace, error_msg.clone(), ip_address_crd.status.as_ref()).await;
                    return Err(ControllerError::NetBox(e));
                }
            }
                    }
                };
        
        // Update status
        let status_patch = Self::create_typed_ip_address_status_patch(
            netbox_ip_address.id,
            netbox_ip_address.url.clone(),
            Some(netbox_ip_address.address.to_string()),
            ResourceState::Created,
            None,
        );
        update_resource_status(
            &*self.netbox_ip_address_api,
            name,
            namespace,
            &status_patch,
            "NetBoxIPAddress",
            netbox_ip_address.id,
        ).await?;
        info!("Updated NetBoxIPAddress {}/{} status: NetBox ID {}", namespace, name, netbox_ip_address.id);
        Ok(())
    }

    /// Global duplicate cleanup: Find and remediate all duplicate IP addresses in NetBox
    /// 
    /// This function:
    /// 1. Queries ALL IP addresses in NetBox
    /// 2. Groups them by address (exact match)
    /// 3. For each group with duplicates, selects the oldest IP (by created timestamp)
    /// 4. Deletes all other duplicates
    /// 
    /// This is useful for:
    /// - Cleaning up orphaned duplicates (not managed by any CRD)
    /// - Running periodic cleanup jobs
    /// - Recovering from bugs that created duplicates
    /// 
    /// Returns: (total_duplicates_found, duplicates_deleted, errors)
    pub async fn cleanup_all_duplicate_ips(
        &self,
        netbox_client: &dyn NetBoxClientTrait,
    ) -> Result<(usize, usize, usize), ControllerError> {
        info!("Starting global duplicate IP address cleanup");
        
        // Query ALL IP addresses in NetBox
        let all_ips = match netbox_client.query_ip_addresses(&[], true).await {
            Ok(ips) => ips,
            Err(e) => {
                error!("Failed to query all IP addresses for global cleanup: {}", e);
                return Err(ControllerError::NetBox(e));
            }
        };
        
        info!("Found {} total IP addresses in NetBox, analyzing for duplicates", all_ips.len());
        
        // Group IPs by address (exact match)
        use std::collections::HashMap;
        let mut ip_groups: HashMap<String, Vec<netbox_client::IPAddress>> = HashMap::new();
        
        for ip in all_ips {
            let address_str = ip.address.to_string();
            ip_groups.entry(address_str).or_insert_with(Vec::new).push(ip);
        }
        
        // Find groups with duplicates (more than 1 IP)
        let duplicate_groups: Vec<(String, Vec<netbox_client::IPAddress>)> = ip_groups
            .into_iter()
            .filter(|(_, ips)| ips.len() > 1)
            .collect();
        
        let total_duplicates = duplicate_groups.iter()
            .map(|(_, ips)| ips.len())
            .sum::<usize>();
        
        info!("Found {} duplicate groups affecting {} total IP addresses", duplicate_groups.len(), total_duplicates);
        
        if duplicate_groups.is_empty() {
            info!("No duplicate IP addresses found, cleanup complete");
            return Ok((0, 0, 0));
        }
        
        // Process each duplicate group
        let mut total_deleted = 0;
        let mut total_errors = 0;
        
        for (address, mut ips) in duplicate_groups {
            // Sort by created timestamp (oldest first)
            ips.sort_by(|a, b| {
                let a_created = chrono::DateTime::parse_from_rfc3339(&a.created)
                    .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                let b_created = chrono::DateTime::parse_from_rfc3339(&b.created)
                    .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                
                match a_created.cmp(&b_created) {
                    std::cmp::Ordering::Equal => {
                        let a_updated = chrono::DateTime::parse_from_rfc3339(&a.last_updated)
                            .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                        let b_updated = chrono::DateTime::parse_from_rfc3339(&b.last_updated)
                            .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
                        a_updated.cmp(&b_updated)
                    }
                    other => other,
                }
            });
            
            // Select the oldest IP (first in sorted list)
            let selected_ip = ips.remove(0);
            let duplicates = ips;
            
            info!("Address {}: Keeping IP ID {} (created: {}), deleting {} duplicates", 
                address, selected_ip.id, selected_ip.created, duplicates.len());
            
            // Delete all duplicates
            for duplicate in duplicates {
                match netbox_client.delete_ip_address(IpAddressId(duplicate.id)).await {
                    Ok(_) => {
                        total_deleted += 1;
                        debug!("Deleted duplicate IP {} (ID: {}, created: {})", 
                            duplicate.address, duplicate.id, duplicate.created);
                    }
                    Err(e) => {
                        total_errors += 1;
                        warn!("Failed to delete duplicate IP {} (ID: {}, created: {}): {}", 
                            duplicate.address, duplicate.id, duplicate.created, e);
                    }
                }
            }
        }
        
        info!("Global duplicate cleanup complete: {} duplicates deleted, {} errors", total_deleted, total_errors);
        Ok((total_duplicates, total_deleted, total_errors))
    }
}

