//! Unit tests for IPClaim reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crds::{IPClaim, IPPool, NetBoxPrefix, AllocationState};
    
    /// Helper to set up test data for IP claim reconciliation
    fn setup_ip_claim_test_data() -> (IPClaim, IPPool, NetBoxPrefix) {
        // Create test NetBoxPrefix with status
        let prefix = create_test_netbox_prefix(
            "test-prefix",
            "default",
            1,
            Some("http://test-netbox/api/ipam/prefixes/1/".to_string()),
        );
        
        // Create test IPPool with status
        let mut pool = create_test_ip_pool("test-pool", "default", "test-prefix", None);
        pool.status = Some(crds::IPPoolStatus {
            netbox_prefix_id: Some(1),
            netbox_prefix_url: Some("http://test-netbox/api/ipam/prefixes/1/".to_string()),
            total_ips: 256,
            allocated_ips: 0,
            available_ips: 256,
            last_reconciled: None,
        });
        
        // Create test IPClaim CRD
        let claim = create_test_ip_claim(
            "test-claim",
            "default",
            "test-pool",
            None,
            "test-device",
            Some("eth0"),
            None,
        );
        
        (claim, pool, prefix)
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_claim_success() {
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
        let (claim, pool, prefix) = setup_ip_claim_test_data();
        
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
        apis.ip_pool_api.store("test-pool".to_string(), pool);
        apis.ip_claim_api.store("test-claim".to_string(), claim.clone());
        
        // Setup: Add prefix to mock NetBox (required for get_prefix call)
        let netbox_prefix = create_test_prefix(1, "192.168.1.0/24", &netbox_url);
        mock_client.add_prefix(netbox_prefix);
        
        // Setup: Add available IPs to mock NetBox (required for allocate_ip call)
        let available_ips = vec![
            netbox_client::AvailableIP {
                family: 4,
                address: "192.168.1.1/24".to_string(),
                vrf: None,
                description: None,
            },
        ];
        mock_client.set_available_ips(1, available_ips);
        
        // Execute: Reconcile
        let result = reconciler.reconcile_ip_claim(&claim).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with allocated IP
        let updated_crd = apis.ip_claim_api.get("test-claim").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.ip.is_some(), "IP should be allocated");
        assert_eq!(status.state, AllocationState::Allocated, "State should be Allocated");
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_claim_pool_not_found() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url));
        
        // Setup: Create reconciler
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create IPClaim with reference to non-existent pool
        let claim = create_test_ip_claim(
            "test-claim",
            "default",
            "non-existent-pool",
            None,
            "test-device",
            None,
            None,
        );
        apis.ip_claim_api.store("test-claim".to_string(), claim.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_ip_claim(&claim).await;
        
        // Assert: Should fail with IPPoolNotFound error
        assert!(result.is_err(), "Reconciliation should fail when pool not found");
        
        // Assert: Status should be updated with error
        let updated_crd = apis.ip_claim_api.get("test-claim").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.state, AllocationState::Failed, "State should be Failed");
        assert!(status.error.is_some(), "Error should be set");
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_claim_pool_no_status() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Create reconciler
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create prefix without status
        let mut prefix = create_test_netbox_prefix("test-prefix", "default", 1, None);
        prefix.status = None; // Clear status
        apis.prefix_api.store("test-prefix".to_string(), prefix);
        
        // Setup: Create IPPool without status
        let mut pool = create_test_ip_pool("test-pool", "default", "test-prefix", None);
        pool.status = None; // No status
        apis.ip_pool_api.store("test-pool".to_string(), pool);
        
        // Setup: Create IPClaim
        let claim = create_test_ip_claim(
            "test-claim",
            "default",
            "test-pool",
            None,
            "test-device",
            None,
            None,
        );
        apis.ip_claim_api.store("test-claim".to_string(), claim.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_ip_claim(&claim).await;
        
        // Assert: Should fail (pool has no status, can't resolve prefix ID)
        assert!(result.is_err(), "Reconciliation should fail when pool has no status");
        
        // Assert: Status should be updated with error
        let updated_crd = apis.ip_claim_api.get("test-claim").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.state, AllocationState::Failed, "State should be Failed");
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_claim_already_allocated() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url));
        
        // Setup: Create reconciler
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create IPClaim with status showing already allocated
        let mut claim = create_test_ip_claim(
            "test-claim",
            "default",
            "test-pool",
            None,
            "test-device",
            None,
            None,
        );
        claim.status = Some(crds::IPClaimStatus {
            ip: Some("192.168.1.1/24".to_string()),
            state: AllocationState::Allocated,
            error: None,
            netbox_ip_ref: None,
            last_reconciled: None,
        });
        apis.ip_claim_api.store("test-claim".to_string(), claim.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_ip_claim(&claim).await;
        
        // Assert: Should succeed (early return)
        assert!(result.is_ok(), "Reconciliation should succeed for already allocated claim");
        
        // Note: We can't easily verify that status patch was NOT called without more sophisticated mocking
        // But the early return is verified by the test passing
    }
    
    #[tokio::test]
    async fn test_reconcile_ip_claim_no_available_ips() {
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
        
        // Setup: Create test data
        let prefix = create_test_netbox_prefix(
            "test-prefix",
            "default",
            1,
            Some("http://test-netbox/api/ipam/prefixes/1/".to_string()),
        );
        let mut pool = create_test_ip_pool("test-pool", "default", "test-prefix", None);
        pool.status = Some(crds::IPPoolStatus {
            netbox_prefix_id: Some(1),
            netbox_prefix_url: Some("http://test-netbox/api/ipam/prefixes/1/".to_string()),
            total_ips: 256,
            allocated_ips: 256,
            available_ips: 0, // No available IPs
            last_reconciled: None,
        });
        let claim = create_test_ip_claim(
            "test-claim",
            "default",
            "test-pool",
            None,
            "test-device",
            None,
            None,
        );
        
        // Setup: Store test data
        apis.tenant_api.store("datacenter-tenant".to_string(), create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        ));
        apis.prefix_api.store("test-prefix".to_string(), prefix);
        apis.ip_pool_api.store("test-pool".to_string(), pool);
        apis.ip_claim_api.store("test-claim".to_string(), claim.clone());
        
        // Setup: Add prefix to mock NetBox
        let netbox_prefix = create_test_prefix(1, "192.168.1.0/24", &netbox_url);
        mock_client.add_prefix(netbox_prefix);
        
        // Setup: Set no available IPs (empty vector)
        mock_client.set_available_ips(1, vec![]);
        
        // Execute: Reconcile
        let result = reconciler.reconcile_ip_claim(&claim).await;
        
        // Assert: Should fail (no available IPs)
        assert!(result.is_err(), "Reconciliation should fail when no IPs are available");
        
        // Assert: Status should be updated with error
        let updated_crd = apis.ip_claim_api.get("test-claim").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.state, AllocationState::Failed, "State should be Failed");
        assert!(status.error.is_some(), "Error should be set");
    }
    
    #[tokio::test]
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_ip_claim_with_device_interface() {
        // TODO: Test IP allocation with device and interface specified
        // 1. Create IPClaim with device and interface
        // 2. Reconcile
        // 3. Verify IP is allocated and assigned to interface
    }
}

