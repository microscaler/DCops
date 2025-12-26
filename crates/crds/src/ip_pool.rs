//! IPPool CRD
//!
//! Defines IP address pools (references NetBox prefixes).

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "IPPool",
    namespaced,
    status = "IPPoolStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct IPPoolSpec {
    /// NetBox prefix reference (references NetBoxPrefix CRD)
    /// This is a stable, GitOps-friendly reference that resolves to the NetBox prefix ID
    pub netbox_prefix_ref: NetBoxResourceReference,
    
    /// Pool scope/role (e.g., "control-plane", "worker", "management")
    #[serde(default)]
    pub role: String,
    
    /// Allocation strategy
    #[serde(default)]
    pub allocation_strategy: AllocationStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AllocationStrategy {
    /// Sequential allocation
    #[default]
    Sequential,
    
    /// Random allocation
    Random,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct IPPoolStatus {
    /// Resolved NetBox prefix ID (observed state)
    /// This is set by the controller after resolving the NetBoxPrefix CRD reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_prefix_id: Option<u64>,
    
    /// NetBox prefix URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_prefix_url: Option<String>,
    
    /// Total available IPs in pool
    pub total_ips: u32,
    
    /// Allocated IPs
    pub allocated_ips: u32,
    
    /// Available IPs
    pub available_ips: u32,
    
    /// Last reconciliation timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

