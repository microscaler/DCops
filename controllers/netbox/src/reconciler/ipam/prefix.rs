//! NetBoxPrefix reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers::check_and_update_existing;
use crate::kube_api_trait::KubeApiTrait;
use tracing::{info, error, debug, warn};
use crds::{NetBoxPrefix, NetBoxPrefixStatus, PrefixState};
use netbox_client::{NetBoxClientTrait, PrefixId, TenantId, SiteId, VlanId, RoleId};
use std::str::FromStr;
use ipnet::IpNet;

impl Reconciler {
    /// Check if prefix needs updating by comparing spec with existing NetBox resource
    /// 
    /// Note: NetBox Prefix model doesn't have a `site` field in the response,
    /// but site can be set via API. We can't compare site from existing resource,
    /// so we'll update if other fields changed. Site updates will be handled by
    /// always including site_id in update calls when provided.
    fn prefix_needs_update(
        spec: &crds::NetBoxPrefixSpec,
        existing: &netbox_client::Prefix,
        desired_tenant_id: u64, // tenant is now required
        _desired_site_id: Option<u64>, // Prefix model doesn't have site field, can't compare
        desired_vlan_id: Option<u32>,
        desired_role_id: Option<u64>,
        desired_status: &str,
    ) -> bool {
        // Compare tenant
        let existing_tenant_id = existing.tenant.as_ref().map(|t| t.id);
        if Some(desired_tenant_id) != existing_tenant_id {
            debug!("Prefix tenant changed: {:?} -> {}", existing_tenant_id, desired_tenant_id);
            return true;
        }
        
        // Compare vlan
        let existing_vlan_id = existing.vlan.as_ref().map(|v| v.id as u32);
        if desired_vlan_id != existing_vlan_id {
            debug!("Prefix vlan changed: {:?} -> {:?}", existing_vlan_id, desired_vlan_id);
            return true;
        }
        
        // Compare role
        let existing_role_id = existing.role.as_ref().map(|r| r.id);
        if desired_role_id != existing_role_id {
            debug!("Prefix role changed: {:?} -> {:?}", existing_role_id, desired_role_id);
            return true;
        }
        
        // Compare description - Prefix model has description as String, not Option<String>
        let spec_desc = spec.description.as_deref().unwrap_or("");
        if spec_desc != existing.description {
            debug!("Prefix description changed: '{}' -> '{}'", existing.description, spec_desc);
            return true;
        }
        
        // Compare status
        let existing_status = match existing.status {
            netbox_client::PrefixStatus::Active => "active",
            netbox_client::PrefixStatus::Reserved => "reserved",
            netbox_client::PrefixStatus::Deprecated => "deprecated",
            netbox_client::PrefixStatus::Container => "container",
        };
        if desired_status != existing_status {
            debug!("Prefix status changed: '{}' -> '{}'", existing_status, desired_status);
            return true;
        }
        
        false // No changes needed
    }

