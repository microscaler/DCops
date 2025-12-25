//! Helper functions for common reconciliation patterns
//!
//! This module provides reusable functions to eliminate code duplication
//! across all reconcilers.

use crate::error::ControllerError;
use tracing::{debug, info, warn, error};

/// Trait for NetBox resources that have an ID and URL
pub trait NetBoxResource {
    fn id(&self) -> u64;
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
    _client: &netbox_client::NetBoxClient,
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
/// Returns:
/// - `Ok(Some(resource))` if resource exists
/// - `Ok(None)` if resource was deleted (drift detected)
/// - `Err(e)` if there's an error that should be retried
pub async fn check_existing<FGet, Resource>(
    _client: &netbox_client::NetBoxClient,
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

