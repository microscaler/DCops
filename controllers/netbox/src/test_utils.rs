//! Test utilities for unit testing reconcilers
//!
//! This module provides helpers for creating test data and setting up test scenarios.
//!
//! ## Kubernetes API Mocking
//!
//! Reconciler tests require mocking `kube::Api<T>` instances. See `docs/KUBE_API_MOCKING.md`
//! for the strategy and implementation plan. The recommended approach is to use `tower-test`
//! to create a mock HTTP service that emulates the Kubernetes API server.
//!
//! The mocking infrastructure is organized in the `kube_mock` submodule:
//! - `kube_mock::store`: In-memory resource store
//! - `kube_mock::service`: Mock HTTP service using tower-test
//! - `kube_mock::client`: Mock kube::Client creation
//! - `kube_mock::helpers`: Utility functions for common scenarios

#[cfg(test)]
mod kube_mock;

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
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxPrefix".to_string(),
                name: prefix_ref_name.to_string(),
                namespace: prefix_ref_namespace.map(|s| s.to_string()),
            },
            role: String::new(),
            allocation_strategy: crds::AllocationStrategy::Sequential,
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
            description: None,
            site: None,
            tenant: crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTenant".to_string(),
                name: "datacenter-tenant".to_string(),
                namespace: Some(namespace.to_string()),
            },
            aggregate: None,
            vlan: None,
            status: crds::PrefixStatus::Active,
            role: None,
            tags: None,
            comments: None,
        },
        status: Some(crds::NetBoxPrefixStatus {
            netbox_id: Some(netbox_id),
            netbox_url,
            state: crds::PrefixState::Created,
            error: None,
            last_reconciled: None,
        }),
    }
}

/// Helper to create a test Reconciler with a mock NetBoxClient and mock Kubernetes APIs.
/// 
/// This creates a reconciler with all APIs mocked, enabling true unit testing.
#[cfg(test)]
// Note: This function is incomplete - it needs TokenResolver which requires kube::Client
// For now, tests that need full reconciler setup should be marked #[ignore]
// until proper mocking infrastructure is in place
#[allow(dead_code)]
pub fn create_test_reconciler(
    _mock_client: MockNetBoxClient,
) {
    // TODO: This function needs to be updated to create a TokenResolver
    // which requires a kube::Client. For now, tests using this should be #[ignore]
    // use crate::kube_api_trait::mock::MockKubeApi;
    // use crate::token_resolver::TokenResolver;
    // use std::sync::Arc;
    // 
    // let token_resolver = Arc::new(TokenResolver::new(kube_client, "http://test-netbox".to_string()));
    // 
    // Reconciler::new(
    //     token_resolver,
    //     // IPAM APIs (21 total including netbox_rir_api)
    //     MockKubeApi::new(), // netbox_prefix_api
    //     MockKubeApi::new(), // netbox_role_api
    //     MockKubeApi::new(), // netbox_tag_api
    //     MockKubeApi::new(), // netbox_aggregate_api
    //     MockKubeApi::new(), // netbox_vlan_api
    //     MockKubeApi::new(), // netbox_rir_api
    //     // Tenancy APIs
    //     MockKubeApi::new(), // netbox_tenant_api
    //     // DCIM APIs
    //     MockKubeApi::new(), // netbox_site_api
    //     MockKubeApi::new(), // netbox_device_role_api
    //     MockKubeApi::new(), // netbox_manufacturer_api
    //     MockKubeApi::new(), // netbox_platform_api
    //     MockKubeApi::new(), // netbox_device_type_api
    //     MockKubeApi::new(), // netbox_device_api
    //     MockKubeApi::new(), // netbox_interface_api
    //     MockKubeApi::new(), // netbox_mac_address_api
    //     MockKubeApi::new(), // netbox_region_api
    //     MockKubeApi::new(), // netbox_site_group_api
    //     MockKubeApi::new(), // netbox_location_api
    //     // Custom CRDs
    //     MockKubeApi::new(), // ip_pool_api
    //     MockKubeApi::new(), // ip_claim_api
    // )
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

/// Helper to create test IPClaim CRD
#[cfg(test)]
pub fn create_test_ip_claim(
    name: &str,
    namespace: &str,
    pool_ref_name: &str,
    pool_ref_namespace: Option<&str>,
    device_name: &str,
    interface: Option<&str>,
    preferred_ip: Option<&str>,
) -> IPClaim {
    IPClaim {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            namespace: Some(namespace.to_string()),
            ..Default::default()
        },
        spec: crds::IPClaimSpec {
            pool_ref: crds::IPPoolRef {
                name: pool_ref_name.to_string(),
                namespace: pool_ref_namespace.map(|s| s.to_string()),
            },
            device_ref: crds::DeviceRef {
                name: device_name.to_string(),
                interface: interface.map(|s| s.to_string()),
            },
            preferred_ip: preferred_ip.map(|s| s.to_string()),
        },
        status: None,
    }
}