    pub async fn reconcile_netbox_prefix(&self, prefix_crd: &NetBoxPrefix) -> Result<(), ControllerError> {
        // Extract name and namespace using helper
        use crate::reconcile_helpers::extract_name_and_namespace;
        let (name, namespace) = extract_name_and_namespace(prefix_crd, "NetBoxPrefix")?;
        
        info!("Reconciling NetBoxPrefix {}/{}", namespace, name);
        
        // SINGLE POINT: Get tenant-specific client
        let tenant_ref = &prefix_crd.spec.tenant;
        let netbox_client = self.token_resolver
            .create_client_for_tenant(namespace, tenant_ref)
            .await?;
        
        // Helper function to update status with error
        async fn update_status_error(
            api: &dyn KubeApiTrait<NetBoxPrefix>,
            name: &str,
            namespace: &str,
            error_msg: String,
            current_status: Option<&NetBoxPrefixStatus>,
        ) {
            // Check if error is already set to avoid unnecessary updates
            if let Some(status) = current_status {
                if status.state == PrefixState::Failed && status.error.as_ref() == Some(&error_msg) {
                    debug!("NetBoxPrefix {}/{} already has this error in status, skipping update", namespace, name);
                    return;
                }
            }
            
            // Update status with error (use lowercase state to match CRD validation schema)
            let status_patch = Reconciler::create_prefix_status_patch(
                0, // No netbox_id on error
                String::new(), // No URL on error
                PrefixState::Failed,
                Some(error_msg.clone()),
            );
            
            let pp = kube::api::PatchParams::default();
            if let Err(e) = api
                .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                .await
            {
                error!("Failed to update NetBoxPrefix {}/{} error status: {}", namespace, name, e);
            } else {
                info!("Updated NetBoxPrefix {}/{} status with error", namespace, name);
            }
        }
        
        // Convert PrefixStatus enum to string for NetBox API
        let status_str = match prefix_crd.spec.status {
            crds::PrefixStatus::Active => "active",
            crds::PrefixStatus::Reserved => "reserved",
            crds::PrefixStatus::Deprecated => "deprecated",
            crds::PrefixStatus::Container => "container",
        };
        
        // Resolve all references first (needed for both update detection and creation) using helpers
        use crate::reconcile_helpers::{validate_reference_kind, resolve_required_dependency_id, resolve_optional_dependency_id};
        
        // Resolve optional Site reference
        let site_id: Option<u64> = resolve_optional_dependency_id(
            &*self.netbox_site_api,
            prefix_crd.spec.site.as_ref(),
            "NetBoxSite",
            "site",
            name,
            |crd| crd.status.as_ref(),
        ).await;
        
        // Resolve optional VLAN reference (convert to u32 for VlanId)
        let vlan_id: Option<u32> = resolve_optional_dependency_id(
            &*self.netbox_vlan_api,
            prefix_crd.spec.vlan.as_ref(),
            "NetBoxVLAN",
            "vlan",
            name,
            |crd| crd.status.as_ref(),
        ).await.map(|id| id as u32);
        
        // Validate and resolve Tenant reference (required)
        validate_reference_kind(&prefix_crd.spec.tenant, "NetBoxTenant", "tenant", name)?;
        let tenant_id = match resolve_required_dependency_id(
            &*self.netbox_tenant_api,
            &prefix_crd.spec.tenant.name,
            "Tenant",
            name,
            |crd| crd.status.as_ref(),
        ).await {
            Ok(id) => id,
            Err(e) => {
                // Emit event for dependency not found
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::DEPENDENCY_NOT_FOUND,
                    &format!("Tenant '{}' not found or not ready: {}", prefix_crd.spec.tenant.name, e),
                    prefix_crd,
                ).await;
                return Err(e);
            }
        };
        
        // Resolve optional Role reference
        let role_id: Option<u64> = resolve_optional_dependency_id(
            &*self.netbox_role_api,
            prefix_crd.spec.role.as_ref(),
            "NetBoxRole",
            "role",
            name,
            |crd| crd.status.as_ref(),
        ).await;
        
