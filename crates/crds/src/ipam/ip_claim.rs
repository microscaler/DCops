//! IPClaim Custom Resource Definition
//!
//! Defines a Kubernetes CRD for claiming an IP address from an IPPool.
//! This is the "PersistentVolumeClaim" side of the PV/PVC-like IP management
//! pattern. It allocates an IP from the IPPool's child prefix in NetBox
//! and optionally assigns it to a NetBoxDevice interface.

use crate::references::NetBoxResourceReference;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// IPClaim status contains the reconciliation state
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct IPClaimStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,

    /// The allocated IP address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,

    pub state: IPClaimState,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// State of an IPClaim
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "PascalCase")]
pub enum IPClaimState {
    #[default]
    Pending,
    Created,
    Failed,
}

/// IPClaim CRD - claims an IP address from an IPPool
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "IPClaim",
    plural = "ipclaims",
    shortname = "ipc",
    status = IPClaimStatus,
    namespaced
)]
pub struct IPClaimSpec {
    /// Reference to the IPPool to claim an IP from
    #[schemars(with = "NetBoxResourceReference")]
    pub pool: NetBoxResourceReference,

    /// Optional preferred IP address to use (in CIDR notation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_ip: Option<String>,

    /// Optional reference to a NetBoxDevice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<NetBoxResourceReference>,

    /// Optional reference to a NetBoxInterface
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<NetBoxResourceReference>,

    /// Optional description for the IP address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
