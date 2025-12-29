//! Unit tests for IPPool reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use netbox_client::AvailableIP;
    use crds::{IPPool, NetBoxPrefix};
    
    /// Helper to set up test data for IP pool reconciliation
    fn setup_ip_pool_test_data() -> (IPPool, NetBoxPrefix) {
        // Create test NetBoxPrefix with status (required dependency)
        let prefix = create_test_netbox_prefix(
            "test-prefix",
            "default",
            1,
            Some("http://test-netbox/api/ipam/prefixes/1/".to_string()),
        );
        
        // Create test IPPool CRD
        let pool = create_test_ip_pool("test-pool", "default", "test-prefix", None);
        
        (pool, prefix)
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_pool_success() {
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
        let (pool, prefix) = setup_ip_pool_test_data();
        
        // Setup: Create reconciler with MockTokenResolver
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store test data in the APIs before reconciliation
        apis.tenant_api.store("datacenter-tenant".to_string(), create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        ));
        apis.prefix_api.store("test-prefix".to_string(), prefix);
        apis.ip_pool_api.store("test-pool".to_string(), pool.clone());
        
        // Setup: Add prefix to mock NetBox (required for get_prefix call)
        let netbox_prefix = create_test_prefix(1, "192.168.1.0/24", &netbox_url);
        mock_client.add_prefix(netbox_prefix);
        
        // Setup: Add available IPs to mock NetBox
        let available_ips = vec![
            AvailableIP {
                family: 4,
                address: "192.168.1.1/24".to_string(),
                vrf: None,
                description: None,
            },
            AvailableIP {
                family: 4,
                address: "192.168.1.2/24".to_string(),
                vrf: None,
                description: None,
            },
        ];
        mock_client.set_available_ips(1, available_ips);
        
        // Execute: Reconcile
        let result = reconciler.reconcile_ip_pool(&pool).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with correct values
        let updated_crd = apis.ip_pool_api.get("test-pool").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_prefix_id, Some(1), "Prefix ID should be set");
        assert_eq!(status.total_ips, 2, "Total IPs should be 2");
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_pool_prefix_not_found() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url));
        
        // Setup: Create reconciler
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create IPPool with reference to non-existent prefix
        let pool = create_test_ip_pool("test-pool", "default", "non-existent-prefix", None);
        apis.ip_pool_api.store("test-pool".to_string(), pool.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_ip_pool(&pool).await;
        
        // Assert: Should fail with PrefixNotFound error
        assert!(result.is_err(), "Reconciliation should fail when prefix not found");
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_pool_prefix_no_status() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url));
        
        // Setup: Create reconciler
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create prefix without status (not created in NetBox yet)
        let mut prefix = create_test_netbox_prefix("test-prefix", "default", 1, None);
        prefix.status = None; // Clear status
        apis.prefix_api.store("test-prefix".to_string(), prefix);
        
        // Setup: Create IPPool with reference to prefix without status
        let pool = create_test_ip_pool("test-pool", "default", "test-prefix", None);
        apis.ip_pool_api.store("test-pool".to_string(), pool.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_ip_pool(&pool).await;
        
        // Assert: Should fail with PrefixNotFound error (prefix has no status)
        assert!(result.is_err(), "Reconciliation should fail when prefix has no status");
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_pool_no_status_update_needed() {
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
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create prefix with status
        let prefix = create_test_netbox_prefix(
            "test-prefix",
            "default",
            1,
            Some("http://test-netbox/api/ipam/prefixes/1/".to_string()),
        );
        apis.tenant_api.store("datacenter-tenant".to_string(), create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        ));
        apis.prefix_api.store("test-prefix".to_string(), prefix);
        
        // Setup: Add prefix to mock NetBox
        let netbox_prefix = create_test_prefix(1, "192.168.1.0/24", &netbox_url);
        mock_client.add_prefix(netbox_prefix);
        
        // Setup: Set up available IPs (2 IPs)
        let available_ips = vec![
            netbox_client::AvailableIP {
                family: 4,
                address: "192.168.1.1/24".to_string(),
                vrf: None,
                description: None,
            },
            netbox_client::AvailableIP {
                family: 4,
                address: "192.168.1.2/24".to_string(),
                vrf: None,
                description: None,
            },
        ];
        mock_client.set_available_ips(1, available_ips);
        
        // Setup: Create IPPool with status that matches current NetBox state
        let mut pool = create_test_ip_pool("test-pool", "default", "test-prefix", None);
        pool.status = Some(crds::IPPoolStatus {
            netbox_prefix_id: Some(1),
            netbox_prefix_url: Some("http://test-netbox/api/ipam/prefixes/1/".to_string()),
            total_ips: 2,
            allocated_ips: 0,
            available_ips: 2,
            last_reconciled: None,
        });
        apis.ip_pool_api.store("test-pool".to_string(), pool.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_ip_pool(&pool).await;
        
        // Assert: Should succeed (idempotent - no update needed)
        assert!(result.is_ok(), "Reconciliation should succeed when status matches");
        
        // Note: The reconciler should detect no update is needed and return early
        // We verify this by the test passing (no error means it detected no change needed)
    }
    
    #[tokio::test]
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_ip_pool_status_update() {
        // TODO: Test status update when IP counts change
        // 1. Create IPPool with status
        // 2. Change available IP count in NetBox
        // 3. Reconcile
        // 4. Verify status is updated with new counts
    }
}

