//! BootProfile CRD
//!
//! Defines boot configurations (kernel, initrd, cmdline).

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "BootProfile",
    namespaced,
    status = "BootProfileStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct BootProfileSpec {
    /// Kernel image URL or path
    pub kernel: String,
    
    /// Initrd image URLs or paths
    #[serde(default)]
    pub initrd: Vec<String>,
    
    /// Kernel command-line parameters
    #[serde(default)]
    pub cmdline: String,
    
    /// Boot message to display (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    
    /// Talos Image Factory schematic ID (for Raspberry Pi custom images)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schematic_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct BootProfileStatus {
    /// Whether the boot profile is ready
    pub ready: bool,
    
    /// Last reconciliation timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

