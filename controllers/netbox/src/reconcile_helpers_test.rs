//! Unit tests for reconcile_helpers module

#[cfg(test)]
mod tests {
    use crate::reconcile_helpers::{status_needs_update, ipclaim_status_needs_update, create_pending_status_patch, create_drift_status_patch};
    use crds::*;

    #[test]
    fn test_status_needs_update_no_status() {
        // When there's no status, it should always need updating
        let needs_update = status_needs_update::<NetBoxDeviceStatus>(
            None,
            1,
            "http://netbox/api/dcim/devices/1/",
            "Created",
            None,
        );
        assert!(needs_update, "Should need update when status is None");
    }

    #[test]
    fn test_status_needs_update_all_match() {
        // When all fields match, should not need update
        let status = NetBoxDeviceStatus {
            netbox_id: Some(1),
            netbox_url: Some("http://netbox/api/dcim/devices/1/".to_string()),
            state: ResourceState::Created,
            error: None,
            last_reconciled: None,
        };
        let needs_update = status_needs_update(
            Some(&status),
            1,
            "http://netbox/api/dcim/devices/1/",
            "Created",
            None,
        );
        assert!(!needs_update, "Should not need update when all fields match");
    }

    #[test]
    fn test_status_needs_update_netbox_id_changed() {
        // When netbox_id changes, should need update
        let status = NetBoxDeviceStatus {
            netbox_id: Some(1),
            netbox_url: Some("http://netbox/api/dcim/devices/1/".to_string()),
            state: ResourceState::Created,
            error: None,
            last_reconciled: None,
        };
        let needs_update = status_needs_update(
            Some(&status),
            2, // Different ID
            "http://netbox/api/dcim/devices/1/",
            "Created",
            None,
        );
        assert!(needs_update, "Should need update when netbox_id changes");
    }

    #[test]
    fn test_status_needs_update_url_changed() {
        // When URL changes, should need update
        let status = NetBoxDeviceStatus {
            netbox_id: Some(1),
            netbox_url: Some("http://netbox/api/dcim/devices/1/".to_string()),
            state: ResourceState::Created,
            error: None,
            last_reconciled: None,
        };
        let needs_update = status_needs_update(
            Some(&status),
            1,
            "http://netbox/api/dcim/devices/2/", // Different URL
            "Created",
            None,
        );
        assert!(needs_update, "Should need update when URL changes");
    }

    #[test]
    fn test_status_needs_update_state_changed() {
        // When state changes, should need update
        let status = NetBoxDeviceStatus {
            netbox_id: Some(1),
            netbox_url: Some("http://netbox/api/dcim/devices/1/".to_string()),
            state: ResourceState::Pending,
            error: None,
            last_reconciled: None,
        };
        let needs_update = status_needs_update(
            Some(&status),
            1,
            "http://netbox/api/dcim/devices/1/",
            "Created", // Different state
            None,
        );
        assert!(needs_update, "Should need update when state changes");
    }

    #[test]
    fn test_status_needs_update_error_changed() {
        // When error changes, should need update
        let status = NetBoxDeviceStatus {
            netbox_id: Some(1),
            netbox_url: Some("http://netbox/api/dcim/devices/1/".to_string()),
            state: ResourceState::Created,
            error: None,
            last_reconciled: None,
        };
        let needs_update = status_needs_update(
            Some(&status),
            1,
            "http://netbox/api/dcim/devices/1/",
            "Created",
            Some("Error occurred"), // Error added
        );
        assert!(needs_update, "Should need update when error changes");
    }

    #[test]
    fn test_status_needs_update_error_cleared() {
        // When error is cleared, should need update
        let status = NetBoxDeviceStatus {
            netbox_id: Some(1),
            netbox_url: Some("http://netbox/api/dcim/devices/1/".to_string()),
            state: ResourceState::Failed,
            error: Some("Previous error".to_string()),
            last_reconciled: None,
        };
        let needs_update = status_needs_update(
            Some(&status),
            1,
            "http://netbox/api/dcim/devices/1/",
            "Created", // State also changed
            None, // Error cleared
        );
        assert!(needs_update, "Should need update when error is cleared");
    }

    #[test]
    fn test_ipclaim_status_needs_update_no_status() {
        // When there's no status, should need update
        let needs_update = ipclaim_status_needs_update(
            None,
            Some("192.168.1.10/24"),
            "Allocated",
            Some("http://netbox/api/ipam/ip-addresses/1/"),
            None,
        );
        assert!(needs_update, "Should need update when status is None");
    }

