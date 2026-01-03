//! NetBoxTenant Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox tenants.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::{NetBoxResourceReference, SecretReference};

/// NetBoxTenantSpec defines the desired state of a NetBox tenant
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxTenant",
    namespaced,
    status = "NetBoxTenantStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxTenantSpec {
    /// Tenant name
    pub name: String,
    
    /// Tenant slug (optional, auto-generated from name if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    
    /// Description of the tenant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Comments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
    
    /// Tenant group reference (references NetBoxTenantGroup CRD, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<NetBoxResourceReference>,
    
    /// Secret reference containing the NetBox API token for this tenant
    /// 
    /// The Secret must contain a key (defaults to "token") with the NetBox API token.
    /// If namespace is not specified, the Secret is expected to be in the same namespace
    /// as the Tenant CRD.
    pub token_secret: SecretReference,
    
    /// Reconcile interval in seconds
    /// 
    /// How often to check for drift between the CRD spec and NetBox, even when no changes occur.
    /// Defaults to 300 seconds (5 minutes) if not specified.
    /// Set to 0 to disable periodic reconciliation (only reconcile on changes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconcile_interval: Option<u64>,
    
    /// Tag references (references NetBoxTag CRDs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<NetBoxResourceReference>>,
    
    /// Enable drift detection (default: true)
    /// 
    /// When enabled, the reconciler will detect and correct any changes made to the resource
    /// in the NetBox UI that differ from the CRD spec. Git is the source of truth.
    /// Set to false to disable drift detection (not recommended for GitOps workflows).
    #[serde(skip_serializing_if = "Option::is_none", default = "default_drift_detection")]
    pub drift_detection: Option<bool>,
}

fn default_drift_detection() -> Option<bool> {
    Some(true)
}

/// NetBoxTenantStatus defines the observed state of a NetBox tenant
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxTenantStatus {
    /// NetBox tenant ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox tenant URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the tenant
    pub state: ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

/// Resource reconciliation state
/// Resource reconciliation state
///
/// Serializes as PascalCase ("Created", "Failed", etc.) but deserializes
/// both PascalCase and lowercase ("created", "failed", etc.) for backward
/// compatibility with existing CRs in the cluster.
///
/// NOTE: The CRD validation schema currently expects lowercase enum values.
/// This is a known issue that needs to be fixed in the CRD schema generation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "PascalCase")]
pub enum ResourceState {
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