        // Check if already created - use helper for drift detection and updates
        let netbox_prefix = if let Some(status) = &prefix_crd.status {
            // Skip if it's a permanent auth error
            if status.state == PrefixState::Failed {
                if let Some(error) = &status.error {
                    if error.contains("Invalid token") || error.contains("403 Forbidden") {
                        debug!("NetBoxPrefix {}/{} already marked as failed with authentication error, skipping reconciliation", namespace, name);
                        return Ok(());
                    }
                }
            }
            
            if status.state == PrefixState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Use helper function for drift detection, diffing, and updating
                    match check_and_update_existing(
                        netbox_client.as_ref(),
                        netbox_id,
                        &format!("NetBoxPrefix {}/{}", namespace, name),
                        netbox_client.get_prefix(PrefixId(netbox_id)),
                        |existing| Self::prefix_needs_update(
                            &prefix_crd.spec,
                            existing,
                            tenant_id,
                            site_id,
                            vlan_id,
                            role_id,
                            &status_str,
                        ),
                        netbox_client.update_prefix(
                            PrefixId(netbox_id),
                            None, // prefix - don't update prefix CIDR
                            prefix_crd.spec.description.clone(),
                            Some(status_str),
                            None, // role - role_id not easily convertible to role name, omit for now
                            Some(TenantId(tenant_id)), // tenant is now required
                            site_id.map(SiteId), // Include site if resolved
                            vlan_id.map(VlanId), // Include vlan if resolved
                            None, // tags - omit for now
                        ),
                    ).await {
                        Ok(Some(resource)) => {
                            // Resource exists and is up-to-date (or was updated)
                            Some(resource)
                        }
                        Ok(None) => {
                            // Drift detected - resource was deleted, clear status and recreate
                            warn!("NetBoxPrefix {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_prefix_status_patch(
                                0, // Clear netbox_id
                                String::new(), // Clear URL
                                PrefixState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_prefix_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                                .await
                            {
                                warn!("Failed to clear NetBoxPrefix status after drift detection: {}", e);
                            }
                            // Fall through to creation
                            None
                        }
                        Err(e) => {
                            // Error during drift detection/update - return to retry
                            return Err(e);
                        }
                    }
                } else {
                    None // No netbox_id, need to create
                }
            } else {
                // Check if resource exists even if status is Failed (idempotency)
                if status.state == PrefixState::Failed && status.netbox_id.is_some() {
                    if let Some(netbox_id) = status.netbox_id {
                        info!("NetBoxPrefix {}/{} has Failed status, checking if resource exists in NetBox for idempotency", namespace, name);
                        // Try to get the resource - if it exists, we'll update status to Created
                        match netbox_client.get_prefix(PrefixId(netbox_id)).await {
                            Ok(existing) => {
                                info!("NetBoxPrefix {}/{} exists in NetBox (ID: {}), updating status from Failed to Created", namespace, name, netbox_id);
                                Some(existing)
                            }
                            Err(_) => None // Resource doesn't exist, need to create
                        }
                    } else {
                        None
                    }
                } else {
                    None // Not in Created state, need to create
                }
            }
        } else {
            None // No status, need to create
        };
        
        // Handle existing prefix (from helper) or create new
        let netbox_prefix = match netbox_prefix {
            Some(prefix) => {
                // Resource exists and is up-to-date - only update status if it changed
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    prefix_crd.status.as_ref(),
                    prefix.id,
                    &prefix.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_prefix_status_patch(
                        prefix.id,
                        prefix.url.clone(),
                        PrefixState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_prefix_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxPrefix {}/{} status: NetBox ID {}", namespace, name, prefix.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxPrefix status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxPrefix {}/{} already has correct status (ID: {}), skipping update", namespace, name, prefix.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create prefix - try to find existing by prefix CIDR (idempotency fallback)
                // Convert PrefixStatus enum to string for NetBox API
                let status_str = match prefix_crd.spec.status {
                    crds::PrefixStatus::Active => "active",
                    crds::PrefixStatus::Reserved => "reserved",
                    crds::PrefixStatus::Deprecated => "deprecated",
                    crds::PrefixStatus::Container => "container",
                };
                
                // Resolve optional VLAN reference (convert to u32 for VlanId) using helper
                let vlan_id: Option<u32> = resolve_optional_dependency_id(
                    &*self.netbox_vlan_api,
                    prefix_crd.spec.vlan.as_ref(),
                    "NetBoxVLAN",
                    "vlan",
                    name,
                    |crd| crd.status.as_ref(),
                ).await.map(|id| id as u32);
                
                // Resolve optional Site reference using helper
                let site_id: Option<u64> = resolve_optional_dependency_id(
                    &*self.netbox_site_api,
                    prefix_crd.spec.site.as_ref(),
                    "NetBoxSite",
                    "site",
                    name,
                    |crd| crd.status.as_ref(),
                ).await;
                
                // Resolve Tenant reference (required) - need ID for NetBox API using helper
                validate_reference_kind(&prefix_crd.spec.tenant, "NetBoxTenant", "tenant", name)?;
                let tenant_id = resolve_required_dependency_id(
                    &*self.netbox_tenant_api,
                    &prefix_crd.spec.tenant.name,
                    "Tenant",
                    name,
                    |crd| crd.status.as_ref(),
                ).await?;
                
                // Resolve optional Role reference using helper
                let role_id: Option<u64> = resolve_optional_dependency_id(
                    &*self.netbox_role_api,
                    prefix_crd.spec.role.as_ref(),
                    "NetBoxRole",
                    "role",
                    name,
                    |crd| crd.status.as_ref(),
                ).await;
        
                // Convert CRD string to IpNet for comparison
                let prefix_net = IpNet::from_str(&prefix_crd.spec.prefix)
                    .map_err(|e| ControllerError::InvalidIPFormat(format!("Invalid prefix format in CRD: {} - {}", prefix_crd.spec.prefix, e)))?;
        
                // Try to find existing prefix by querying NetBox (idempotency fallback)
                let existing_prefix = match netbox_client.query_prefixes(
                    &[("prefix", &prefix_crd.spec.prefix)],
                    false, // Just check first page
                ).await {
                    Ok(prefixes) => {
                        prefixes.iter().find(|p| p.prefix == prefix_net).cloned()
                    }
                    Err(e) => {
                        // Query failed - try alternative methods to find existing prefix
                        warn!("Failed to query prefixes in NetBox: {}, trying alternative methods", e);
                        
                        // Try to get all prefixes and search (if fetch_all works)
                        match netbox_client.query_prefixes(
                            &[],
                            true, // fetch_all
                        ).await {
                            Ok(all_prefixes) => {
                                all_prefixes.iter().find(|p| p.prefix == prefix_net).cloned()
                            }
                            Err(_) => {
                                warn!("Could not query prefixes, will try to create (resource may already exist)");
                                None
                            }
                        }
                    }
                };
                
                let netbox_prefix = if let Some(existing) = existing_prefix {
                    // Prefix exists in NetBox - this is the idempotent case
                    info!("Prefix {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", prefix_net, existing.id);
                    
                    // Update prefix if needed (tenant, site, vlan, description, status)
                    // Note: Omitting role and tags for now (requires numeric IDs or string slugs)
                    match netbox_client.update_prefix(
                        PrefixId(existing.id),
                        None, // prefix - don't update prefix CIDR
                        prefix_crd.spec.description.clone(),
                        Some(status_str),
                        None, // role - role_id not easily convertible to role name, omit for now
                        Some(TenantId(tenant_id)), // tenant is now required
                        site_id.map(SiteId), // Include site if resolved
                        vlan_id.map(VlanId), // Include vlan if resolved
                        None, // tags - omit for now (requires numeric IDs or tag slugs)
                    ).await {
                        Ok(updated) => {
                            info!("Updated prefix {} in NetBox (ID: {})", updated.prefix.to_string(), updated.id);
                            
                            // Emit event for successful update
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::UPDATED,
                                &format!("Updated prefix {} in NetBox (ID: {})", updated.prefix.to_string(), updated.id),
                                prefix_crd,
                            ).await;
                            updated
                        }
                        Err(e) => {
                            // If update fails, use the existing prefix we already have
                            // This is still a success case - resource exists, we just couldn't update it
                            warn!("Failed to update prefix in NetBox: {}, but resource exists (ID: {}), using existing data", e, existing.id);
                            existing
                        }
                    }
                } else {
                    // Prefix doesn't exist, create it
                    debug!("Attempting to create prefix {} in NetBox", prefix_net);
                    
                    // NetBox API requires site and role to be numeric IDs
                    // Tags must be numeric IDs or tag slugs
                    // TODO: Add support for resolving tag names to tag slugs
                    match netbox_client.create_prefix(
                        &prefix_net,
                        prefix_crd.spec.description.clone(),
                        site_id.map(SiteId),
                        vlan_id.map(VlanId),
                        Some(status_str),
                        role_id.map(RoleId),
                        Some(TenantId(tenant_id)), // tenant is now required
                        None, // tags - omit for now (requires numeric IDs or tag slugs)
                    ).await {
                        Ok(created) => {
                            info!("Created prefix {} in NetBox (ID: {})", created.prefix.to_string(), created.id);
                            
                            // Emit event for successful creation
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::CREATED,
                                &format!("Created prefix {} in NetBox (ID: {})", created.prefix.to_string(), created.id),
                                prefix_crd,
                            ).await;
                            created
                        }
                        Err(e) => {
                            // Check if error is "already exists" - if so, try to find it (idempotency)
                            let error_str = format!("{}", e);
                            if error_str.contains("already exists") || error_str.contains("duplicate") || error_str.contains("unique constraint") {
                                warn!("Prefix {} already exists in NetBox, attempting to retrieve it (idempotency)", prefix_net);
                                
                                // Try to find the existing prefix using fetch_all
                                match netbox_client.query_prefixes(
                                    &[],
                                    true, // fetch_all
                                ).await {
                                    Ok(all_prefixes) => {
                                        if let Some(found) = all_prefixes.iter().find(|p| p.prefix == prefix_net) {
                                            info!("Found existing prefix {} in NetBox (ID: {}) after create conflict", found.prefix.to_string(), found.id);
                                            found.clone()
                                        } else {
                                            // Prefix exists but we can't find it - this is unusual
                                            let error_msg = format!("Prefix {} already exists in NetBox but could not retrieve it: {}", prefix_net, e);
                                            error!("{}", error_msg);
                                            update_status_error(&*self.netbox_prefix_api, name, namespace, error_msg.clone(), prefix_crd.status.as_ref()).await;
                                            return Err(ControllerError::NetBox(e));
                                        }
                                    }
                                    Err(query_err) => {
                                        // Couldn't query - this is a real error
                                        let error_msg = format!("Failed to create prefix in NetBox (may already exist, but could not verify): {} (query error: {})", e, query_err);
                                        error!("{}", error_msg);
                                        update_status_error(&*self.netbox_prefix_api, name, namespace, error_msg.clone(), prefix_crd.status.as_ref()).await;
                                        // Emit event for reconciliation failure
                                        use crate::events::reasons;
                                        self.record_event_warning(
                                            reasons::RECONCILIATION_FAILED,
                                            &error_msg,
                                            prefix_crd,
                                        ).await;
                                        return Err(ControllerError::NetBox(e));
                                    }
                                }
                            } else {
                                // Real creation error
                                let error_msg = format!("Failed to create prefix in NetBox: {}", e);
                                error!("{}", error_msg);
                                update_status_error(&*self.netbox_prefix_api, name, namespace, error_msg.clone(), prefix_crd.status.as_ref()).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                };
                
                netbox_prefix
            }
        };
        
        // Update NetBoxPrefix status with success
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_prefix_status_patch(
            netbox_prefix.id,
            netbox_prefix.url.clone(),
            PrefixState::Created,
            None,
        );
        
        // Patch the status using kube-rs status subresource API
        use kube::api::PatchParams;
        let pp = PatchParams::default();
        match self.netbox_prefix_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxPrefix {}/{} status (NetBox ID: {})", namespace, name, netbox_prefix.id);
                // Reset error count on success
                let resource_key = format!("{}/{}", namespace, name);
                self.reset_error(&resource_key);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxPrefix status: {}", e);
                error!("{}", error_msg);
                update_status_error(&*self.netbox_prefix_api, name, namespace, error_msg.clone(), prefix_crd.status.as_ref()).await;
                // Emit event for reconciliation failure
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::RECONCILIATION_FAILED,
                    &error_msg,
                    prefix_crd,
                ).await;
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
