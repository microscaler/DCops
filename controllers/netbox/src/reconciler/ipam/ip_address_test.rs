//! Unit tests for NetBoxIPAddress reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crds::{NetBoxIPAddress, NetBoxIPRange, NetBoxTenant, ResourceState};
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
                address: Some("192.168.1.10/24".to_string()),
                ip_range: None,
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
                mac_address: None,
                interface: None,
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
        let (reconciler, apis, _mock_event_recorder) = 
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
            tags: vec![],
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
            tags: vec![],
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
                address: Some("192.168.1.10/24".to_string()),
                ip_range: None,
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
                mac_address: None,
                interface: None,
            },
            status: Some(crds::NetBoxIPAddressStatus {
                address: Some("192.168.1.10/24".to_string()),
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
                address: Some("192.168.1.10/24".to_string()),
                ip_range: None,
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
                mac_address: None,
                interface: None,
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
                address: Some("invalid-ip-address".to_string()), // Invalid format
                ip_range: None,
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
                mac_address: None,
                interface: None,
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
    
    // ========== Tag Tests ==========
    
    #[tokio::test]
    async fn test_reconcile_ip_address_with_tags_create() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::test_utils::{create_test_netbox_tag, create_test_nested_tag};
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver with MockNetBoxClient
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Create test data
        let (mut ip_address, tenant) = setup_ip_address_test_data();
        ip_address.status = None;
        ip_address.spec.tags = Some(vec![
            crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTag".to_string(),
                name: "production".to_string(),
                namespace: Some("default".to_string()),
            },
            crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTag".to_string(),
                name: "web-tier".to_string(),
                namespace: Some("default".to_string()),
            },
        ]);
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store test data
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.ip_address_api.store("test-ip-address".to_string(), ip_address.clone());
        
        // Setup: Create tag CRDs with status
        let tag1 = create_test_netbox_tag("production", "default", Some(10));
        let tag2 = create_test_netbox_tag("web-tier", "default", Some(11));
        apis.tag_api.store("production".to_string(), tag1);
        apis.tag_api.store("web-tier".to_string(), tag2);
        
        // Setup: Add tags to mock NetBox
        mock_client.add_tag(netbox_client::Tag {
            id: 10,
            url: format!("{}/api/extras/tags/10/", netbox_url),
            display: "production".to_string(),
            name: "production".to_string(),
            slug: "production".to_string(),
            color: "ff0000".to_string(),
            description: None,
            comments: None,
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });
        mock_client.add_tag(netbox_client::Tag {
            id: 11,
            url: format!("{}/api/extras/tags/11/", netbox_url),
            display: "web-tier".to_string(),
            name: "web-tier".to_string(),
            slug: "web-tier".to_string(),
            color: "00ff00".to_string(),
            description: None,
            comments: None,
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });
        
        // Setup: Add tenant to mock NetBox
        use netbox_client::Tenant;
        let netbox_tenant = Tenant {
            id: 1,
            url: format!("{}/api/tenancy/tenants/1/", netbox_url),
            display: "Data Center Operations".to_string(),
            name: "Data Center Operations".to_string(),
            slug: "datacenter-ops".to_string(),
            description: None,
            comments: None,
            group: None,
            tags: vec![],
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_client.add_tenant(netbox_tenant);
        
        // Setup: Mock IP address creation with tags
        let ip_net = IpNet::from_str("192.168.1.10/24").unwrap();
        let created_ip = netbox_client::IPAddress {
            id: 42,
            url: format!("{}/api/ipam/ip-addresses/42/", netbox_url),
            display: "192.168.1.10/24".to_string(),
            family: 4,
            address: ip_net,
            vrf: None,
            tenant: Some(netbox_client::NestedTenant {
                id: 1,
                url: format!("{}/api/tenancy/tenants/1/", netbox_url),
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
            dns_name: None,
            description: "Test IP address".to_string(),
            comments: String::new(),
            tags: vec![
                create_test_nested_tag(10, "production", &netbox_url),
                create_test_nested_tag(11, "web-tier", &netbox_url),
            ],
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
        let updated_crd = apis.ip_address_api.as_ref().get("test-ip-address").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(42), "NetBox ID should be 42");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
        
        // Assert: Tags should be included in the create request
        // (We can't directly verify this, but if reconciliation succeeds, tags were resolved)
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_address_with_tags_update() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::test_utils::{create_test_netbox_tag, create_test_nested_tag};
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Create IP address with existing status and different tags
        let (mut ip_address, tenant) = setup_ip_address_test_data();
        ip_address.status = Some(crds::NetBoxIPAddressStatus {
            netbox_id: Some(42),
            netbox_url: Some(format!("{}/api/ipam/ip-addresses/42/", netbox_url)),
            address: None,
            state: ResourceState::Created,
            error: None,
            last_reconciled: None,
        });
        ip_address.spec.tags = Some(vec![
            crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTag".to_string(),
                name: "production".to_string(),
                namespace: Some("default".to_string()),
            },
        ]);
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store test data
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.ip_address_api.store("test-ip-address".to_string(), ip_address.clone());
        
        // Setup: Create tag CRD
        let tag = create_test_netbox_tag("production", "default", Some(10));
        apis.tag_api.store("production".to_string(), tag);
        
        // Setup: Add tag to mock NetBox
        mock_client.add_tag(netbox_client::Tag {
            id: 10,
            url: format!("{}/api/extras/tags/10/", netbox_url),
            display: "production".to_string(),
            name: "production".to_string(),
            slug: "production".to_string(),
            color: "ff0000".to_string(),
            description: None,
            comments: None,
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });
        
        // Setup: Add tenant to mock NetBox
        use netbox_client::Tenant;
        let netbox_tenant = Tenant {
            id: 1,
            url: format!("{}/api/tenancy/tenants/1/", netbox_url),
            display: "Data Center Operations".to_string(),
            name: "Data Center Operations".to_string(),
            slug: "datacenter-ops".to_string(),
            description: None,
            comments: None,
            group: None,
            tags: vec![],
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_client.add_tenant(netbox_tenant);
        
        // Setup: Mock existing IP address with different tags
        let ip_net = IpNet::from_str("192.168.1.10/24").unwrap();
        let existing_ip = netbox_client::IPAddress {
            id: 42,
            url: format!("{}/api/ipam/ip-addresses/42/", netbox_url),
            display: "192.168.1.10/24".to_string(),
            family: 4,
            address: ip_net,
            vrf: None,
            tenant: Some(netbox_client::NestedTenant {
                id: 1,
                url: format!("{}/api/tenancy/tenants/1/", netbox_url),
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
            dns_name: None,
            description: "Test IP address".to_string(),
            comments: String::new(),
            tags: vec![create_test_nested_tag(20, "old-tag", &netbox_url)], // Different tag
            custom_fields: serde_json::json!({}),
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_client.add_ip_address(existing_ip);
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_ip_address(&ip_address).await;
        
        // Assert: Should succeed (tags differ, so update should be triggered)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_address_with_missing_tags() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Create IP address with tags that don't exist
        let (mut ip_address, tenant) = setup_ip_address_test_data();
        ip_address.status = None;
        ip_address.spec.tags = Some(vec![
            crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTag".to_string(),
                name: "non-existent-tag".to_string(),
                namespace: Some("default".to_string()),
            },
        ]);
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store test data (no tag CRD, tag doesn't exist in NetBox)
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.ip_address_api.store("test-ip-address".to_string(), ip_address.clone());
        
        // Setup: Add tenant to mock NetBox
        use netbox_client::Tenant;
        let netbox_tenant = Tenant {
            id: 1,
            url: format!("{}/api/tenancy/tenants/1/", netbox_url),
            display: "Data Center Operations".to_string(),
            name: "Data Center Operations".to_string(),
            slug: "datacenter-ops".to_string(),
            description: None,
            comments: None,
            group: None,
            tags: vec![],
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_client.add_tenant(netbox_tenant);
        
        // Setup: Mock IP address creation (tags will be skipped since they don't exist)
        let ip_net = IpNet::from_str("192.168.1.10/24").unwrap();
        let created_ip = netbox_client::IPAddress {
            id: 42,
            url: format!("{}/api/ipam/ip-addresses/42/", netbox_url),
            display: "192.168.1.10/24".to_string(),
            family: 4,
            address: ip_net,
            vrf: None,
            tenant: Some(netbox_client::NestedTenant {
                id: 1,
                url: format!("{}/api/tenancy/tenants/1/", netbox_url),
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
            dns_name: None,
            description: "Test IP address".to_string(),
            comments: String::new(),
            tags: vec![], // No tags since tag doesn't exist
            custom_fields: serde_json::json!({}),
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_client.add_ip_address(created_ip);
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_ip_address(&ip_address).await;
        
        // Assert: Should succeed (missing tags are skipped, not an error)
        assert!(result.is_ok(), "Reconciliation should succeed even with missing tags: {:?}", result.err());
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_address_with_invalid_tag_kind() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Create IP address with invalid tag kind
        let (mut ip_address, tenant) = setup_ip_address_test_data();
        ip_address.status = None;
        ip_address.spec.tags = Some(vec![
            crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "InvalidKind".to_string(), // Wrong kind
                name: "some-tag".to_string(),
                namespace: Some("default".to_string()),
            },
        ]);
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store test data
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.ip_address_api.store("test-ip-address".to_string(), ip_address.clone());
        
        // Setup: Add tenant to mock NetBox
        use netbox_client::Tenant;
        let netbox_tenant = Tenant {
            id: 1,
            url: format!("{}/api/tenancy/tenants/1/", netbox_url),
            display: "Data Center Operations".to_string(),
            name: "Data Center Operations".to_string(),
            slug: "datacenter-ops".to_string(),
            description: None,
            comments: None,
            group: None,
            tags: vec![],
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_client.add_tenant(netbox_tenant);
        
        // Setup: Mock IP address creation
        let ip_net = IpNet::from_str("192.168.1.10/24").unwrap();
        let created_ip = netbox_client::IPAddress {
            id: 42,
            url: format!("{}/api/ipam/ip-addresses/42/", netbox_url),
            display: "192.168.1.10/24".to_string(),
            family: 4,
            address: ip_net,
            vrf: None,
            tenant: Some(netbox_client::NestedTenant {
                id: 1,
                url: format!("{}/api/tenancy/tenants/1/", netbox_url),
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
            dns_name: None,
            description: "Test IP address".to_string(),
            comments: String::new(),
            tags: vec![], // Invalid tag kind should be skipped
            custom_fields: serde_json::json!({}),
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_client.add_ip_address(created_ip);
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_ip_address(&ip_address).await;
        
        // Assert: Should succeed (invalid tag kind is skipped, not an error)
        assert!(result.is_ok(), "Reconciliation should succeed even with invalid tag kind: {:?}", result.err());
    }
    
    #[tokio::test]
    async fn test_reconcile_dhcp_static_with_mac() {
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
        
        // Setup: Create test tenant
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        
        // Setup: Create test IP address CRD with DHCP static reservation
        let mut ip_address = NetBoxIPAddress {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("dhcp-static-ip".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxIPAddressSpec {
                address: Some("192.168.1.100/24".to_string()),
                ip_range: None,
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "datacenter-tenant".to_string(),
                    namespace: Some("default".to_string()),
                },
                vrf: None,
                vlan: None,
                status: crds::IPAddressStatus::Dhcp,
                role: None,
                dns_name: None,
                description: Some("Static DHCP reservation".to_string()),
                tags: None,
                comments: None,
                mac_address: Some("aa:bb:cc:dd:ee:ff".to_string()),
                interface: None,
            },
            status: None,
        };
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store test data
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.ip_address_api.store("dhcp-static-ip".to_string(), ip_address.clone());
        
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
            tags: vec![],
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_client.add_tenant(netbox_tenant);
        
        // Setup: Mock interface with matching MAC address
        use netbox_client::Interface;
        use netbox_client::NestedDevice;
        let interface = Interface {
            id: 10,
            url: "http://test-netbox/api/dcim/interfaces/10/".to_string(),
            display: "eth0".to_string(),
            device: NestedDevice {
                id: 1,
                url: "http://test-netbox/api/dcim/devices/1/".to_string(),
                display: "test-device".to_string(),
                name: "test-device".to_string(),
            },
            vdcs: vec![],
            module: None,
            name: "eth0".to_string(),
            label: None,
            r#type: "1000base-t".to_string(),
            enabled: true,
            parent: None,
            bridge: None,
            lag: None,
            mac_address: Some("aa:bb:cc:dd:ee:ff".to_string()),
            mtu: Some(1500),
            description: None,
            ip_addresses: vec![],
            tags: vec![],
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_client.add_interface(interface);
        
        // Setup: Mock IP address creation
        let ip_net = IpNet::from_str("192.168.1.100/24").unwrap();
        let created_ip = netbox_client::IPAddress {
            id: 42,
            url: "http://test-netbox/api/ipam/ip-addresses/42/".to_string(),
            display: "192.168.1.100/24".to_string(),
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
            status: netbox_client::IPAddressStatus::Dhcp,
            role: None,
            assigned_object_type: Some("dcim.interface".to_string()),
            assigned_object_id: Some(10),
            assigned_object: None,
            nat_inside: None,
            nat_outside: vec![],
            dns_name: None,
            description: "Static DHCP reservation".to_string(),
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
        
        // Assert: IP should be created with interface assignment
        let stored_ip = apis.ip_address_api.get("dhcp-static-ip").await.unwrap();
        assert!(stored_ip.status.is_some(), "Status should be set");
        let status = stored_ip.status.as_ref().unwrap();
        assert_eq!(status.netbox_id, Some(42), "NetBox ID should be set");
        assert_eq!(status.address, Some("192.168.1.100/24".to_string()), "Address should be in status");
    }
    
    #[tokio::test]
    async fn test_validation_dhcp_static_missing_mac_and_interface() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Create test IP address CRD with DHCP but no MAC or interface
        let ip_address = NetBoxIPAddress {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("dhcp-invalid".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxIPAddressSpec {
                address: Some("192.168.1.100/24".to_string()),
                ip_range: None,
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "datacenter-tenant".to_string(),
                    namespace: Some("default".to_string()),
                },
                vrf: None,
                vlan: None,
                status: crds::IPAddressStatus::Dhcp,
                role: None,
                dns_name: None,
                description: None,
                tags: None,
                comments: None,
                mac_address: None,
                interface: None,
            },
            status: None,
        };
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.ip_address_api.store("dhcp-invalid".to_string(), ip_address.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_ip_address(&ip_address).await;
        
        // Assert: Should fail with InvalidInput error
        assert!(result.is_err(), "Reconciliation should fail");
        match result.unwrap_err() {
            crate::error::ControllerError::InvalidInput(msg) => {
                assert!(msg.contains("macAddress") || msg.contains("interface"), 
                    "Error message should mention macAddress or interface: {}", msg);
            }
            e => panic!("Expected InvalidInput error, got: {:?}", e),
        }
    }
    
    #[tokio::test]
    async fn test_validation_invalid_mac_format() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Create test IP address CRD with invalid MAC format
        let ip_address = NetBoxIPAddress {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("dhcp-invalid-mac".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxIPAddressSpec {
                address: Some("192.168.1.100/24".to_string()),
                ip_range: None,
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "datacenter-tenant".to_string(),
                    namespace: Some("default".to_string()),
                },
                vrf: None,
                vlan: None,
                status: crds::IPAddressStatus::Dhcp,
                role: None,
                dns_name: None,
                description: None,
                tags: None,
                comments: None,
                mac_address: Some("invalid-format".to_string()),
                interface: None,
            },
            status: None,
        };
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.ip_address_api.store("dhcp-invalid-mac".to_string(), ip_address.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_ip_address(&ip_address).await;
        
        // Assert: Should fail with InvalidInput error
        assert!(result.is_err(), "Reconciliation should fail");
        match result.unwrap_err() {
            crate::error::ControllerError::InvalidInput(msg) => {
                assert!(msg.contains("Invalid MAC address format") || msg.contains("MAC address"), 
                    "Error message should mention MAC address format: {}", msg);
            }
            e => panic!("Expected InvalidInput error, got: {:?}", e),
        }
    }
    
    #[tokio::test]
    async fn test_validation_dhcp_random_missing_ip_range() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Create test IP address CRD with DHCP but no address and no ipRange
        let ip_address = NetBoxIPAddress {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("dhcp-random-invalid".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxIPAddressSpec {
                address: None,
                ip_range: None,
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "datacenter-tenant".to_string(),
                    namespace: Some("default".to_string()),
                },
                vrf: None,
                vlan: None,
                status: crds::IPAddressStatus::Dhcp,
                role: None,
                dns_name: None,
                description: None,
                tags: None,
                comments: None,
                mac_address: None,
                interface: None,
            },
            status: None,
        };
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = 
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.ip_address_api.store("dhcp-random-invalid".to_string(), ip_address.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_ip_address(&ip_address).await;
        
        // Assert: Should fail with InvalidInput error
        assert!(result.is_err(), "Reconciliation should fail");
        match result.unwrap_err() {
            crate::error::ControllerError::InvalidInput(msg) => {
                assert!(msg.contains("ipRange") || msg.contains("address"), 
                    "Error message should mention ipRange or address: {}", msg);
            }
            e => panic!("Expected InvalidInput error, got: {:?}", e),
        }
    }

    /// An IP address that falls inside a *populated* IP range must NOT be created in
    /// NetBox (NetBox prohibits it), but reconciliation must still succeed and record the
    /// address in the CR status as terminally Created with NO NetBox ID. This is the
    /// behaviour documented in docs/NETBOX_IP_RANGE_ANALYSIS.md (Option 1) and prevents
    /// the previous 400-error → Failed(netbox_id=0) → recreate loop.
    #[tokio::test]
    async fn test_reconcile_ip_address_in_populated_range_is_tracked_only() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;

        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        let mock_client = mock_token_resolver.mock_client();

        let (reconciler, apis, _mock_event_recorder) =
            create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        // Tenant (required dependency) — both as CRD and in mock NetBox.
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        use netbox_client::Tenant;
        mock_client.add_tenant(Tenant {
            id: 1,
            url: "http://test-netbox/api/tenancy/tenants/1/".to_string(),
            display: "Data Center Operations".to_string(),
            name: "Data Center Operations".to_string(),
            slug: "datacenter-ops".to_string(),
            description: None,
            comments: None,
            group: None,
            tags: vec![],
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });

        // IP range CRD (resolves the ipRange reference to NetBox ID 100) ...
        let ip_range_crd = NetBoxIPRange {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("dhcp-pool-range".to_string()),
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
                description: Some("DHCP pool".to_string()),
                mark_utilized: false,
                mark_populated: true,
                comments: None,
                tags: None,
                drift_detection: Some(true),
            },
            status: Some(crds::NetBoxIPRangeStatus {
                netbox_id: Some(100),
                netbox_url: Some("http://test-netbox/api/ipam/ip-ranges/100/".to_string()),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        };
        apis.ip_range_api.store("dhcp-pool-range".to_string(), ip_range_crd);

        // ... and the corresponding *populated* range in mock NetBox.
        let start_net = IpNet::from_str("192.168.1.100/24").unwrap();
        let end_net = IpNet::from_str("192.168.1.200/24").unwrap();
        mock_client.add_ip_range(netbox_client::IPRange {
            id: 100,
            url: "http://test-netbox/api/ipam/ip-ranges/100/".to_string(),
            display: "192.168.1.100-200/24".to_string(),
            family: 4,
            start_address: start_net,
            end_address: end_net,
            vrf: None,
            tenant: None,
            status: netbox_client::IPRangeStatus::Active,
            role: None,
            description: "DHCP pool".to_string(),
            comments: None,
            mark_utilized: false,
            mark_populated: true, // <-- the crux: populated range
            tags: vec![],
            custom_fields: serde_json::json!({}),
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });

        // IP address CRD: static address inside the populated range, referencing it.
        let ip_address = NetBoxIPAddress {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("static-in-pool".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxIPAddressSpec {
                address: Some("192.168.1.150/24".to_string()),
                ip_range: Some(crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxIPRange".to_string(),
                    name: "dhcp-pool-range".to_string(),
                    namespace: Some("default".to_string()),
                }),
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "datacenter-tenant".to_string(),
                    namespace: Some("default".to_string()),
                },
                vrf: None,
                vlan: None,
                status: crds::IPAddressStatus::Reserved,
                role: None,
                dns_name: None,
                description: Some("Static reservation in DHCP pool".to_string()),
                tags: None,
                comments: None,
                mac_address: None,
                interface: None,
            },
            status: None,
        };
        apis.ip_address_api.store("static-in-pool".to_string(), ip_address.clone());

        // NOTE: deliberately do NOT add any IPAddress to mock NetBox — creation must be skipped.

        // Execute
        let result = reconciler.reconcile_netbox_ip_address(&ip_address).await;
        assert!(result.is_ok(), "Reconciliation should succeed for populated range: {:?}", result.err());

        // Status: Created, address recorded, but NO NetBox ID (externally managed).
        let updated = apis.ip_address_api.get("static-in-pool").await.unwrap();
        let status = updated.status.expect("status should be set");
        assert_eq!(status.state, ResourceState::Created, "populated-range IP should be terminally Created");
        assert_eq!(status.netbox_id, None, "populated-range IP must have NO NetBox ID");
        assert_eq!(
            status.address,
            Some("192.168.1.150/24".to_string()),
            "address should be recorded in status",
        );

        // Idempotency: reconciling again with the now-populated status is a no-op (no loop).
        let updated2 = apis.ip_address_api.get("static-in-pool").await.unwrap();
        let result2 = reconciler.reconcile_netbox_ip_address(&updated2).await;
        assert!(result2.is_ok(), "second reconcile should also succeed: {:?}", result2.err());
        let status2 = apis.ip_address_api.get("static-in-pool").await.unwrap().status.unwrap();
        assert_eq!(status2.state, ResourceState::Created);
        assert_eq!(status2.netbox_id, None);
    }
}