/// Helper to create test NetBoxSite CRD
#[cfg(test)]
pub fn create_test_netbox_site(
    name: &str,
    namespace: &str,
    netbox_id: Option<u64>,
    netbox_url: Option<String>,
) -> NetBoxSite {
    NetBoxSite {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            namespace: Some(namespace.to_string()),
            ..Default::default()
        },
        spec: crds::NetBoxSiteSpec {
            name: name.to_string(),
            slug: None,
            description: None,
            physical_address: None,
            shipping_address: None,
            latitude: None,
            longitude: None,
            tenant: crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTenant".to_string(),
                name: "datacenter-tenant".to_string(),
                namespace: Some(namespace.to_string()),
            },
            region: None,
            site_group: None,
            status: crds::SiteStatus::Active,
            facility: None,
            time_zone: None,
            comments: None,
        },
        status: netbox_id.map(|id| crds::NetBoxSiteStatus {
            netbox_id: Some(id),
            netbox_url,
            state: crds::ResourceState::Created,
            error: None,
            last_reconciled: None,
        }),
    }
}

/// Helper to create test NetBoxTenant CRD
#[cfg(test)]
pub fn create_test_netbox_tenant(
    name: &str,
    namespace: &str,
    netbox_id: Option<u64>,
    netbox_url: Option<String>,
) -> NetBoxTenant {
    NetBoxTenant {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            namespace: Some(namespace.to_string()),
            ..Default::default()
        },
        spec: crds::NetBoxTenantSpec {
            name: name.to_string(),
            slug: None,
            group: None,
            description: None,
            comments: None,
            reconcile_interval: None,
            token_secret: crds::SecretReference {
                name: format!("netbox-token-{}", name),
                namespace: None,
                key: None,
            },
        },
        status: netbox_id.map(|id| crds::NetBoxTenantStatus {
            netbox_id: Some(id),
            netbox_url,
            state: crds::ResourceState::Created,
            error: None,
            last_reconciled: None,
        }),
    }
}

/// Helper to create test NetBoxDevice CRD with status
#[cfg(test)]
pub fn create_test_netbox_device(
    name: &str,
    namespace: &str,
    device_type_name: &str,
    device_role_name: &str,
    site_name: &str,
    netbox_id: Option<u64>,
    netbox_url: Option<String>,
) -> NetBoxDevice {
    use crds::DeviceStatus;
    NetBoxDevice {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            namespace: Some(namespace.to_string()),
            ..Default::default()
        },
        spec: crds::NetBoxDeviceSpec {
            name: Some(name.to_string()),
            device_type: crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxDeviceType".to_string(),
                name: device_type_name.to_string(),
                namespace: Some(namespace.to_string()),
            },
            device_role: crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxDeviceRole".to_string(),
                name: device_role_name.to_string(),
                namespace: Some(namespace.to_string()),
            },
            site: crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxSite".to_string(),
                name: site_name.to_string(),
                namespace: Some(namespace.to_string()),
            },
            location: None,
            tenant: crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTenant".to_string(),
                name: "datacenter-tenant".to_string(),
                namespace: Some(namespace.to_string()),
            },
            platform: None,
            serial: None,
            asset_tag: None,
            status: DeviceStatus::Active,
            primary_ip4: None,
            primary_ip6: None,
            description: None,
            comments: None,
        },
        status: netbox_id.map(|id| crds::NetBoxDeviceStatus {
            netbox_id: Some(id),
            netbox_url: Some(netbox_url.unwrap_or_else(|| format!("http://netbox/api/dcim/devices/{}/", id))),
            state: crds::ResourceState::Created,
            error: None,
            last_reconciled: None,
        }),
    }
}

