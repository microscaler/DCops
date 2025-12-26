//! Test utilities for unit testing reconcilers
//!
//! This module provides helpers for creating test data and setting up test scenarios.

#[cfg(test)]
use crate::reconciler::Reconciler;
#[cfg(test)]
use crds::*;
#[cfg(test)]
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
#[cfg(test)]
use kube::Api;
#[cfg(test)]
use kube::Client;
#[cfg(test)]
use netbox_client::{MockNetBoxClient, NetBoxClientTrait};
#[cfg(test)]
use std::collections::HashMap;

/// Helper to create test IPPool CRD
#[cfg(test)]
pub fn create_test_ip_pool(
    name: &str,
    namespace: &str,
    prefix_ref_name: &str,
    prefix_ref_namespace: Option<&str>,
) -> IPPool {
    IPPool {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            namespace: Some(namespace.to_string()),
            ..Default::default()
        },
        spec: crds::IPPoolSpec {
            netbox_prefix_ref: crds::NetBoxResourceReference {
                kind: "NetBoxPrefix".to_string(),
                name: prefix_ref_name.to_string(),
                namespace: prefix_ref_namespace.map(|s| s.to_string()),
            },
        },
        status: None,
    }
}

/// Helper to create test NetBoxPrefix CRD with status
#[cfg(test)]
pub fn create_test_netbox_prefix(
    name: &str,
    namespace: &str,
    netbox_id: u64,
    netbox_url: Option<String>,
) -> NetBoxPrefix {
    NetBoxPrefix {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            namespace: Some(namespace.to_string()),
            ..Default::default()
        },
        spec: crds::NetBoxPrefixSpec {
            prefix: "192.168.1.0/24".to_string(),
            ..Default::default()
        },
        status: Some(crds::NetBoxPrefixStatus {
            netbox_id: Some(netbox_id),
            netbox_url,
            state: crds::PrefixState::Created,
            error: None,
        }),
    }
}

/// Helper to create a test Reconciler with a mock NetBoxClient
/// 
/// This creates a reconciler with all required Kubernetes API clients.
/// For unit tests, you'll need to mock the Kubernetes API calls separately
/// (e.g., using kube's test framework or a custom mock).
#[cfg(test)]
pub fn create_test_reconciler(
    mock_client: MockNetBoxClient,
    kube_client: Client,
    namespace: &str,
) -> Reconciler {
    use kube::Api;
    
    Reconciler::new(
        Box::new(mock_client),
        // IPAM APIs
        Api::namespaced(kube_client.clone(), namespace),
        Api::namespaced(kube_client.clone(), namespace),
        Api::namespaced(kube_client.clone(), namespace),
        Api::namespaced(kube_client.clone(), namespace),
        Api::namespaced(kube_client.clone(), namespace),
        // Tenancy APIs
        Api::namespaced(kube_client.clone(), namespace),
        // DCIM APIs
        Api::namespaced(kube_client.clone(), namespace),
        Api::namespaced(kube_client.clone(), namespace),
        Api::namespaced(kube_client.clone(), namespace),
        Api::namespaced(kube_client.clone(), namespace),
        Api::namespaced(kube_client.clone(), namespace),
        Api::namespaced(kube_client.clone(), namespace),
        Api::namespaced(kube_client.clone(), namespace),
        Api::namespaced(kube_client.clone(), namespace),
        Api::namespaced(kube_client.clone(), namespace),
        Api::namespaced(kube_client.clone(), namespace),
        Api::namespaced(kube_client.clone(), namespace),
        // Custom CRDs
        Api::namespaced(kube_client.clone(), namespace),
        Api::namespaced(kube_client.clone(), namespace),
    )
}

/// Helper to create a test Prefix with all required fields
#[cfg(test)]
pub fn create_test_prefix(
    id: u64,
    prefix: &str,
    base_url: &str,
) -> netbox_client::Prefix {
    use netbox_client::{Prefix, PrefixStatus};
    use chrono::Utc;
    
    Prefix {
        id,
        url: format!("{}/api/ipam/prefixes/{}/", base_url, id),
        display: prefix.to_string(),
        family: if prefix.contains(':') { 6 } else { 4 },
        prefix: prefix.to_string(),
        vrf: None,
        tenant: None,
        vlan: None,
        status: PrefixStatus::Active,
        role: None,
        is_pool: false,
        mark_utilized: false,
        description: String::new(),
        comments: String::new(),
        tags: vec![],
        custom_fields: serde_json::json!({}),
        created: Utc::now().to_rfc3339(),
        last_updated: Utc::now().to_rfc3339(),
        children: 0,
        _depth: 0,
    }
}

