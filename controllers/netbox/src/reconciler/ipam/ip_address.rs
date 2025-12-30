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
        
        // Parse IP address from spec (required for reconciliation)
        let ip_net = if let Some(address) = &ip_address_crd.spec.address {
            IpNet::from_str(address)
                .map_err(|e| ControllerError::InvalidInput(format!("Invalid IP address format '{}': {}", address, e)))?
        } else {
            // If no address but ip_range is provided, we can't proceed
            // DHCP IPs should have the address specified (assigned by DHCP server)
            let error_msg = "IP address must be specified when ipRange is provided. DHCP-assigned IPs should include the assigned address.".to_string();
            error!("NetBoxIPAddress {}/{}: {}", namespace, name, error_msg);
            update_status_error(&*self.netbox_ip_address_api, name, namespace, error_msg.clone(), ip_address_crd.status.as_ref()).await;
            return Err(ControllerError::InvalidInput(error_msg));
        };
        
        // If both address and ip_range are provided, validate address is within range
        if let (Some(address_str), Some(range_id)) = (&ip_address_crd.spec.address, ip_range_id) {
            // Get the IP range to validate address is within it
            match netbox_client.get_ip_range(netbox_client::IPRangeId(range_id)).await {
                Ok(range) => {
                    let address_ip = ip_net.addr();
                    let range_start = range.start_address.addr();
                    let range_end = range.end_address.addr();
                    
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
        
        let netbox_ip_address = match drift_result {
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
                            let status_patch = Self::create_resource_status_patch(
                                ip.id,
                                ip.url.clone(),
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
                
                // Check if any field changed
                if Self::ip_address_needs_update(
                    &ip_address_crd.spec,
                    &remediated_ip,
                    tenant_id,
                    vlan_id, // Note: vlan_id comparison not implemented in needs_update yet
                    status_str,
                ) {
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
                        tags: None, // Tags are not easily updatable via AllocateIPRequest
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
                            Some(updated_ip)
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
                        let status_patch = Self::create_resource_status_patch(
                            remediated_ip.id,
                            remediated_ip.url.clone(),
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
                // Emit event for drift detection
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DRIFT_DETECTED,
                    &format!("NetBoxIPAddress {}/{} drift detected: {}", namespace, name, message),
                    ip_address_crd,
                ).await;
                
                let status_patch = Self::create_resource_status_patch(
                    0,
                    String::new(),
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
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        // Handle existing IP address (from helper) or create new
        let netbox_ip_address = match netbox_ip_address {
            Some(ip) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    ip_address_crd.status.as_ref(),
                    ip.id,
                    &ip.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        ip.id,
                        ip.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    update_resource_status(
                        &*self.netbox_ip_address_api,
                        name,
                        namespace,
                        &status_patch,
                        "NetBoxIPAddress",
                        ip.id,
                    ).await?;
                    debug!("Updated NetBoxIPAddress {}/{} status: NetBox ID {}", namespace, name, ip.id);
                    return Ok(());
                } else {
                    debug!("NetBoxIPAddress {}/{} already has correct status (ID: {}), skipping update", namespace, name, ip.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create IP address - resolve dependencies first
                validate_reference_kind(&ip_address_crd.spec.tenant, "NetBoxTenant", "tenant", name)?;
                let _tenant_id = match resolve_required_dependency_id(
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
                
                // Try to find existing IP address by address, with duplicate detection and remediation
                let address_str = ip_address_crd.spec.address.as_ref()
                    .ok_or_else(|| ControllerError::InvalidInput("Address is required for IP address reconciliation".to_string()))?;
                
                let netbox_ip_address = match self.detect_and_remediate_duplicate_ips(
                    netbox_client.as_ref(),
                    ip_address_crd,
                    address_str,
                ).await {
                    Ok(existing) => {
                        info!("IP address {} already exists in NetBox (ID: {}), using it", address_str, existing.id);
                        existing
                    }
                    Err(ControllerError::NetBox(netbox_client::NetBoxError::NotFound(_))) => {
                        // No IP found - need to create
                        // Create IP address
                        use netbox_client::AllocateIPRequest;
                        let create_request = AllocateIPRequest {
                        address: Some(ip_net), // Specify the exact IP address
                        description: ip_address_crd.spec.description.clone(),
                        status: Some(netbox_status),
                        role: ip_address_crd.spec.role.clone(),
                        dns_name: ip_address_crd.spec.dns_name.clone(),
                        tags: None, // Tags would need to be converted to NetBox tag format
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
                                    address_str,
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
                }
                    Err(e) => {
                        // Error during duplicate detection - return error
                        return Err(e);
                    }
                };
                
                netbox_ip_address
            }
        };
        
        // Update status
        let status_patch = Self::create_resource_status_patch(
            netbox_ip_address.id,
            netbox_ip_address.url.clone(),
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
}

