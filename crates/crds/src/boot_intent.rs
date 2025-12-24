//! BootIntent CRD
//!
//! Maps MAC addresses to boot profiles.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "BootIntent",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct BootIntentSpec {
    /// MAC address of the machine
    pub mac_address: String,
    
    /// Reference to BootProfile
    pub profile_ref: BootProfileRef,
    
    /// Lifecycle state
    #[serde(default)]
    pub lifecycle: LifecycleState,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BootProfileRef {
    /// Name of the BootProfile
    pub name: String,
    
    /// Namespace (defaults to same namespace as BootIntent)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LifecycleState {
    /// Machine discovered, not yet booted
    #[default]
    Discovered,
    
    /// Machine is installing/booting
    Installing,
    
    /// Machine is installed and running
    Installed,
    
    /// Machine is locked (prevent reinstall)
    Locked,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct BootIntentStatus {
    /// Whether boot intent is configured
    pub configured: bool,
    
    /// Current lifecycle state
    pub lifecycle: LifecycleState,
    
    /// Last reconciliation timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Error message if reconciliation failed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

