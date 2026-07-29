//! API types for Pixiecore-compatible boot configuration.

use serde::{Deserialize, Serialize};

/// Boot configuration returned by `GET /v1/boot/:mac` (Pixiecore API shape).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BootConfig {
    /// Kernel image URL.
    pub kernel: String,
    /// Initrd image URLs.
    #[serde(default)]
    pub initrd: Vec<String>,
    /// Kernel command line (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmdline: Option<String>,
    /// Message shown during boot (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
