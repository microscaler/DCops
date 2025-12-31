//! Helper functions for common reconciliation patterns
//!
//! This module provides reusable functions to eliminate code duplication
//! across all reconcilers.

#[cfg(test)]
mod tests;

use crate::error::ControllerError;
use tracing::{debug, info, warn, error};
use crds;

/// Trait for NetBox resources that have an ID and URL
pub trait NetBoxResource {
    fn id(&self) -> u64;
    #[allow(dead_code)] // May be used in future for status updates
    fn url(&self) -> &str;
}

// Implement for common NetBox resource types
impl NetBoxResource for netbox_client::Site {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::Tenant {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::Role {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::Tag {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::Aggregate {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::Prefix {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::Region {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::DeviceRole {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::SiteGroup {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::Location {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::Manufacturer {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::Platform {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::DeviceType {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::Device {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::Interface {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::Vlan {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}
impl NetBoxResource for netbox_client::MACAddress {
    fn id(&self) -> u64 { self.id }
    fn url(&self) -> &str { &self.url }
}

/// Generic drift detection and update pattern
/// 
/// This function handles the common pattern:
/// 1. Check if resource exists in NetBox (by ID from status)
/// 2. If exists, diff and update if needed
/// 3. If deleted (NotFound), clear status
/// 4. If other error, return error
/// 
/// Returns:
/// - `Ok(Some(existing_resource))` if resource exists and is up-to-date
/// - `Ok(Some(updated_resource))` if resource exists and was updated
/// - `Ok(None)` if resource was deleted (drift detected) or doesn't exist
/// - `Err(e)` if there's an error that should be retried
pub async fn check_and_update_existing<FGet, FUpdate, FNeedsUpdate, Resource>(
    _client: &dyn netbox_client::NetBoxClientTrait,
    netbox_id: u64,
    resource_name: &str,
    get_fn: FGet,
    needs_update_fn: FNeedsUpdate,
    update_fn: FUpdate,
) -> Result<Option<Resource>, ControllerError>
where
    FGet: std::future::Future<Output = Result<Resource, netbox_client::NetBoxError>> + Send,
    FUpdate: std::future::Future<Output = Result<Resource, netbox_client::NetBoxError>> + Send,
    FNeedsUpdate: Fn(&Resource) -> bool,
    Resource: Clone + Send + Sync + NetBoxResource,
{
    match get_fn.await {
        Ok(existing) => {
            // Check if resource needs updating
            if needs_update_fn(&existing) {
                info!("{} (ID: {}) spec changed, updating in NetBox", resource_name, netbox_id);
                match update_fn.await {
                    Ok(updated) => {
                        info!("Updated {} in NetBox (ID: {})", resource_name, updated.id());
                        Ok(Some(updated))
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to update {} in NetBox: {}", resource_name, e);
                        error!("{}", error_msg);
                        Err(ControllerError::NetBox(e))
                    }
                }
            } else {
                debug!("{} (ID: {}) already up-to-date in NetBox", resource_name, netbox_id);
                Ok(Some(existing))
            }
        }
        Err(netbox_client::NetBoxError::NotFound(_)) => {
            // Drift detected - resource was deleted in NetBox
            warn!("{} (ID: {}) was deleted in NetBox (drift detected), will recreate", resource_name, netbox_id);
            Ok(None) // Signal to recreate
        }
        Err(e) => {
            // Other errors (auth, network) - don't assume deleted
            error!("Failed to verify {} (ID: {}) exists: {}, will retry", resource_name, netbox_id, e);
            Err(ControllerError::NetBox(e))
        }
    }
}

/// Clear CR status when drift is detected (resource deleted in NetBox)
/// 
/// This creates a generic status patch that can be used by any reconciler.
/// The patch clears netboxId, netboxUrl, sets state to "Pending", and includes an error message.
/// 
/// Note: This returns a generic JSON structure. For type-specific status patches,
/// reconcilers should use their own create_resource_status_patch/create_prefix_status_patch methods.
/// However, this helper can be used as a template or for simple cases.
#[allow(dead_code)]
pub fn create_pending_status_patch() -> serde_json::Value {
    serde_json::json!({
        "status": {
            "netboxId": 0,
            "netboxUrl": "",
            "state": "Pending",
            "error": "Resource was deleted in NetBox, will recreate"
        }
    })
}

/// Simple drift detection (without diffing/update)
/// 
/// This is for resources that don't have update logic yet.
/// It only checks if the resource exists and detects drift.
/// 
/// **DEPRECATED**: Use `validate_status_and_drift()` instead for better Failed state handling.
/// This function is kept for backward compatibility and tests.
/// 
/// Returns:
/// - `Ok(Some(resource))` if resource exists
/// - `Ok(None)` if resource was deleted (drift detected)
/// - `Err(e)` if there's an error that should be retried
#[allow(dead_code)] // Still used in tests
pub async fn check_existing<FGet, Resource>(
    _client: &dyn netbox_client::NetBoxClientTrait,
    netbox_id: u64,
    resource_name: &str,
    get_fn: FGet,
) -> Result<Option<Resource>, ControllerError>
where
    FGet: std::future::Future<Output = Result<Resource, netbox_client::NetBoxError>> + Send,
    Resource: Clone + Send + Sync + NetBoxResource,
{
    match get_fn.await {
        Ok(existing) => {
            debug!("{} (ID: {}) exists in NetBox", resource_name, netbox_id);
            Ok(Some(existing))
        }
        Err(netbox_client::NetBoxError::NotFound(_)) => {
            // Drift detected - resource was deleted in NetBox
            warn!("{} (ID: {}) was deleted in NetBox (drift detected), will recreate", resource_name, netbox_id);
            Ok(None) // Signal to recreate
        }
        Err(e) => {
            // Other errors (auth, network) - don't assume deleted
            error!("Failed to verify {} (ID: {}) exists: {}, will retry", resource_name, netbox_id, e);
            Err(ControllerError::NetBox(e))
        }
    }
}

/// Clear CR status when drift is detected (resource deleted in NetBox)
/// 
/// This helper creates a status patch that clears the netboxId and sets state to Pending.
/// 
/// **Why this isn't used directly:**
/// Each reconciler has type-specific status patch methods (e.g., `create_resource_status_patch`,
/// `create_prefix_status_patch`, `create_ipclaim_status_patch`) that handle the correct state enum
/// types (`ResourceState::Pending`, `PrefixState::Pending`, `AllocationState::Pending`).
/// 
/// The generic helper here returns a JSON structure with a hardcoded "Pending" string, but
/// CRD validation schemas expect PascalCase enum values that match the specific state enum type.
/// 
/// **Current pattern:** Each reconciler calls its own type-specific method:
/// ```rust
/// let status_patch = Self::create_resource_status_patch(
///     0, // Clear netbox_id
///     String::new(), // Clear URL
///     ResourceState::Pending, // Type-safe enum
///     Some("Resource was deleted in NetBox, will recreate".to_string()),
/// );
/// ```
/// 
/// This ensures type safety and matches the CRD schema exactly.
#[allow(dead_code)]
pub fn create_drift_status_patch() -> serde_json::Value {
    serde_json::json!({
        "status": {
            "netboxId": 0,
            "netboxUrl": "",
            "state": "Pending",
            "error": "Resource was deleted in NetBox, will recreate"
        }
    })
}

// Status update helpers
//
// Due to kube-rs trait bound complexity for patch_status, these patterns are documented
// here but implemented inline in each reconciler. The common patterns are:
//
// 1. Clear status on drift:
//    let status_patch = Self::create_resource_status_patch(0, String::new(), ResourceState::Pending, Some("Resource was deleted in NetBox, will recreate".to_string()));
//    let pp = kube::api::PatchParams::default();
//    if let Err(e) = api.patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch)).await {
//        warn!("Failed to clear status after drift detection: {}", e);
//    }
//
// 2. Update status on success:
//    let status_patch = Self::create_resource_status_patch(resource.id, resource.url.clone(), ResourceState::Created, None);
//    let pp = kube::api::PatchParams::default();
//    match api.patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch)).await {
//        Ok(_) => info!("Updated status: NetBox ID {}", resource.id),
//        Err(e) => return Err(ControllerError::Kube(e.into())),
//    }
//
// 3. Update status with error:
//    let status_patch = Self::create_resource_status_patch(0, String::new(), ResourceState::Failed, Some(error_msg.clone()));
//    let pp = kube::api::PatchParams::default();
//    if let Err(e) = api.patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch)).await {
//        error!("Failed to update error status: {}", e);
//    }
//
// Future enhancement: These could be implemented as macros to reduce boilerplate while
// maintaining type safety.

/// Trait for checking status values without needing specific CRD types
/// 
/// All NetBox CRD status types implement this trait to enable generic status comparison.
/// This allows us to have a single helper function that works for all status types.
pub trait NetBoxStatusCheck {
    fn netbox_id(&self) -> Option<u64>;
    fn netbox_url(&self) -> Option<&str>;
    fn state_str(&self) -> &str;  // Returns string representation of state enum (e.g., "Created", "Pending")
    fn error(&self) -> Option<&str>;
}

// Implement the trait for all NetBox status types
// This allows the generic helper function to work with any status type

impl NetBoxStatusCheck for crds::NetBoxDeviceStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxSiteStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxTenantStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxPrefixStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::PrefixState::Pending => "Pending",
            crds::PrefixState::Created => "Created",
            crds::PrefixState::Updated => "Updated",
            crds::PrefixState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxIPAddressStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxIPRangeStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::IPClaimStatus {
    fn netbox_id(&self) -> Option<u64> { None }  // IPClaim doesn't have netbox_id
    fn netbox_url(&self) -> Option<&str> { self.netbox_ip_ref.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::AllocationState::Pending => "Pending",
            crds::AllocationState::Allocated => "Allocated",
            crds::AllocationState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

/// Extended trait for IPClaim status that includes IP address checking
/// 
/// IPClaim has an additional `ip` field that needs to be checked separately
/// because it's not part of the standard NetBoxStatusCheck trait.
pub trait IPClaimStatusCheck: NetBoxStatusCheck {
    fn allocated_ip(&self) -> Option<&str>;
}

impl IPClaimStatusCheck for crds::IPClaimStatus {
    fn allocated_ip(&self) -> Option<&str> { self.ip.as_deref() }
}

/// Check if IPClaim status needs updating (includes IP address check)
pub fn ipclaim_status_needs_update(
    current_status: Option<&crds::IPClaimStatus>,
    desired_ip: Option<&str>,
    desired_state: &str,
    desired_netbox_ip_ref: Option<&str>,
    desired_error: Option<&str>,
) -> bool {
    match current_status {
        None => {
            // No status - definitely need to update
            true
        }
        Some(status) => {
            // Check if any status field changed
            status.allocated_ip() != desired_ip
                || status.state_str() != desired_state
                || status.netbox_url() != desired_netbox_ip_ref
                || status.error() != desired_error
        }
    }
}

// Implement for all remaining NetBox status types
// They all follow the same pattern with ResourceState

impl NetBoxStatusCheck for crds::NetBoxInterfaceStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxMACAddressStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxRegionStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxSiteGroupStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxLocationStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxDeviceRoleStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxManufacturerStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxPlatformStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxDeviceTypeStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxVLANStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxRoleStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxTagStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxAggregateStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

impl NetBoxStatusCheck for crds::NetBoxRIRStatus {
    fn netbox_id(&self) -> Option<u64> { self.netbox_id }
    fn netbox_url(&self) -> Option<&str> { self.netbox_url.as_deref() }
    fn state_str(&self) -> &str {
        match self.state {
            crds::ResourceState::Pending => "Pending",
            crds::ResourceState::Created => "Created",
            crds::ResourceState::Updated => "Updated",
            crds::ResourceState::Failed => "Failed",
        }
    }
    fn error(&self) -> Option<&str> { self.error.as_deref() }
}

/// Check if status needs updating by comparing current status with desired values
/// 
/// Returns true if status should be updated (values changed), false if status is already correct.
/// This prevents unnecessary status updates that trigger reconciliation loops.
/// 
/// This is a generic function that works with any status type implementing `NetBoxStatusCheck`.
/// 
/// # Example
/// ```rust
/// let needs_update = status_needs_update(
///     device_crd.status.as_ref(),
///     device.id,
///     &device.url,
///     "Created",
///     None,
/// );
/// if needs_update {
///     // Update status
/// } else {
///     // Skip update - status already correct
/// }
/// ```
pub fn status_needs_update<S: NetBoxStatusCheck>(
    current_status: Option<&S>,
    desired_netbox_id: u64,
    desired_netbox_url: &str,
    desired_state: &str,
    desired_error: Option<&str>,
) -> bool {
    match current_status {
        None => {
            // No status - definitely need to update
            true
        }
        Some(status) => {
            // Check if any status field changed
            status.netbox_id() != Some(desired_netbox_id)
                || status.netbox_url().as_deref() != Some(desired_netbox_url)
                || status.state_str() != desired_state
                || status.error() != desired_error
        }
    }
}

/// Helper macro to update status only if it changed
/// 
/// This macro checks if status needs updating, and if so, updates it.
/// If status is already correct, it returns early to skip unnecessary updates.
/// 
/// # Usage
/// ```rust
/// update_status_if_changed!(
///     api: self.netbox_device_api,
///     name: name,
///     namespace: namespace,
///     current_status: device_crd.status.as_ref(),
///     desired_netbox_id: device.id,
///     desired_netbox_url: &device.url,
///     desired_state: ResourceState::Created,
///     desired_error: None,
///     status_patch_fn: Self::create_resource_status_patch,
///     resource_name: "NetBoxDevice",
/// )?;
/// ```
#[macro_export]
macro_rules! update_status_if_changed {
    (
        api: $api:expr,
        name: $name:expr,
        namespace: $namespace:expr,
        current_status: $current_status:expr,
        desired_netbox_id: $desired_netbox_id:expr,
        desired_netbox_url: $desired_netbox_url:expr,
        desired_state: $desired_state:expr,
        desired_error: $desired_error:expr,
        status_patch_fn: $status_patch_fn:expr,
        resource_name: $resource_name:expr,
    ) => {
        {
            use crate::reconcile_helpers::status_needs_update;
            use tracing::debug;
            
            let needs_update = status_needs_update(
                $current_status,
                $desired_netbox_id,
                $desired_netbox_url,
                match $desired_state {
                    $crate::crds::ResourceState::Pending => "Pending",
                    $crate::crds::ResourceState::Created => "Created",
                    $crate::crds::ResourceState::Updated => "Updated",
                    $crate::crds::ResourceState::Failed => "Failed",
                },
                $desired_error.as_deref(),
            );
            
            if needs_update {
                let status_patch = $status_patch_fn(
                    $desired_netbox_id,
                    $desired_netbox_url.to_string(),
                    $desired_state,
                    $desired_error,
                );
                let pp = $crate::kube::api::PatchParams::default();
                match $api
                    .patch_status($name, &pp, &$crate::kube::api::Patch::Merge(&status_patch))
                    .await
                {
                    Ok(_) => {
                        debug!("Updated {} {}/{} status: NetBox ID {}", $resource_name, $namespace, $name, $desired_netbox_id);
                    }
                    Err(e) => {
                        return Err($crate::error::ControllerError::Kube(e.into()));
                    }
                }
            } else {
                debug!("{} {}/{} already has correct status (ID: {}), skipping update", $resource_name, $namespace, $name, $desired_netbox_id);
            }
        }
    };
}

// Macro removed for now - helper functions are sufficient
// Can be added later if boilerplate becomes too much

/// Result of drift detection and status validation
/// 
/// This enum indicates what action the reconciler should take based on
/// the current status and drift detection results.
#[derive(Debug, Clone)]
pub enum DriftCheckResult<Resource> {
    /// Resource exists and is valid - use this resource
    UseExisting(Resource),
    /// Resource needs to be recreated (status cleared, invalid netbox_id, or resource deleted)
    Recreate,
    /// Status was cleared and needs to be updated (caller should update status to Pending)
    StatusCleared {
        message: String,
    },
}

/// Resolve netbox_id from a dependent resource's status.
/// 
/// This helper centralizes the pattern of checking if a dependent resource
/// has been created in NetBox (has a netbox_id). If the resource is not ready,
/// returns None so the caller can return early and let the controller requeue.
/// 
/// # Arguments
/// - `status`: Optional status of the dependent resource (implements `NetBoxStatusCheck`)
/// - `resource_kind`: Human-readable resource kind for logging (e.g., "Device", "Tenant", "Site")
/// - `resource_name`: Name of the dependent resource for logging
/// 
/// # Returns
/// - `Some(u64)` if the resource has a netbox_id (ready to use)
/// - `None` if the resource doesn't have a netbox_id (not ready yet)
/// 
/// # Example
/// ```rust
/// let device_id = resolve_dependency_id(
///     device_crd.status.as_ref(),
///     "Device",
///     device_name,
/// )?;
/// 
/// // If None, return early:
/// let device_id = match resolve_dependency_id(
///     device_crd.status.as_ref(),
///     "Device",
///     device_name,
/// ) {
///     Some(id) => id,
///     None => {
///         debug!("Device '{}' not ready, waiting...", device_name);
///         return Ok(()); // Controller will requeue when device status updates
///     }
/// };
/// ```
pub fn resolve_dependency_id(
    status: Option<&impl NetBoxStatusCheck>,
    resource_kind: &str,
    resource_name: &str,
) -> Option<u64> {
    match status.and_then(|s| s.netbox_id()) {
        Some(id) => Some(id),
        None => {
            debug!("{} '{}' has not been created in NetBox yet (no netbox_id in status), waiting for {} to be created", resource_kind, resource_name, resource_kind);
            None
        }
    }
}

/// Extract name and namespace from CRD metadata.
/// 
/// This helper centralizes the common pattern of extracting name and namespace
/// from CRD metadata with proper error handling.
/// 
/// # Arguments
/// - `crd`: The CRD resource
/// - `resource_kind`: Human-readable resource kind for error messages (e.g., "NetBoxSite")
/// 
/// # Returns
/// - `Ok((name, namespace))` if both are available (namespace defaults to "default" if None)
/// - `Err(InvalidConfig)` if name is missing
/// 
/// # Example
/// ```rust
/// let (name, namespace) = extract_name_and_namespace(&site_crd, "NetBoxSite")?;
/// ```
pub fn extract_name_and_namespace<'a, CRD>(
    crd: &'a CRD,
    resource_kind: &str,
) -> Result<(&'a str, &'a str), ControllerError>
where
    CRD: kube::Resource,
{
    let name = crd.meta().name.as_deref()
        .ok_or_else(|| ControllerError::InvalidConfig(
            format!("{} missing name", resource_kind)
        ))?;
    let namespace = crd.meta().namespace.as_deref()
        .unwrap_or("default");
    Ok((name, namespace))
}

/// Validate that a resource reference kind matches the expected kind.
/// 
/// This helper centralizes the pattern of validating resource reference kinds.
/// 
/// # Arguments
/// - `reference`: The resource reference to validate
/// - `expected_kind`: The expected kind (e.g., "NetBoxTenant")
/// - `reference_name`: Name of the reference field (e.g., "tenant", "site") for error messages
/// - `current_resource_name`: Name of the current resource (for error messages)
/// 
/// # Returns
/// - `Ok(())` if the kind matches
/// - `Err(InvalidConfig)` if the kind doesn't match
/// 
/// # Example
/// ```rust
/// validate_reference_kind(
///     &site_crd.spec.tenant,
///     "NetBoxTenant",
///     "tenant",
///     name,
/// )?;
/// ```
pub fn validate_reference_kind(
    reference: &crds::NetBoxResourceReference,
    expected_kind: &str,
    reference_name: &str,
    current_resource_name: &str,
) -> Result<(), ControllerError> {
    if reference.kind != expected_kind {
        return Err(ControllerError::InvalidConfig(
            format!("Invalid kind '{}' for {} reference in {}, expected '{}'", 
                reference.kind, reference_name, current_resource_name, expected_kind)
        ));
    }
    Ok(())
}

/// Resolve a required dependency's netbox_id from CRD status.
/// 
/// This helper centralizes the pattern of resolving a required dependency's netbox_id.
/// It gets the CRD, extracts the netbox_id from its status using a closure, and returns
/// appropriate errors if the CRD is not found or doesn't have a netbox_id yet.
/// 
/// # Arguments
/// - `api`: Kubernetes API for the dependency CRD type
/// - `dependency_name`: Name of the dependency CRD to fetch
/// - `dependency_kind`: Human-readable kind for error messages (e.g., "Tenant", "DeviceType")
/// - `current_resource_name`: Name of the resource that depends on this dependency (for error messages)
/// - `extract_status`: Closure that extracts the status from the CRD (e.g., `|crd| crd.status.as_ref()`)
/// 
/// # Returns
/// - `Ok(u64)` if the dependency has a netbox_id (ready to use)
/// - `Err(InvalidConfig)` if the CRD is not found or doesn't have a netbox_id
/// 
/// # Example
/// ```rust
/// let tenant_id = resolve_required_dependency_id(
///     &self.netbox_tenant_api,
///     &site_crd.spec.tenant.name,
///     "Tenant",
///     name,
///     |crd| crd.status.as_ref(),
/// ).await?;
/// ```
pub async fn resolve_required_dependency_id<API, CRD, F, S>(
    api: &API,
    dependency_name: &str,
    dependency_kind: &str,
    current_resource_name: &str,
    extract_status: F,
) -> Result<u64, ControllerError>
where
    API: crate::kube_api_trait::KubeApiTrait<CRD> + ?Sized,
    CRD: kube::Resource + Clone + Send + Sync + 'static,
    CRD: std::fmt::Debug + serde::de::DeserializeOwned,
    <CRD as kube::Resource>::DynamicType: Send + Sync,
    F: FnOnce(&CRD) -> Option<&S>,
    S: NetBoxStatusCheck,
{
    match api.get(dependency_name).await {
        Ok(dependency_crd) => {
            match extract_status(&dependency_crd) {
                Some(status) => {
                    match status.netbox_id() {
                        Some(id) => Ok(id),
                        None => Err(ControllerError::InvalidConfig(
                            format!("{} '{}' has not been created in NetBox yet (no netbox_id in status)", dependency_kind, dependency_name)
                        ))
                    }
                }
                None => Err(ControllerError::InvalidConfig(
                    format!("{} '{}' has no status", dependency_kind, dependency_name)
                ))
            }
        }
        Err(_) => {
            Err(ControllerError::InvalidConfig(
                format!("{} CRD '{}' not found for {}", dependency_kind, dependency_name, current_resource_name)
            ))
        }
    }
}

/// Resolve an optional dependency's netbox_id from CRD status.
/// 
/// This helper centralizes the pattern of resolving an optional dependency's netbox_id.
/// It validates the kind, gets the CRD, extracts the netbox_id from its status, and returns
/// None if the CRD is not found or doesn't have a netbox_id (with appropriate warnings).
/// 
/// # Arguments
/// - `api`: Kubernetes API for the dependency CRD type
/// - `reference`: Optional resource reference (None means dependency not specified)
/// - `expected_kind`: The expected kind (e.g., "NetBoxRegion")
/// - `dependency_name`: Name of the dependency field (e.g., "region", "site_group") for error messages
/// - `current_resource_name`: Name of the current resource (for error messages)
/// - `extract_status`: Closure that extracts the status from the CRD (e.g., `|crd| crd.status.as_ref()`)
/// 
/// # Returns
/// - `Some(u64)` if the dependency has a netbox_id (ready to use)
/// - `None` if the reference is None, kind doesn't match, CRD not found, or no netbox_id
/// 
/// # Example
/// ```rust
/// let region_id = resolve_optional_dependency_id(
///     &self.netbox_region_api,
///     site_crd.spec.region.as_ref(),
///     "NetBoxRegion",
///     "region",
///     name,
///     |crd| crd.status.as_ref(),
/// ).await;
/// ```
pub async fn resolve_optional_dependency_id<API, CRD, F, S>(
    api: &API,
    reference: Option<&crds::NetBoxResourceReference>,
    expected_kind: &str,
    dependency_name: &str,
    current_resource_name: &str,
    extract_status: F,
) -> Option<u64>
where
    API: crate::kube_api_trait::KubeApiTrait<CRD> + ?Sized,
    CRD: kube::Resource + Clone + Send + Sync + 'static,
    CRD: std::fmt::Debug + serde::de::DeserializeOwned,
    <CRD as kube::Resource>::DynamicType: Send + Sync,
    F: FnOnce(&CRD) -> Option<&S>,
    S: NetBoxStatusCheck,
{
    let reference = match reference {
        Some(ref_ref) => ref_ref,
        None => return None,
    };
    
    if reference.kind != expected_kind {
        warn!("Invalid kind '{}' for {} reference in {}, expected '{}'", 
            reference.kind, dependency_name, current_resource_name, expected_kind);
        return None;
    }
    
    match api.get(&reference.name).await {
        Ok(dependency_crd) => {
            extract_status(&dependency_crd)
                .and_then(|status| status.netbox_id())
                .and_then(|id| {
                    // Filter out invalid IDs (0) - these indicate the dependency hasn't been created yet
                    if id == 0 {
                        warn!("{} '{}' has invalid netboxId (0) for {} reference in {}, skipping", 
                            expected_kind, reference.name, dependency_name, current_resource_name);
                        None
                    } else {
                        Some(id)
                    }
                })
        }
        Err(_) => {
            warn!("{} CRD '{}' not found for {}, skipping {} reference", 
                expected_kind, reference.name, current_resource_name, dependency_name);
            None
        }
    }
}

/// Update resource status with consistent error handling.
/// 
/// This helper centralizes the pattern of patching resource status with proper
/// error handling and logging. It works with any status patch created by the
/// reconciler's status patch creation methods.
/// 
/// # Arguments
/// - `api`: Kubernetes API for the CRD type
/// - `name`: Resource name
/// - `namespace`: Resource namespace
/// - `status_patch`: Pre-created status patch JSON (from `create_resource_status_patch`, `create_typed_*_status_patch`, etc.)
/// - `resource_name`: Human-readable resource name for logging (e.g., "NetBoxSite")
/// - `netbox_id`: Optional NetBox ID for logging (use 0 if not available)
/// 
/// # Returns
/// - `Ok(())` if status was updated successfully
/// - `Err(ControllerError::Kube)` if the patch failed
/// 
/// # Example
/// ```rust
/// let status_patch = Self::create_resource_status_patch(
///     netbox_id,
///     netbox_url,
///     ResourceState::Created,
///     None,
/// );
/// update_resource_status(
///     &*self.netbox_site_api,
///     name,
///     namespace,
///     status_patch,
///     "NetBoxSite",
///     netbox_id,
/// ).await?;
/// ```
pub async fn update_resource_status<API, CRD>(
    api: &API,
    name: &str,
    namespace: &str,
    status_patch: &serde_json::Value,
    resource_name: &str,
    netbox_id: u64,
) -> Result<(), ControllerError>
where
    API: crate::kube_api_trait::KubeApiTrait<CRD> + ?Sized,
    CRD: kube::Resource + Clone + Send + Sync + 'static,
    CRD: std::fmt::Debug + serde::de::DeserializeOwned,
    <CRD as kube::Resource>::DynamicType: Send + Sync,
{
    let pp = kube::api::PatchParams::default();
    match api
        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
        .await
    {
        Ok(_) => {
            if netbox_id > 0 {
                debug!("Updated {} {}/{} status: NetBox ID {}", resource_name, namespace, name, netbox_id);
            } else {
                debug!("Updated {} {}/{} status", resource_name, namespace, name);
            }
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("Failed to update {} status: {}", resource_name, e);
            error!("{}", error_msg);
            Err(ControllerError::Kube(e.into()))
        }
    }
}

/// Validate status and handle drift detection for Failed/Created states
/// 
/// This helper centralizes the common pattern of:
/// 1. Checking if status has Failed state with invalid netbox_id (0) - clear and recreate
/// 2. Checking if status has Failed state with valid netbox_id - verify resource exists
/// 3. Checking if status has Created state with invalid netbox_id (0) - clear and recreate
/// 4. Checking if status has Created state with valid netbox_id - verify resource exists
/// 
/// # Arguments
/// - `status`: Current CR status (implements `NetBoxStatusCheck`)
/// - `resource_name`: Human-readable resource name for logging (e.g., "NetBoxSite")
/// - `namespace`: Kubernetes namespace
/// - `name`: Kubernetes resource name
/// - `get_resource_fn`: Async function that takes netbox_id (u64) and fetches resource from NetBox
/// 
/// # Returns
/// - `Ok(DriftCheckResult::UseExisting(resource))` if resource exists and is valid
/// - `Ok(DriftCheckResult::Recreate)` if resource should be recreated (status already cleared or resource deleted)
/// - `Ok(DriftCheckResult::StatusCleared { message })` if status was cleared and needs to be updated
/// - `Err(e)` if there's an error that should be retried
/// 
/// # Example
/// ```rust
/// let result = validate_status_and_drift::<netbox_client::Site, _>(
///     site_crd.status.as_ref(),
///     "NetBoxSite",
///     namespace,
///     name,
///     |id| async move { netbox_client.get_site(SiteId(id)).await },
/// ).await?;
/// 
/// match result {
///     DriftCheckResult::UseExisting(site) => {
///         // Use existing site for updates
///     }
///     DriftCheckResult::Recreate => {
///         // Create new resource
///     }
///     DriftCheckResult::StatusCleared { message } => {
///         // Status was cleared, update it to Pending
///         let status_patch = Self::create_resource_status_patch(0, String::new(), ResourceState::Pending, Some(message));
///         // ... patch status ...
///         // Then proceed to create
///     }
/// }
/// ```
pub async fn validate_status_and_drift<Resource, FGet, Fut>(
    status: Option<&impl NetBoxStatusCheck>,
    resource_name: &str,
    namespace: &str,
    name: &str,
    get_resource_fn: FGet,
) -> Result<DriftCheckResult<Resource>, ControllerError>
where
    FGet: FnOnce(u64) -> Fut,
    Fut: std::future::Future<Output = Result<Resource, netbox_client::NetBoxError>> + Send,
    Resource: Clone + Send + Sync,
{
    let status = match status {
        Some(s) => s,
        None => {
            // No status - need to create
            return Ok(DriftCheckResult::Recreate);
        }
    };

    let state_str = status.state_str();
    let netbox_id = status.netbox_id();

    // Handle Failed state
    if state_str == "Failed" {
        match netbox_id {
            Some(id) if id == 0 => {
                // Failed state with invalid netbox_id (0) - clear status and recreate
                warn!("{} {}/{} has Failed state with invalid netbox_id (0), clearing status and will recreate", resource_name, namespace, name);
                return Ok(DriftCheckResult::StatusCleared {
                    message: format!("Clearing Failed status with invalid netbox_id (0), will recreate"),
                });
            }
            Some(id) => {
                // Failed state with valid netbox_id - check if resource still exists
                match get_resource_fn(id).await {
                    Ok(existing) => {
                        // Resource exists, update status to Created
                        info!("{} {}/{} exists in NetBox (ID: {}), updating status from Failed to Created", resource_name, namespace, name, id);
                        return Ok(DriftCheckResult::UseExisting(existing));
                    }
                    Err(netbox_client::NetBoxError::NotFound(_)) => {
                        // Resource doesn't exist, clear status and recreate
                        warn!("{} {}/{} has Failed status but resource doesn't exist in NetBox (ID: {}), clearing status and will recreate", resource_name, namespace, name, id);
                        return Ok(DriftCheckResult::StatusCleared {
                            message: format!("Resource with Failed status doesn't exist in NetBox, will recreate"),
                        });
                    }
                    Err(e) => {
                        // Other errors - retry
                        error!("Failed to verify {} {}/{} (ID: {}) exists: {}, will retry", resource_name, namespace, name, id, e);
                        return Err(ControllerError::NetBox(e));
                    }
                }
            }
            None => {
                // Failed state but no netbox_id - retry creation
                return Ok(DriftCheckResult::Recreate);
            }
        }
    }

    // Handle Created state
    if state_str == "Created" {
        match netbox_id {
            Some(id) if id == 0 => {
                // Created state with invalid netbox_id (0) - clear status and recreate
                warn!("{} {}/{} has invalid netbox_id (0), clearing status and will recreate", resource_name, namespace, name);
                return Ok(DriftCheckResult::StatusCleared {
                    message: format!("Invalid netbox_id (0) detected, will recreate"),
                });
            }
            Some(id) => {
                // Created state with valid netbox_id - verify resource exists
                match get_resource_fn(id).await {
                    Ok(existing) => {
                        // Resource exists and is valid
                        return Ok(DriftCheckResult::UseExisting(existing));
                    }
                    Err(netbox_client::NetBoxError::NotFound(_)) => {
                        // Drift detected - resource was deleted
                        warn!("{} {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", resource_name, namespace, name, id);
                        return Ok(DriftCheckResult::StatusCleared {
                            message: format!("Resource was deleted in NetBox, will recreate"),
                        });
                    }
                    Err(e) => {
                        // Other errors - retry
                        error!("Failed to verify {} {}/{} (ID: {}) exists: {}, will retry", resource_name, namespace, name, id, e);
                        return Err(ControllerError::NetBox(e));
                    }
                }
            }
            None => {
                // Created state but no netbox_id - need to create
                return Ok(DriftCheckResult::Recreate);
            }
        }
    }

    // Other states (Pending, Updated) - need to create or retry
    Ok(DriftCheckResult::Recreate)
}

// Conflict handling helpers for GitOps compliance
//
// GitOps Principle 3: "If something can't be created due to conflict, query for existing
// resource and use it if found."

/// Compare tags between existing NetBox resource and desired CRD spec
/// 
/// This helper compares tags by name (not ID) because:
/// - We don't have resolved tag IDs at comparison time
/// - Tag resolution happens later when actually updating
/// - Comparing by name is sufficient to detect changes
/// 
/// Returns `true` if tags differ, `false` if they match.
pub fn tags_differ(
    existing_tags: &[netbox_client::NestedTag],
    desired_tag_refs: &Option<Vec<crds::NetBoxResourceReference>>,
) -> bool {
    use std::collections::HashSet;
    
    // Extract tag names from existing NetBox resource
    let existing_tag_names: HashSet<String> = existing_tags.iter()
        .map(|t| t.name.clone())
        .collect();
    
    // Extract tag names from desired CRD spec
    let desired_tag_names: HashSet<String> = desired_tag_refs.as_ref()
        .map(|tags| tags.iter().map(|t| t.name.clone()).collect())
        .unwrap_or_default();
    
    // Compare sets
    if existing_tag_names != desired_tag_names {
        debug!("Tags differ: existing {:?} vs desired {:?}", existing_tag_names, desired_tag_names);
        return true;
    }
    
    false
}

/// Convert resolved tag references from Vec<serde_json::Value> to Vec<String>
/// 
/// This helper converts tag IDs or dictionaries to string format expected by NetBox API.
/// Tag IDs are converted to strings, and dictionaries with "slug" are extracted.
pub fn convert_tags_to_strings(tags_json: Option<Vec<serde_json::Value>>) -> Option<Vec<String>> {
    tags_json.map(|tags| {
        tags.into_iter()
            .filter_map(|tag_value| {
                if let Some(id) = tag_value.as_u64() {
                    Some(id.to_string())
                } else if let Some(dict) = tag_value.as_object() {
                    dict.get("slug")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::tags_differ;
    use netbox_client::NestedTag;
    
    fn create_nested_tag(id: u64, name: &str) -> NestedTag {
        NestedTag {
            id,
            url: format!("http://test/api/extras/tags/{}/", id),
            display: name.to_string(),
            name: name.to_string(),
            slug: name.to_string(),
        }
    }
    
    fn create_tag_ref(name: &str) -> crds::NetBoxResourceReference {
        crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTag".to_string(),
            name: name.to_string(),
            namespace: None,
        }
    }
    
    #[test]
    fn test_tags_differ_empty_vs_empty() {
        let existing: Vec<NestedTag> = vec![];
        let desired: Option<Vec<crds::NetBoxResourceReference>> = None;
        
        assert!(!tags_differ(&existing, &desired), 
            "Empty tags should not differ");
    }
    
    #[test]
    fn test_tags_differ_empty_vs_some() {
        let existing: Vec<NestedTag> = vec![];
        let desired = Some(vec![create_tag_ref("tag1")]);
        
        assert!(tags_differ(&existing, &desired), 
            "Empty existing vs some desired should differ");
    }
    
    #[test]
    fn test_tags_differ_some_vs_empty() {
        let existing = vec![create_nested_tag(1, "tag1")];
        let desired: Option<Vec<crds::NetBoxResourceReference>> = None;
        
        assert!(tags_differ(&existing, &desired), 
            "Some existing vs empty desired should differ");
    }
    
    #[test]
    fn test_tags_differ_same_tags() {
        let existing = vec![
            create_nested_tag(1, "tag1"),
            create_nested_tag(2, "tag2"),
        ];
        let desired = Some(vec![
            create_tag_ref("tag1"),
            create_tag_ref("tag2"),
        ]);
        
        assert!(!tags_differ(&existing, &desired), 
            "Same tags should not differ");
    }
    
    #[test]
    fn test_tags_differ_different_tags() {
        let existing = vec![create_nested_tag(1, "tag1")];
        let desired = Some(vec![create_tag_ref("tag2")]);
        
        assert!(tags_differ(&existing, &desired), 
            "Different tags should differ");
    }
    
    #[test]
    fn test_tags_differ_different_order() {
        let existing = vec![
            create_nested_tag(1, "tag1"),
            create_nested_tag(2, "tag2"),
        ];
        let desired = Some(vec![
            create_tag_ref("tag2"),
            create_tag_ref("tag1"),
        ]);
        
        assert!(!tags_differ(&existing, &desired), 
            "Tags in different order should not differ (order doesn't matter)");
    }
    
    #[test]
    fn test_tags_differ_extra_existing() {
        let existing = vec![
            create_nested_tag(1, "tag1"),
            create_nested_tag(2, "tag2"),
        ];
        let desired = Some(vec![create_tag_ref("tag1")]);
        
        assert!(tags_differ(&existing, &desired), 
            "Extra existing tags should differ");
    }
    
    #[test]
    fn test_tags_differ_extra_desired() {
        let existing = vec![create_nested_tag(1, "tag1")];
        let desired = Some(vec![
            create_tag_ref("tag1"),
            create_tag_ref("tag2"),
        ]);
        
        assert!(tags_differ(&existing, &desired), 
            "Extra desired tags should differ");
    }
    
    #[test]
    fn test_tags_differ_case_sensitive() {
        let existing = vec![create_nested_tag(1, "Tag1")];
        let desired = Some(vec![create_tag_ref("tag1")]);
        
        assert!(tags_differ(&existing, &desired), 
            "Tags should be case-sensitive");
    }
}

pub fn is_conflict_error(error: &netbox_client::NetBoxError) -> bool {
    let error_str = format!("{}", error);
    error_str.contains("already exists") ||
    error_str.contains("duplicate") ||
    error_str.contains("unique constraint") ||
    error_str.contains("tenant with this name already exists") ||
    error_str.contains("tenant with this slug already exists") ||
    (error_str.contains("slug") && error_str.contains("already")) ||
    error_str.contains("asset tag") ||
    error_str.contains("overlap") || // IP ranges/addresses that overlap with existing ranges
    (error_str.contains("range") && error_str.contains("in VRF")) // IP range already exists
}

// Handle CREATE conflict errors by trying multiple query strategies (GitOps idempotency)
//
// NOTE: This helper is currently unused due to Rust closure/async complexity.
// Reconcilers use `is_conflict_error` to check for conflicts, then implement the
// query pattern inline. This is still WET code that should be refactored.
//
// Future improvement: Create a macro or simpler helper pattern to eliminate
// the duplicated query logic across reconcilers.
//
// Current pattern used in reconcilers:
// if is_conflict_error(&e) {
//     // Try query strategy 1
//     // Try query strategy 2
//     // Try fallback query
//     // Use found resource or error
// }