    #[test]
    fn test_ipclaim_status_needs_update_all_match() {
        // When all fields match, should not need update
        let status = IPClaimStatus {
            ip: Some("192.168.1.10/24".to_string()),
            state: AllocationState::Allocated,
            netbox_ip_ref: Some("http://netbox/api/ipam/ip-addresses/1/".to_string()),
            error: None,
            last_reconciled: None,
        };
        let needs_update = ipclaim_status_needs_update(
            Some(&status),
            Some("192.168.1.10/24"),
            "Allocated",
            Some("http://netbox/api/ipam/ip-addresses/1/"),
            None,
        );
        assert!(!needs_update, "Should not need update when all fields match");
    }

    #[test]
    fn test_ipclaim_status_needs_update_ip_changed() {
        // When IP changes, should need update
        let status = IPClaimStatus {
            ip: Some("192.168.1.10/24".to_string()),
            state: AllocationState::Allocated,
            netbox_ip_ref: Some("http://netbox/api/ipam/ip-addresses/1/".to_string()),
            error: None,
            last_reconciled: None,
        };
        let needs_update = ipclaim_status_needs_update(
            Some(&status),
            Some("192.168.1.11/24"), // Different IP
            "Allocated",
            Some("http://netbox/api/ipam/ip-addresses/1/"),
            None,
        );
        assert!(needs_update, "Should need update when IP changes");
    }

    #[test]
    fn test_ipclaim_status_needs_update_state_changed() {
        // When state changes, should need update
        let status = IPClaimStatus {
            ip: Some("192.168.1.10/24".to_string()),
            state: AllocationState::Pending,
            netbox_ip_ref: Some("http://netbox/api/ipam/ip-addresses/1/".to_string()),
            error: None,
            last_reconciled: None,
        };
        let needs_update = ipclaim_status_needs_update(
            Some(&status),
            Some("192.168.1.10/24"),
            "Allocated", // Different state
            Some("http://netbox/api/ipam/ip-addresses/1/"),
            None,
        );
        assert!(needs_update, "Should need update when state changes");
    }

    #[test]
    fn test_create_pending_status_patch() {
        // Test that create_pending_status_patch returns correct JSON structure
        let patch = create_pending_status_patch();
        
        assert!(patch.is_object(), "Patch should be a JSON object");
        assert!(patch.get("status").is_some(), "Patch should have 'status' field");
        
        let status = patch.get("status").unwrap();
        assert_eq!(status.get("netboxId"), Some(&serde_json::json!(0)));
        assert_eq!(status.get("netboxUrl"), Some(&serde_json::json!("")));
        assert_eq!(status.get("state"), Some(&serde_json::json!("Pending")));
        assert!(status.get("error").is_some(), "Should have error message");
    }

    #[test]
    fn test_create_drift_status_patch() {
        // Test that create_drift_status_patch returns correct JSON structure
        let patch = create_drift_status_patch();
        
        assert!(patch.is_object(), "Patch should be a JSON object");
        assert!(patch.get("status").is_some(), "Patch should have 'status' field");
        
        let status = patch.get("status").unwrap();
        assert_eq!(status.get("netboxId"), Some(&serde_json::json!(0)));
        assert_eq!(status.get("netboxUrl"), Some(&serde_json::json!("")));
        assert_eq!(status.get("state"), Some(&serde_json::json!("Pending")));
        assert!(status.get("error").is_some(), "Should have error message");
        assert_eq!(
            status.get("error").unwrap().as_str(),
            Some("Resource was deleted in NetBox, will recreate")
        );
    }

    #[test]
    fn test_status_needs_update_with_prefix_status() {
        // Test with NetBoxPrefixStatus (different status type)
        let status = NetBoxPrefixStatus {
            netbox_id: Some(1),
            netbox_url: Some("http://netbox/api/ipam/prefixes/1/".to_string()),
            state: PrefixState::Created,
            error: None,
            last_reconciled: None,
        };
        let needs_update = status_needs_update(
            Some(&status),
            1,
            "http://netbox/api/ipam/prefixes/1/",
            "Created",
            None,
        );
        assert!(!needs_update, "Should not need update when all fields match");
    }

    #[test]
    fn test_status_needs_update_with_tenant_status() {
        // Test with NetBoxTenantStatus (different status type)
        let status = NetBoxTenantStatus {
            netbox_id: Some(1),
            netbox_url: Some("http://netbox/api/tenancy/tenants/1/".to_string()),
            state: ResourceState::Created,
            error: None,
            last_reconciled: None,
        };
        let needs_update = status_needs_update(
            Some(&status),
            1,
            "http://netbox/api/tenancy/tenants/1/",
            "Created",
            None,
        );
        assert!(!needs_update, "Should not need update when all fields match");
    }

