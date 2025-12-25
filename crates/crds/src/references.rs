//! Kubernetes object references for NetBox CRDs
//!
//! Provides standard Kubernetes-style object references for cross-resource references.
//! Follows Kubernetes TypedLocalObjectReference pattern with apiGroup, kind, name, and optional namespace.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Kubernetes-compliant resource reference for NetBox CRDs
///
/// This follows the Kubernetes `TypedLocalObjectReference` pattern, which includes:
/// - `apiGroup`: The API group of the referenced resource (e.g., "dcops.microscaler.io")
/// - `kind`: The kind of the referenced resource (e.g., "NetBoxSite")
/// - `name`: The name of the referenced resource (required)
/// - `namespace`: The namespace of the referenced resource (optional, defaults to same namespace)
///
/// This enables Kubernetes to validate reference types and provides clear documentation
/// of what resource type is expected.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxResourceReference {
    /// API group of the referenced resource (e.g., "dcops.microscaler.io")
    pub api_group: String,
    
    /// Kind of the referenced resource (e.g., "NetBoxSite", "NetBoxTenant")
    pub kind: String,
    
    /// Name of the referenced NetBox CRD resource
    pub name: String,
    
    /// Namespace of the referenced resource (defaults to same namespace as the referencing resource)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

impl NetBoxResourceReference {
    /// Create a new reference with apiGroup, kind, and name (same namespace)
    pub fn new(api_group: String, kind: String, name: String) -> Self {
        Self {
            api_group,
            kind,
            name,
            namespace: None,
        }
    }
    
    /// Create a new reference with apiGroup, kind, name, and namespace
    pub fn with_namespace(api_group: String, kind: String, name: String, namespace: String) -> Self {
        Self {
            api_group,
            kind,
            name,
            namespace: Some(namespace),
        }
    }
    
    /// Helper to create a reference for a NetBox CRD in the same API group
    pub fn netbox(kind: &str, name: String) -> Self {
        Self {
            api_group: "dcops.microscaler.io".to_string(),
            kind: kind.to_string(),
            name,
            namespace: None,
        }
    }
}


