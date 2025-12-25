//! IPClaim CRD
//!
//! Requests an IP allocation for a device/interface.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "IPClaim",
    namespaced,
    status = "IPClaimStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct IPClaimSpec {
    /// Reference to IPPool
    pub pool_ref: IPPoolRef,
    
    /// Device/interface reference
    pub device_ref: DeviceRef,
    
    /// Preferred IP (hint, not guarantee)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_ip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IPPoolRef {
    /// Name of the IPPool
    pub name: String,
    
    /// Namespace (defaults to same namespace as IPClaim)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRef {
    /// Device name or identifier
    pub name: String,
    
    /// Interface name (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,
    
    /// NetBox device reference (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub netbox_device_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct IPClaimStatus {
    /// Allocated IP address
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    
    /// Allocation state
    pub state: AllocationState,
    
    /// NetBox IPAddress object reference
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub netbox_ip_ref: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Error message if allocation failed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
/// IP allocation state
///
/// Serializes as PascalCase ("Allocated", "Failed", etc.) but deserializes
/// both PascalCase and lowercase ("allocated", "failed", etc.) for backward
/// compatibility with existing CRs in the cluster.
#[serde(rename_all = "PascalCase")]
pub enum AllocationState {
    /// Allocation pending
    #[default]
    #[serde(alias = "pending")] // Backward compatibility: accept lowercase
    Pending,
    
    /// IP allocated
    #[serde(alias = "allocated")] // Backward compatibility: accept lowercase
    Allocated,
    
    /// Allocation failed
    #[serde(alias = "failed")] // Backward compatibility: accept lowercase
    Failed,
}