    // Async helper function tests using MockNetBoxClient
    mod async_tests {
        use crate::reconcile_helpers::{check_existing, check_and_update_existing, NetBoxResource};
        use crate::test_utils::create_test_prefix;
        use netbox_client::{MockNetBoxClient, NetBoxClientTrait};
        use crate::error::ControllerError;
        
        #[tokio::test]
        async fn test_check_existing_resource_exists() {
            // Setup: Create mock NetBoxClient with existing prefix
            let mut mock_client = MockNetBoxClient::new("http://test-netbox");
            let test_prefix = create_test_prefix(1, "192.168.1.0/24", "http://test-netbox");
            mock_client.add_prefix(test_prefix.clone());
            
            // Execute: Check if resource exists (using trait method via reference)
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            let result = check_existing(
                client_ref,
                1,
                "test-prefix",
                async { client_ref.get_prefix(1).await },
            ).await;
            
            // Assert: Should return Some(resource)
            assert!(result.is_ok());
            let resource = result.unwrap();
            assert!(resource.is_some());
            let prefix = resource.unwrap();
            assert_eq!(prefix.id(), 1);
        }
        
        #[tokio::test]
        async fn test_check_existing_resource_not_found() {
            // Setup: Create mock NetBoxClient without prefix
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            
            // Execute: Check if resource exists
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            let result = check_existing(
                client_ref,
                999,
                "non-existent-prefix",
                async { client_ref.get_prefix(999).await },
            ).await;
            
            // Assert: Should return Ok(None) for drift detection
            assert!(result.is_ok());
            let resource = result.unwrap();
            assert!(resource.is_none()); // Drift detected - resource deleted
        }
        
        #[tokio::test]
        async fn test_check_and_update_existing_no_update_needed() {
            // Setup: Create mock NetBoxClient with existing prefix
            let mut mock_client = MockNetBoxClient::new("http://test-netbox");
            let test_prefix = create_test_prefix(1, "192.168.1.0/24", "http://test-netbox");
            mock_client.add_prefix(test_prefix.clone());
            
            // Execute: Check and update (but needs_update returns false)
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            let result = check_and_update_existing(
                client_ref,
                1,
                "test-prefix",
                async { client_ref.get_prefix(1).await },
                |_| false, // No update needed
                async { client_ref.get_prefix(1).await }, // Won't be called
            ).await;
            
            // Assert: Should return Some(existing) without updating
            assert!(result.is_ok());
            let resource = result.unwrap();
            assert!(resource.is_some());
            let prefix = resource.unwrap();
            assert_eq!(prefix.id(), 1);
        }
        
        #[tokio::test]
        async fn test_check_and_update_existing_update_needed() {
            // Setup: Create mock NetBoxClient with existing prefix
            let mut mock_client = MockNetBoxClient::new("http://test-netbox");
            let test_prefix = create_test_prefix(1, "192.168.1.0/24", "http://test-netbox");
            mock_client.add_prefix(test_prefix.clone());
            
            // Create updated prefix for update response
            let mut updated_prefix = test_prefix.clone();
            updated_prefix.description = "Updated description".to_string();
            
            // Execute: Check and update (needs_update returns true)
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            let result = check_and_update_existing(
                client_ref,
                1,
                "test-prefix",
                async { client_ref.get_prefix(1).await },
                |_| true, // Update needed
                async { 
                    // Simulate update by returning updated prefix
                    Ok(updated_prefix.clone())
                },
            ).await;
            
            // Assert: Should return Some(updated)
            assert!(result.is_ok());
            let resource = result.unwrap();
            assert!(resource.is_some());
            let updated_prefix = resource.unwrap();
            assert_eq!(updated_prefix.id(), 1);
        }
        
        #[tokio::test]
        async fn test_check_and_update_existing_resource_deleted() {
            // Setup: Create mock NetBoxClient without prefix
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            
            // Execute: Check and update (resource not found)
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            let result = check_and_update_existing(
                client_ref,
                999,
                "non-existent-prefix",
                async { client_ref.get_prefix(999).await },
                |_| true,
                async { client_ref.get_prefix(999).await },
            ).await;
            
            // Assert: Should return Ok(None) for drift detection
            assert!(result.is_ok());
            let resource = result.unwrap();
            assert!(resource.is_none()); // Drift detected - resource deleted
        }
    }
}

