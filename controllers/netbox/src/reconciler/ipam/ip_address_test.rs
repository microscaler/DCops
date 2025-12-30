//! Unit tests for NetBoxIPAddress reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crds::{NetBoxIPAddress, NetBoxTenant, ResourceState};
    use ipnet::IpNet;
    use std::str::FromStr;
    
    /// Helper to set up test data for IP address reconciliation
    fn setup_ip_address_test_data() -> (NetBoxIPAddress, NetBoxTenant) {
        // Create test tenant with status (required dependency)
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        
        // Create test IP address CRD
        let mut ip_address = NetBoxIPAddress {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("test-ip-address".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxIPAddressSpec {
                address: "192.168.1.10/24".to_string(),
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "datacenter-tenant".to_string(),
                    namespace: Some("default".to_string()),
                },
                vrf: None,
                vlan: None,
                status: crds::IPAddressStatus::Active,
                role: None,
                dns_name: None,
                description: Some("Test IP address".to_string()),
                tags: None,
                comments: None,
            },
            status: None,
        };
        
        (ip_address, tenant)
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_address_create() {
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
        let (mut ip_address, tenant) = setup_ip_address_test_data();
        ip_address.status = None; // Clear status to test create path
        
        // Setup: Create reconciler with MockTokenResolver
        let (reconciler, apis, _mock_event_recorder, _mock_secret_fetcher) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store test data in the APIs before reconciliation
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.ip_address_api.store("test-ip-address".to_string(), ip_address.clone());
        
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
        
        // Setup: Mock IP address creation in NetBox
        let ip_net = IpNet::from_str("192.168.1.10/24").unwrap();
        let created_ip = netbox_client::IPAddress {
            id: 42,
            url: "http://test-netbox/api/ipam/ip-addresses/42/".to_string(),
            display: "192.168.1.10/24".to_string(),
            family: 4,
            address: ip_net,
            vrf: None,
            tenant: Some(netbox_client::NestedTenant {
                id: 1,
                url: "http://test-netbox/api/tenancy/tenants/1/".to_string(),
                display: "Data Center Operations".to_string(),
                name: "Data Center Operations".to_string(),
                slug: "datacenter-ops".to_string(),
            }),
            status: netbox_client::IPAddressStatus::Active,
            role: None,
            assigned_object_type: None,
            assigned_object_id: None,
            assigned_object: None,
            nat_inside: None,
            nat_outside: vec![],
            dns_name: String::new(),
            description: "Test IP address".to_string(),
            comments: String::new(),
            tags: vec![],
            custom_fields: serde_json::json!({}),
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_client.add_ip_address(created_ip);
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_ip_address(&ip_address).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.ip_address_api.get("test-ip-address").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.netbox_id, Some(42), "NetBox ID should be 42");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_address_update() {
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
        let (reconciler, apis, _mock_event_recorder) = 
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
        
        // Setup: Create existing IP address in NetBox
        let ip_net = IpNet::from_str("192.168.1.10/24").unwrap();
        let existing_ip = netbox_client::IPAddress {
            id: 42,
            url: "http://test-netbox/api/ipam/ip-addresses/42/".to_string(),
            display: "192.168.1.10/24".to_string(),
            family: 4,
            address: ip_net,
            vrf: None,
            tenant: Some(netbox_client::NestedTenant {
                id: 1,
                url: "http://test-netbox/api/tenancy/tenants/1/".to_string(),
                display: "Data Center Operations".to_string(),
                name: "Data Center Operations".to_string(),
                slug: "datacenter-ops".to_string(),
            }),
            status: netbox_client::IPAddressStatus::Active,
            role: None,
            assigned_object_type: None,
            assigned_object_id: None,
            assigned_object: None,
            nat_inside: None,
            nat_outside: vec![],
            dns_name: String::new(),
            description: "Old description".to_string(),
            comments: String::new(),
            tags: vec![],
            custom_fields: serde_json::json!({}),
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_client.add_ip_address(existing_ip);
        
        // Setup: Create IP address CRD with status (existing resource)
        let mut ip_address = NetBoxIPAddress {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("test-ip-address".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxIPAddressSpec {
                address: "192.168.1.10/24".to_string(),
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "datacenter-tenant".to_string(),
                    namespace: Some("default".to_string()),
                },
                vrf: None,
                vlan: None,
                status: crds::IPAddressStatus::Active,
                role: None,
                dns_name: None,
                description: Some("New description".to_string()), // Changed description
                tags: None,
                comments: None,
            },
            status: Some(crds::NetBoxIPAddressStatus {
                netbox_id: Some(42),
                netbox_url: Some("http://test-netbox/api/ipam/ip-addresses/42/".to_string()),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        };
        apis.ip_address_api.store("test-ip-address".to_string(), ip_address.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_ip_address(&ip_address).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain Created (update successful)
        let updated_crd = apis.ip_address_api.get("test-ip-address").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(42), "NetBox ID should remain 42");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_address_dependency_not_found() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create IP address CRD with missing tenant dependency
        let ip_address = NetBoxIPAddress {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("test-ip-address".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxIPAddressSpec {
                address: "192.168.1.10/24".to_string(),
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "missing-tenant".to_string(), // Tenant doesn't exist
                    namespace: Some("default".to_string()),
                },
                vrf: None,
                vlan: None,
                status: crds::IPAddressStatus::Active,
                role: None,
                dns_name: None,
                description: None,
                tags: None,
                comments: None,
            },
            status: None,
        };
        apis.ip_address_api.store("test-ip-address".to_string(), ip_address.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_ip_address(&ip_address).await;
        
        // Assert: Should fail with dependency not found error
        assert!(result.is_err(), "Reconciliation should fail when tenant is missing");
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_address_invalid_address() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create tenant
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        // Setup: Create IP address CRD with invalid address format
        let ip_address = NetBoxIPAddress {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("test-ip-address".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxIPAddressSpec {
                address: "invalid-ip-address".to_string(), // Invalid format
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "datacenter-tenant".to_string(),
                    namespace: Some("default".to_string()),
                },
                vrf: None,
                vlan: None,
                status: crds::IPAddressStatus::Active,
                role: None,
                dns_name: None,
                description: None,
                tags: None,
                comments: None,
            },
            status: None,
        };
        apis.ip_address_api.store("test-ip-address".to_string(), ip_address.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_ip_address(&ip_address).await;
        
        // Assert: Should fail with invalid input error
        assert!(result.is_err(), "Reconciliation should fail with invalid IP address format");
        if let Err(e) = result {
            assert!(format!("{}", e).contains("Invalid IP address format") || format!("{}", e).contains("Invalid input"), 
                "Error should mention invalid IP address format: {}", e);
        }
    }
}

