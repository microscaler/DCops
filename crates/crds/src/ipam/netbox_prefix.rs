//! NetBoxPrefix Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox IPAM prefixes.
//! This allows GitOps-style management of NetBox prefixes.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxPrefixSpec defines the desired state of a NetBox prefix
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxPrefix",
    namespaced,
    status = "NetBoxPrefixStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxPrefixSpec {
    /// Prefix CIDR (e.g., "192.168.1.0/24")
    pub prefix: String,
    
    /// Description of the prefix
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Site reference (references NetBoxSite CRD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<NetBoxResourceReference>,
    
    /// Tenant reference (references NetBoxTenant CRD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<NetBoxResourceReference>,
    
    /// Aggregate reference (references NetBoxAggregate CRD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregate: Option<NetBoxResourceReference>,
    
    /// VLAN reference (references NetBoxVLAN CRD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan: Option<NetBoxResourceReference>,
    
    /// Status in NetBox (active, reserved, deprecated, container)
    #[serde(default = "default_status")]
    pub status: PrefixStatus,
    
    /// Role reference (references NetBoxRole CRD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<NetBoxResourceReference>,
    
    /// Tag references (references NetBoxTag CRDs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<NetBoxResourceReference>>,
    
    /// Comments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
}

fn default_status() -> PrefixStatus {
    PrefixStatus::Active
}

/// Prefix status in NetBox
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum PrefixStatus {
    Active,
    Reserved,
    Deprecated,
    Container,
}

/// NetBoxPrefixStatus defines the observed state of a NetBox prefix
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxPrefixStatus {
    /// NetBox prefix ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox prefix URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the prefix
    pub state: PrefixState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

/// Prefix reconciliation state
/// Prefix reconciliation state
///
/// Serializes as PascalCase ("Created", "Failed", etc.) but deserializes
/// both PascalCase and lowercase ("created", "failed", etc.) for backward
/// compatibility with existing CRs in the cluster.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "PascalCase")]
pub enum PrefixState {
    #[default]
    #[serde(alias = "pending")] // Backward compatibility: accept lowercase
    Pending,
    #[serde(alias = "created")] // Backward compatibility: accept lowercase
    Created,
    #[serde(alias = "updated")] // Backward compatibility: accept lowercase
    Updated,
    #[serde(alias = "failed")] // Backward compatibility: accept lowercase
    Failed,
}

