//! Unit tests for NetBoxIPRange reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crds::{NetBoxIPRange, NetBoxTenant, ResourceState};
    use ipnet::IpNet;
    use std::str::FromStr;
    
    /// Helper to set up test data for IP range reconciliation
    fn setup_ip_range_test_data() -> (NetBoxIPRange, NetBoxTenant) {
        // Create test tenant with status (required dependency)
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        
        // Create test IP range CRD
        let mut ip_range = NetBoxIPRange {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("test-ip-range".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxIPRangeSpec {
                start_address: "192.168.1.100/24".to_string(),
                end_address: "192.168.1.200/24".to_string(),
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "datacenter-tenant".to_string(),
                    namespace: Some("default".to_string()),
                },
                vrf: None,
                status: crds::IPRangeStatus::Active,
                role: None,
                description: Some("Test IP range".to_string()),
                mark_utilized: false,
                mark_populated: false,
                tags: None,
            },
            status: None,
        };
        
        (ip_range, tenant)
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_range_create() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver with MockNetBoxClient
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        // Setup: Add secret for tenant
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Get MockNetBoxClient to set up test data
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Create test data
        let (mut ip_range, tenant) = setup_ip_range_test_data();
        ip_range.status = None; // Clear status to test create path
        
        // Setup: Create reconciler with MockTokenResolver
        let (reconciler, apis, _mock_event_recorder) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store test data in the APIs before reconciliation
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.ip_range_api.store("test-ip-range".to_string(), ip_range.clone());
        
        // Setup: Add tenant to mock NetBox
        use netbox_client::Tenant;
        let netbox_tenant = Tenant {
            id: 1,
            url: "http://test-netbox/api/tenancy/tenants/1/".to_string(),
            display: "Data Center Operations".to_string(),
            name: "Data Center Operations".to_string(),
            slug: "datacenter-ops".to_string(),
            description: None,
            comments: None,
            group: None,
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_client.add_tenant(netbox_tenant);
        
        // Setup: Mock IP range creation in NetBox
        let start_ip_net = IpNet::from_str("192.168.1.100/24").unwrap();
        let end_ip_net = IpNet::from_str("192.168.1.200/24").unwrap();
        let created_range = netbox_client::IPRange {
            id: 42,
            url: "http://test-netbox/api/ipam/ip-ranges/42/".to_string(),
            display: "192.168.1.100-192.168.1.200".to_string(),
            family: 4,
            start_address: start_ip_net,
            end_address: end_ip_net,
            vrf: None,
            tenant: Some(netbox_client::NestedTenant {
                id: 1,
                url: "http://test-netbox/api/tenancy/tenants/1/".to_string(),
                display: "Data Center Operations".to_string(),
                name: "Data Center Operations".to_string(),
                slug: "datacenter-ops".to_string(),
            }),
            status: netbox_client::IPRangeStatus::Active,
            role: None,
            description: "Test IP range".to_string(),
            mark_utilized: false,
            mark_populated: false,
            tags: vec![],
            custom_fields: serde_json::json!({}),
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_client.add_ip_range(created_range.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_ip_range(&ip_range).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.ip_range_api.get("test-ip-range").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.netbox_id, Some(42), "NetBox ID should be 42");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_range_update() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Get MockNetBoxClient
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder, _mock_secret_fetcher) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create tenant
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        // Setup: Add tenant to mock NetBox
        use netbox_client::Tenant;
        let netbox_tenant = Tenant {
            id: 1,
            url: "http://test-netbox/api/tenancy/tenants/1/".to_string(),
            display: "Data Center Operations".to_string(),
            name: "Data Center Operations".to_string(),
            slug: "datacenter-ops".to_string(),
            description: None,
            comments: None,
            group: None,
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_client.add_tenant(netbox_tenant);
        
        // Setup: Create existing IP range in NetBox
        let start_ip_net = IpNet::from_str("192.168.1.100/24").unwrap();
        let end_ip_net = IpNet::from_str("192.168.1.200/24").unwrap();
        let existing_range: netbox_client::IPRange = netbox_client::IPRange {
            id: 42,
            url: "http://test-netbox/api/ipam/ip-ranges/42/".to_string(),
            display: "192.168.1.100-192.168.1.200".to_string(),
            family: 4,
            start_address: start_ip_net,
            end_address: end_ip_net,
            vrf: None,
            tenant: Some(netbox_client::NestedTenant {
                id: 1,
                url: "http://test-netbox/api/tenancy/tenants/1/".to_string(),
                display: "Data Center Operations".to_string(),
                name: "Data Center Operations".to_string(),
                slug: "datacenter-ops".to_string(),
            }),
            status: netbox_client::IPRangeStatus::Active,
            role: None,
            description: "Old description".to_string(),
            mark_utilized: false,
            mark_populated: false,
            tags: vec![],
            custom_fields: serde_json::json!({}),
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_client.add_ip_range(existing_range);
        
        // Setup: Create IP range CRD with status (existing resource)
        let mut ip_range = NetBoxIPRange {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("test-ip-range".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxIPRangeSpec {
                start_address: "192.168.1.100/24".to_string(),
                end_address: "192.168.1.200/24".to_string(),
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "datacenter-tenant".to_string(),
                    namespace: Some("default".to_string()),
                },
                vrf: None,
                status: crds::IPRangeStatus::Active,
                role: None,
                description: Some("New description".to_string()), // Changed description
                mark_utilized: true, // Changed mark_utilized
                mark_populated: true, // Changed mark_populated
                tags: None,
            },
            status: Some(crds::NetBoxIPRangeStatus {
                netbox_id: Some(42),
                netbox_url: Some("http://test-netbox/api/ipam/ip-ranges/42/".to_string()),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        };
        apis.ip_range_api.store("test-ip-range".to_string(), ip_range.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_ip_range(&ip_range).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain Created (update successful)
        let updated_crd = apis.ip_range_api.get("test-ip-range").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(42), "NetBox ID should remain 42");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_range_dependency_not_found() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder, _mock_secret_fetcher) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create IP range CRD with missing tenant dependency
        let ip_range = NetBoxIPRange {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("test-ip-range".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxIPRangeSpec {
                start_address: "192.168.1.100/24".to_string(),
                end_address: "192.168.1.200/24".to_string(),
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "missing-tenant".to_string(), // Tenant doesn't exist
                    namespace: Some("default".to_string()),
                },
                vrf: None,
                status: crds::IPRangeStatus::Active,
                role: None,
                description: None,
                mark_utilized: false,
                mark_populated: false,
                tags: None,
            },
            status: None,
        };
        apis.ip_range_api.store("test-ip-range".to_string(), ip_range.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_ip_range(&ip_range).await;
        
        // Assert: Should fail with dependency not found error
        assert!(result.is_err(), "Reconciliation should fail when tenant is missing");
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_range_invalid_address() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder, _mock_secret_fetcher) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create tenant
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        // Setup: Create IP range CRD with invalid address format
        let ip_range = NetBoxIPRange {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("test-ip-range".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxIPRangeSpec {
                start_address: "invalid-ip-address".to_string(), // Invalid format
                end_address: "192.168.1.200/24".to_string(),
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "datacenter-tenant".to_string(),
                    namespace: Some("default".to_string()),
                },
                vrf: None,
                status: crds::IPRangeStatus::Active,
                role: None,
                description: None,
                mark_utilized: false,
                mark_populated: false,
                tags: None,
            },
            status: None,
        };
        apis.ip_range_api.store("test-ip-range".to_string(), ip_range.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_ip_range(&ip_range).await;
        
        // Assert: Should fail with invalid input error
        assert!(result.is_err(), "Reconciliation should fail with invalid IP address format");
        if let Err(e) = result {
            assert!(format!("{}", e).contains("Invalid") || format!("{}", e).contains("invalid"), 
                "Error should mention invalid IP address format: {}", e);
        }
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_range_family_mismatch() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder, _mock_secret_fetcher) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create tenant
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        // Setup: Create IP range CRD with mismatched IP families (IPv4 start, IPv6 end)
        let ip_range = NetBoxIPRange {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("test-ip-range".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxIPRangeSpec {
                start_address: "192.168.1.100/24".to_string(), // IPv4
                end_address: "2001:db8::200/64".to_string(), // IPv6 - mismatch!
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "datacenter-tenant".to_string(),
                    namespace: Some("default".to_string()),
                },
                vrf: None,
                status: crds::IPRangeStatus::Active,
                role: None,
                description: None,
                mark_utilized: false,
                mark_populated: false,
                tags: None,
            },
            status: None,
        };
        apis.ip_range_api.store("test-ip-range".to_string(), ip_range.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_ip_range(&ip_range).await;
        
        // Assert: Should fail with family mismatch error
        assert!(result.is_err(), "Reconciliation should fail with IP family mismatch");
        if let Err(e) = result {
            assert!(format!("{}", e).contains("family") || format!("{}", e).contains("mismatch") || format!("{}", e).contains("Invalid"), 
                "Error should mention IP family mismatch: {}", e);
        }
    }
}

