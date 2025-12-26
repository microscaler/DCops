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

