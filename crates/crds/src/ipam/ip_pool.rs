//! IPPool Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing IP pools backed by NetBox prefixes.
//! An IPPool represents a pool of IP addresses allocated from a NetBox prefix,
//! with configurable allocation strategies. It serves as the "PersistentVolume"
//! side of a PV/PVC-like IP management pattern.

use crate::references::NetBoxResourceReference;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Allocation strategy for IPPool child prefixes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum AllocationStrategy {
    #[default]
    Sequential,
    Random,
}

/// IPPool status contains the reconciliation state
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct IPPoolStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,

    pub state: IPPoolState,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// State of an IPPool
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "PascalCase")]
pub enum IPPoolState {
    #[default]
    Pending,
    Created,
    Failed,
}

/// IPPool CRD - represents an IP pool backed by a NetBox prefix
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "IPPool",
    plural = "ippools",
    shortname = "ipp",
    status = "IPPoolStatus",
    namespaced
)]
pub struct IPPoolSpec {
    #[schemars(with = "NetBoxResourceReference")]
    pub prefix: NetBoxResourceReference,

    #[serde(default)]
    pub allocation_strategy: AllocationStrategy,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<NetBoxResourceReference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
