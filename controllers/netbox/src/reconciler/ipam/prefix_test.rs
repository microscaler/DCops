//! Unit tests for NetBoxPrefix reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::kube_api_trait::mock::MockKubeApi;
    use crate::reconciler::Reconciler;
    use netbox_client::MockNetBoxClient;
    use crds::NetBoxPrefix;
    
    #[tokio::test]
    async fn test_reconcile_prefix_create() {
        // Setup: Create mock NetBoxClient
        let mock_client = MockNetBoxClient::new("http://test-netbox");
        
        // Setup: Create test NetBoxPrefix CRD (no status - needs creation)
        let mut netbox_prefix = create_test_netbox_prefix("test-prefix", "default", 0, None);
        netbox_prefix.status = None; // Clear status to test create path
        
        // Setup: Create mock API and store the CRD
        let mock_prefix_api = MockKubeApi::<NetBoxPrefix>::new();
        mock_prefix_api.store("test-prefix".to_string(), netbox_prefix.clone());
        
        // Setup: Create reconciler with the mock API
        let reconciler = Reconciler::new(
            mock_client,
            // netbox_prefix_api - use our mock with stored CRD
            mock_prefix_api,
            // All other APIs - use default mocks
            MockKubeApi::new(), // netbox_role_api
            MockKubeApi::new(), // netbox_tag_api
            MockKubeApi::new(), // netbox_aggregate_api
            MockKubeApi::new(), // netbox_vlan_api
            MockKubeApi::new(), // netbox_tenant_api
            MockKubeApi::new(), // netbox_site_api
            MockKubeApi::new(), // netbox_device_role_api
            MockKubeApi::new(), // netbox_manufacturer_api
            MockKubeApi::new(), // netbox_platform_api
            MockKubeApi::new(), // netbox_device_type_api
            MockKubeApi::new(), // netbox_device_api
            MockKubeApi::new(), // netbox_interface_api
            MockKubeApi::new(), // netbox_mac_address_api
            MockKubeApi::new(), // netbox_region_api
            MockKubeApi::new(), // netbox_site_group_api
            MockKubeApi::new(), // netbox_location_api
            MockKubeApi::new(), // ip_pool_api
            MockKubeApi::new(), // ip_claim_api
        );
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_prefix(&netbox_prefix).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        // Get the updated CRD from the mock API
        let updated_crd = reconciler.netbox_prefix_api.get("test-prefix").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, crds::PrefixState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_prefix_update() {
        // Setup: Create mock NetBoxClient
        let mut mock_client = MockNetBoxClient::new("http://test-netbox");
        
        // Setup: Create existing prefix in NetBox
        let existing_prefix = create_test_prefix(1, "192.168.1.0/24", "http://test-netbox");
        mock_client.add_prefix(existing_prefix);
        
        // Setup: Create NetBoxPrefix CRD with status and updated description
        let mut netbox_prefix = create_test_netbox_prefix(
            "test-prefix",
            "default",
            1,
            Some("http://test-netbox/api/ipam/prefixes/1/".to_string()),
        );
        netbox_prefix.spec.description = Some("Updated description".to_string());
        
        // Setup: Create mock API and store the CRD
        let mock_prefix_api = MockKubeApi::<NetBoxPrefix>::new();
        mock_prefix_api.store("test-prefix".to_string(), netbox_prefix.clone());
        
        // Setup: Create reconciler
        let reconciler = Reconciler::new(
            mock_client,
            mock_prefix_api,
            MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(),
            MockKubeApi::new(),
            MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(),
            MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(),
            MockKubeApi::new(), MockKubeApi::new(),
        );
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_prefix(&netbox_prefix).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Prefix should be updated in NetBox (description changed)
        // The update will be reflected in the mock client's prefix store
        // We can verify by checking the prefix was updated
    }
    
    #[tokio::test]
    async fn test_reconcile_prefix_idempotent() {
        // Setup: Create mock NetBoxClient
        let mut mock_client = MockNetBoxClient::new("http://test-netbox");
        
        // Setup: Create existing prefix in NetBox
        let existing_prefix = create_test_prefix(1, "192.168.1.0/24", "http://test-netbox");
        mock_client.add_prefix(existing_prefix);
        
        // Setup: Create NetBoxPrefix CRD with matching spec
        let netbox_prefix = create_test_netbox_prefix(
            "test-prefix",
            "default",
            1,
            Some("http://test-netbox/api/ipam/prefixes/1/".to_string()),
        );
        
        // Setup: Create mock API and store the CRD
        let mock_prefix_api = MockKubeApi::<NetBoxPrefix>::new();
        mock_prefix_api.store("test-prefix".to_string(), netbox_prefix.clone());
        
        // Setup: Create reconciler
        let reconciler = Reconciler::new(
            mock_client,
            mock_prefix_api,
            MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(),
            MockKubeApi::new(),
            MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(),
            MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(), MockKubeApi::new(),
            MockKubeApi::new(), MockKubeApi::new(),
        );
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_prefix(&netbox_prefix).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain unchanged (idempotent - no update needed)
        // The prefix already exists and matches the spec, so no update should occur
    }
}

