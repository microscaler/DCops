//! Unit tests for reconcile_helpers module

#[cfg(test)]
mod tests {
    use crate::reconcile_helpers::{status_needs_update, ipclaim_status_needs_update, create_pending_status_patch, create_drift_status_patch, is_conflict_error, resolve_dependency_id, extract_name_and_namespace, validate_reference_kind};
    use crds::*;
    use netbox_client::NetBoxError;

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

    // Tests for is_conflict_error
    #[test]
    fn test_is_conflict_error_already_exists() {
        let error = NetBoxError::Api("tenant with this name already exists".to_string());
        assert!(is_conflict_error(&error), "Should detect 'already exists' conflict");
    }

    #[test]
    fn test_is_conflict_error_duplicate() {
        let error = NetBoxError::Api("duplicate entry found".to_string());
        assert!(is_conflict_error(&error), "Should detect 'duplicate' conflict");
    }

    #[test]
    fn test_is_conflict_error_unique_constraint() {
        let error = NetBoxError::Api("unique constraint violation".to_string());
        assert!(is_conflict_error(&error), "Should detect 'unique constraint' conflict");
    }

    #[test]
    fn test_is_conflict_error_slug_already_exists() {
        let error = NetBoxError::Api("slug already exists".to_string());
        assert!(is_conflict_error(&error), "Should detect slug conflict");
    }

    #[test]
    fn test_is_conflict_error_asset_tag() {
        let error = NetBoxError::Api("asset tag conflict".to_string());
        assert!(is_conflict_error(&error), "Should detect asset tag conflict");
    }

    #[test]
    fn test_is_conflict_error_not_conflict() {
        let error = NetBoxError::Api("Invalid token".to_string());
        assert!(!is_conflict_error(&error), "Should not detect non-conflict errors");
    }

    #[test]
    fn test_is_conflict_error_not_found() {
        let error = NetBoxError::NotFound("Resource not found".to_string());
        assert!(!is_conflict_error(&error), "Should not detect NotFound as conflict");
    }

    // Tests for resolve_dependency_id
    #[test]
    fn test_resolve_dependency_id_with_valid_id() {
        let status = NetBoxDeviceStatus {
            netbox_id: Some(42),
            netbox_url: Some("http://netbox/api/dcim/devices/42/".to_string()),
            state: ResourceState::Created,
            error: None,
            last_reconciled: None,
        };
        let id = resolve_dependency_id(Some(&status), "Device", "test-device");
        assert_eq!(id, Some(42), "Should return the netbox_id from status");
    }

    #[test]
    fn test_resolve_dependency_id_no_status() {
        let id = resolve_dependency_id(None::<&NetBoxDeviceStatus>, "Device", "test-device");
        assert_eq!(id, None, "Should return None when status is None");
    }

    #[test]
    fn test_resolve_dependency_id_no_netbox_id() {
        let status = NetBoxDeviceStatus {
            netbox_id: None,
            netbox_url: None,
            state: ResourceState::Pending,
            error: None,
            last_reconciled: None,
        };
        let id = resolve_dependency_id(Some(&status), "Device", "test-device");
        assert_eq!(id, None, "Should return None when netbox_id is None");
    }

    #[test]
    fn test_resolve_dependency_id_invalid_id_zero() {
        // Note: resolve_dependency_id doesn't filter out 0 - it just returns what's in status
        // The filtering happens in resolve_optional_dependency_id
        let status = NetBoxDeviceStatus {
            netbox_id: Some(0),
            netbox_url: Some("http://netbox/api/dcim/devices/0/".to_string()),
            state: ResourceState::Created,
            error: None,
            last_reconciled: None,
        };
        let id = resolve_dependency_id(Some(&status), "Device", "test-device");
        assert_eq!(id, Some(0), "resolve_dependency_id returns the ID from status, even if 0");
    }

    // Tests for extract_name_and_namespace
    #[test]
    fn test_extract_name_and_namespace_with_namespace() {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        let device = NetBoxDevice {
            metadata: ObjectMeta {
                name: Some("test-device".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxDeviceSpec {
                name: Some("test-device".to_string()),
                device_type: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxDeviceType".to_string(),
                    name: "test-type".to_string(),
                    namespace: None,
                },
                device_role: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxDeviceRole".to_string(),
                    name: "test-role".to_string(),
                    namespace: None,
                },
                site: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxSite".to_string(),
                    name: "test-site".to_string(),
                    namespace: None,
                },
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "datacenter-tenant".to_string(),
                    namespace: None,
                },
                location: None,
                platform: None,
                serial: None,
                asset_tag: None,
                status: crds::DeviceStatus::Active,
                primary_ip4: None,
                primary_ip6: None,
                description: None,
                comments: None,
            },
            status: None,
        };
        let result = extract_name_and_namespace(&device, "NetBoxDevice");
        assert!(result.is_ok(), "Should extract name and namespace successfully");
        let (name, namespace) = result.unwrap();
        assert_eq!(name, "test-device");
        assert_eq!(namespace, "default");
    }

    #[test]
    fn test_extract_name_and_namespace_missing_name() {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        let device = NetBoxDevice {
            metadata: ObjectMeta {
                name: None,
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxDeviceSpec {
                name: Some("test-device".to_string()),
                device_type: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxDeviceType".to_string(),
                    name: "test-type".to_string(),
                    namespace: None,
                },
                device_role: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxDeviceRole".to_string(),
                    name: "test-role".to_string(),
                    namespace: None,
                },
                site: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxSite".to_string(),
                    name: "test-site".to_string(),
                    namespace: None,
                },
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "datacenter-tenant".to_string(),
                    namespace: None,
                },
                location: None,
                platform: None,
                serial: None,
                asset_tag: None,
                status: crds::DeviceStatus::Active,
                primary_ip4: None,
                primary_ip6: None,
                description: None,
                comments: None,
            },
            status: None,
        };
        let result = extract_name_and_namespace(&device, "NetBoxDevice");
        assert!(result.is_err(), "Should error when name is missing");
    }

    #[test]
    fn test_extract_name_and_namespace_missing_namespace() {
        // Note: extract_name_and_namespace defaults namespace to "default" if None
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        let device = NetBoxDevice {
            metadata: ObjectMeta {
                name: Some("test-device".to_string()),
                namespace: None,
                ..Default::default()
            },
            spec: crds::NetBoxDeviceSpec {
                name: Some("test-device".to_string()),
                device_type: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxDeviceType".to_string(),
                    name: "test-type".to_string(),
                    namespace: None,
                },
                device_role: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxDeviceRole".to_string(),
                    name: "test-role".to_string(),
                    namespace: None,
                },
                site: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxSite".to_string(),
                    name: "test-site".to_string(),
                    namespace: None,
                },
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: "datacenter-tenant".to_string(),
                    namespace: None,
                },
                location: None,
                platform: None,
                serial: None,
                asset_tag: None,
                status: crds::DeviceStatus::Active,
                primary_ip4: None,
                primary_ip6: None,
                description: None,
                comments: None,
            },
            status: None,
        };
        let result = extract_name_and_namespace(&device, "NetBoxDevice");
        assert!(result.is_ok(), "Should default namespace to 'default' when None");
        let (name, namespace) = result.unwrap();
        assert_eq!(name, "test-device");
        assert_eq!(namespace, "default");
    }

    // Tests for validate_reference_kind
    #[test]
    fn test_validate_reference_kind_correct() {
        let reference = NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxDeviceType".to_string(),
            name: "test-type".to_string(),
            namespace: None,
        };
        let result = validate_reference_kind(&reference, "NetBoxDeviceType", "device_type", "test-resource");
        assert!(result.is_ok(), "Should validate correct kind");
    }

    #[test]
    fn test_validate_reference_kind_incorrect() {
        let reference = NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxDeviceType".to_string(),
            name: "test-type".to_string(),
            namespace: None,
        };
        let result = validate_reference_kind(&reference, "NetBoxSite", "device_type", "test-resource");
        assert!(result.is_err(), "Should error on incorrect kind");
    }

    // Async helper function tests using MockNetBoxClient
    mod async_tests {
        use crate::reconcile_helpers::{check_existing, check_and_update_existing, NetBoxResource};
        use crate::test_utils::create_test_prefix;
        use netbox_client::{MockNetBoxClient, NetBoxClientTrait};
        
        #[tokio::test]
        async fn test_check_existing_resource_exists() {
            // Setup: Create mock NetBoxClient with existing prefix
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            let test_prefix = create_test_prefix(1, "192.168.1.0/24", "http://test-netbox");
            mock_client.add_prefix(test_prefix.clone());
            
            // Execute: Check if resource exists (using trait method via reference)
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            let result = check_existing(
                client_ref,
                1,
                "test-prefix",
                async { client_ref.get_prefix(netbox_client::PrefixId(1)).await },
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
                async { client_ref.get_prefix(netbox_client::PrefixId(999)).await },
            ).await;
            
            // Assert: Should return Ok(None) for drift detection
            assert!(result.is_ok());
            let resource = result.unwrap();
            assert!(resource.is_none()); // Drift detected - resource deleted
        }
        
        #[tokio::test]
        async fn test_check_and_update_existing_no_update_needed() {
            // Setup: Create mock NetBoxClient with existing prefix
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            let test_prefix = create_test_prefix(1, "192.168.1.0/24", "http://test-netbox");
            mock_client.add_prefix(test_prefix.clone());
            
            // Execute: Check and update (but needs_update returns false)
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            let result = check_and_update_existing(
                client_ref,
                1,
                "test-prefix",
                async { client_ref.get_prefix(netbox_client::PrefixId(1)).await },
                |_| false, // No update needed
                async { client_ref.get_prefix(netbox_client::PrefixId(1)).await }, // Won't be called
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
            let mock_client = MockNetBoxClient::new("http://test-netbox");
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
                async { client_ref.get_prefix(netbox_client::PrefixId(1)).await },
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
                async { client_ref.get_prefix(netbox_client::PrefixId(999)).await },
                |_| true,
                async { client_ref.get_prefix(netbox_client::PrefixId(999)).await },
            ).await;
            
            // Assert: Should return Ok(None) for drift detection
            assert!(result.is_ok());
            let resource = result.unwrap();
            assert!(resource.is_none()); // Drift detected - resource deleted
        }
    }

    // Tests for validate_status_and_drift
    mod validate_status_and_drift_tests {
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        use crate::test_utils::create_test_prefix;
        use netbox_client::{MockNetBoxClient, NetBoxClientTrait};
        use crds::*;
        
        #[tokio::test]
        async fn test_validate_status_and_drift_no_status() {
            // No status - should return Recreate
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            
            let result = validate_status_and_drift::<netbox_client::Prefix, _, _>(
                None::<&NetBoxPrefixStatus>,
                "NetBoxPrefix",
                "default",
                "test-prefix",
                |_| async { client_ref.get_prefix(netbox_client::PrefixId(1)).await },
            ).await;
            
            assert!(result.is_ok());
            match result.unwrap() {
                DriftCheckResult::Recreate => {}
                _ => panic!("Should return Recreate when no status"),
            }
        }
        
        #[tokio::test]
        async fn test_validate_status_and_drift_created_with_valid_id_exists() {
            // Created state with valid ID, resource exists
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            let test_prefix = create_test_prefix(1, "192.168.1.0/24", "http://test-netbox");
            mock_client.add_prefix(test_prefix.clone());
            
            let status = NetBoxPrefixStatus {
                netbox_id: Some(1),
                netbox_url: Some("http://test-netbox/api/ipam/prefixes/1/".to_string()),
                state: PrefixState::Created,
                error: None,
                last_reconciled: None,
            };
            
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            let result = validate_status_and_drift::<netbox_client::Prefix, _, _>(
                Some(&status),
                "NetBoxPrefix",
                "default",
                "test-prefix",
                |id| async move { client_ref.get_prefix(netbox_client::PrefixId(id)).await },
            ).await;
            
            assert!(result.is_ok());
            match result.unwrap() {
                DriftCheckResult::UseExisting(prefix) => {
                    assert_eq!(prefix.id, 1);
                }
                _ => panic!("Should return UseExisting when resource exists"),
            }
        }
        
        #[tokio::test]
        async fn test_validate_status_and_drift_created_with_valid_id_not_found() {
            // Created state with valid ID, but resource deleted (drift)
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            
            let status = NetBoxPrefixStatus {
                netbox_id: Some(999),
                netbox_url: Some("http://test-netbox/api/ipam/prefixes/999/".to_string()),
                state: PrefixState::Created,
                error: None,
                last_reconciled: None,
            };
            
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            let result = validate_status_and_drift::<netbox_client::Prefix, _, _>(
                Some(&status),
                "NetBoxPrefix",
                "default",
                "test-prefix",
                |id| async move { client_ref.get_prefix(netbox_client::PrefixId(id)).await },
            ).await;
            
            assert!(result.is_ok());
            match result.unwrap() {
                DriftCheckResult::StatusCleared { message } => {
                    assert!(message.contains("deleted in NetBox"));
                }
                _ => panic!("Should return StatusCleared when resource deleted"),
            }
        }
        
        #[tokio::test]
        async fn test_validate_status_and_drift_created_with_invalid_id_zero() {
            // Created state with invalid ID (0) - should clear status
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            
            let status = NetBoxPrefixStatus {
                netbox_id: Some(0),
                netbox_url: Some("http://test-netbox/api/ipam/prefixes/0/".to_string()),
                state: PrefixState::Created,
                error: None,
                last_reconciled: None,
            };
            
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            let result = validate_status_and_drift::<netbox_client::Prefix, _, _>(
                Some(&status),
                "NetBoxPrefix",
                "default",
                "test-prefix",
                |_| async { client_ref.get_prefix(netbox_client::PrefixId(0)).await },
            ).await;
            
            assert!(result.is_ok());
            match result.unwrap() {
                DriftCheckResult::StatusCleared { message } => {
                    assert!(message.contains("Invalid netbox_id (0)"));
                }
                _ => panic!("Should return StatusCleared when ID is 0"),
            }
        }
        
        #[tokio::test]
        async fn test_validate_status_and_drift_created_no_netbox_id() {
            // Created state but no netbox_id - should recreate
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            
            let status = NetBoxPrefixStatus {
                netbox_id: None,
                netbox_url: None,
                state: PrefixState::Created,
                error: None,
                last_reconciled: None,
            };
            
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            let result = validate_status_and_drift::<netbox_client::Prefix, _, _>(
                Some(&status),
                "NetBoxPrefix",
                "default",
                "test-prefix",
                |_| async { client_ref.get_prefix(netbox_client::PrefixId(1)).await },
            ).await;
            
            assert!(result.is_ok());
            match result.unwrap() {
                DriftCheckResult::Recreate => {}
                _ => panic!("Should return Recreate when no netbox_id"),
            }
        }
        
        #[tokio::test]
        async fn test_validate_status_and_drift_failed_with_valid_id_exists() {
            // Failed state with valid ID, resource exists - should update to Created
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            let test_prefix = create_test_prefix(1, "192.168.1.0/24", "http://test-netbox");
            mock_client.add_prefix(test_prefix.clone());
            
            let status = NetBoxPrefixStatus {
                netbox_id: Some(1),
                netbox_url: Some("http://test-netbox/api/ipam/prefixes/1/".to_string()),
                state: PrefixState::Failed,
                error: Some("Previous error".to_string()),
                last_reconciled: None,
            };
            
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            let result = validate_status_and_drift::<netbox_client::Prefix, _, _>(
                Some(&status),
                "NetBoxPrefix",
                "default",
                "test-prefix",
                |id| async move { client_ref.get_prefix(netbox_client::PrefixId(id)).await },
            ).await;
            
            assert!(result.is_ok());
            match result.unwrap() {
                DriftCheckResult::UseExisting(prefix) => {
                    assert_eq!(prefix.id, 1);
                }
                _ => panic!("Should return UseExisting when resource exists with Failed state"),
            }
        }
        
        #[tokio::test]
        async fn test_validate_status_and_drift_failed_with_invalid_id_zero() {
            // Failed state with invalid ID (0) - should clear status
            // Note: The function doesn't call get_resource_fn when ID is 0, it returns StatusCleared immediately
            let _mock_client = MockNetBoxClient::new("http://test-netbox");
            
            let status = NetBoxPrefixStatus {
                netbox_id: Some(0),
                netbox_url: Some("http://test-netbox/api/ipam/prefixes/0/".to_string()),
                state: PrefixState::Failed,
                error: Some("Previous error".to_string()),
                last_reconciled: None,
            };
            
            // Create a closure that will never be called (since ID is 0)
            let result = validate_status_and_drift::<netbox_client::Prefix, _, _>(
                Some(&status),
                "NetBoxPrefix",
                "default",
                "test-prefix",
                |_| async { 
                    // This should never be called when ID is 0
                    panic!("get_resource_fn should not be called when ID is 0");
                },
            ).await;
            
            assert!(result.is_ok());
            match result.unwrap() {
                DriftCheckResult::StatusCleared { message } => {
                    assert!(message.contains("Failed state with invalid netbox_id (0)") || 
                            message.contains("Clearing Failed status with invalid netbox_id (0)"));
                }
                _ => panic!("Should return StatusCleared when Failed state has ID 0"),
            }
        }
        
        #[tokio::test]
        async fn test_validate_status_and_drift_pending_state() {
            // Pending state - should recreate
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            
            let status = NetBoxPrefixStatus {
                netbox_id: Some(1),
                netbox_url: Some("http://test-netbox/api/ipam/prefixes/1/".to_string()),
                state: PrefixState::Pending,
                error: None,
                last_reconciled: None,
            };
            
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            let result = validate_status_and_drift::<netbox_client::Prefix, _, _>(
                Some(&status),
                "NetBoxPrefix",
                "default",
                "test-prefix",
                |_| async { client_ref.get_prefix(netbox_client::PrefixId(1)).await },
            ).await;
            
            assert!(result.is_ok());
            match result.unwrap() {
                DriftCheckResult::Recreate => {}
                _ => panic!("Should return Recreate for Pending state"),
            }
        }
    }


    // Tests for check_existing and check_and_update_existing
    mod check_existing_tests {
        use super::*;
        use crate::reconcile_helpers::{check_existing, check_and_update_existing};
        use netbox_client::MockNetBoxClient;
        use netbox_client::{NetBoxClientTrait, NetBoxError};
        use crate::reconcile_helpers::NetBoxResource;

        // Implement NetBoxResource for a simple test type
        struct TestResource {
            id: u64,
            name: String,
            url: String, // Store URL to return a reference
        }

        impl NetBoxResource for TestResource {
            fn id(&self) -> u64 { self.id }
            fn url(&self) -> &str { &self.url }
        }

        impl Clone for TestResource {
            fn clone(&self) -> Self {
                Self {
                    id: self.id,
                    name: self.name.clone(),
                    url: self.url.clone(),
                }
            }
        }

        #[tokio::test]
        async fn test_check_existing_resource_exists() {
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            
            // Create a test resource
            let resource = TestResource {
                id: 1,
                name: "test-resource".to_string(),
                url: "http://test/api/resource/1/".to_string(),
            };
            
            let result = check_existing(
                client_ref,
                1,
                "test-resource",
                async { Ok(resource.clone()) },
            ).await;
            
            assert!(result.is_ok());
            let existing = result.unwrap();
            assert!(existing.is_some());
            assert_eq!(existing.unwrap().id(), 1);
        }

        #[tokio::test]
        async fn test_check_existing_resource_not_found() {
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            
            let result: Result<Option<TestResource>, _> = check_existing(
                client_ref,
                1,
                "test-resource",
                async { Err::<TestResource, _>(NetBoxError::NotFound("Resource not found".to_string())) },
            ).await;
            
            assert!(result.is_ok());
            let existing = result.unwrap();
            assert!(existing.is_none(), "Should return None when resource not found (drift detected)");
        }

        #[tokio::test]
        async fn test_check_existing_other_error() {
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            
            let result: Result<Option<TestResource>, _> = check_existing(
                client_ref,
                1,
                "test-resource",
                async { Err::<TestResource, _>(NetBoxError::Api("Network error".to_string())) },
            ).await;
            
            assert!(result.is_err(), "Should return error for non-NotFound errors");
        }

        #[tokio::test]
        async fn test_check_and_update_existing_no_update_needed() {
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            
            let resource = TestResource {
                id: 1,
                name: "test-resource".to_string(),
                url: "http://test/api/resource/1/".to_string(),
            };
            
            let result = check_and_update_existing(
                client_ref,
                1,
                "test-resource",
                async { Ok(resource.clone()) },
                |_| false, // No update needed
                async { Ok(resource.clone()) },
            ).await;
            
            assert!(result.is_ok());
            let existing = result.unwrap();
            assert!(existing.is_some());
            assert_eq!(existing.unwrap().id(), 1);
        }

        #[tokio::test]
        async fn test_check_and_update_existing_update_needed() {
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            
            let resource = TestResource {
                id: 1,
                name: "test-resource".to_string(),
                url: "http://test/api/resource/1/".to_string(),
            };
            
            let updated_resource = TestResource {
                id: 1,
                name: "updated-resource".to_string(),
                url: "http://test/api/resource/1/".to_string(),
            };
            
            let result = check_and_update_existing(
                client_ref,
                1,
                "test-resource",
                async { Ok(resource.clone()) },
                |_| true, // Update needed
                async { Ok(updated_resource.clone()) },
            ).await;
            
            assert!(result.is_ok());
            let existing = result.unwrap();
            assert!(existing.is_some());
            assert_eq!(existing.unwrap().name, "updated-resource");
        }

        #[tokio::test]
        async fn test_check_and_update_existing_not_found() {
            let mock_client = MockNetBoxClient::new("http://test-netbox");
            let client_ref: &dyn NetBoxClientTrait = &mock_client;
            
            let result = check_and_update_existing(
                client_ref,
                1,
                "test-resource",
                async { Err(NetBoxError::NotFound("Resource not found".to_string())) },
                |_| false,
                async { Ok(TestResource { id: 1, name: "test".to_string(), url: "http://test/api/resource/1/".to_string() }) },
            ).await;
            
            assert!(result.is_ok());
            let existing = result.unwrap();
            assert!(existing.is_none(), "Should return None when resource not found (drift detected)");
        }
    }

    // Tests for resolve_required_dependency_id
    mod resolve_required_dependency_id_tests {
        use super::*;
        use crate::reconcile_helpers::resolve_required_dependency_id;
        use crate::kube_api_trait::mock::MockKubeApi;
        use crate::test_utils::create_test_netbox_tenant;

        #[tokio::test]
        async fn test_resolve_required_dependency_id_success() {
            let mock_api = MockKubeApi::new();
            let tenant = create_test_netbox_tenant("test-tenant", "default", Some(42), Some("http://netbox/api/tenancy/tenants/42/".to_string()));
            mock_api.store("test-tenant".to_string(), tenant);

            let result = resolve_required_dependency_id(
                &mock_api,
                "test-tenant",
                "NetBoxTenant",
                "test-resource",
                |crd| crd.status.as_ref(),
            ).await;

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 42);
        }

        #[tokio::test]
        async fn test_resolve_required_dependency_id_not_found() {
            let mock_api = MockKubeApi::<NetBoxTenant>::new();

            let result = resolve_required_dependency_id(
                &mock_api,
                "missing-tenant",
                "NetBoxTenant",
                "test-resource",
                |crd| crd.status.as_ref(),
            ).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            match error {
                crate::error::ControllerError::InvalidConfig(msg) => {
                    assert!(msg.contains("not found"));
                }
                _ => panic!("Expected InvalidConfig error"),
            }
        }

        #[tokio::test]
        async fn test_resolve_required_dependency_id_no_status() {
            let mock_api = MockKubeApi::new();
            let mut tenant = create_test_netbox_tenant("test-tenant", "default", None, None);
            tenant.status = None;
            mock_api.store("test-tenant".to_string(), tenant);

            let result = resolve_required_dependency_id(
                &mock_api,
                "test-tenant",
                "NetBoxTenant",
                "test-resource",
                |crd| crd.status.as_ref(),
            ).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            match error {
                crate::error::ControllerError::InvalidConfig(msg) => {
                    assert!(msg.contains("no status"));
                }
                _ => panic!("Expected InvalidConfig error"),
            }
        }

        #[tokio::test]
        async fn test_resolve_required_dependency_id_no_netbox_id() {
            let mock_api = MockKubeApi::new();
            // Create tenant with status but no netbox_id
            let mut tenant = create_test_netbox_tenant("test-tenant", "default", Some(1), None);
            tenant.status = Some(crds::NetBoxTenantStatus {
                netbox_id: None, // Status exists but no netbox_id
                netbox_url: None,
                state: crds::ResourceState::Pending,
                error: None,
                last_reconciled: None,
            });
            mock_api.store("test-tenant".to_string(), tenant);

            let result = resolve_required_dependency_id(
                &mock_api,
                "test-tenant",
                "NetBoxTenant",
                "test-resource",
                |crd| crd.status.as_ref(),
            ).await;

            assert!(result.is_err());
            let error = result.unwrap_err();
            match error {
                crate::error::ControllerError::InvalidConfig(msg) => {
                    assert!(msg.contains("not been created in NetBox yet") || msg.contains("no netbox_id in status"));
                }
                _ => panic!("Expected InvalidConfig error, got: {:?}", error),
            }
        }
    }

    // Tests for resolve_optional_dependency_id
    mod resolve_optional_dependency_id_tests {
        use super::*;
        use crate::reconcile_helpers::resolve_optional_dependency_id;
        use crate::kube_api_trait::mock::MockKubeApi;
        use crate::test_utils::create_test_netbox_tenant;

        #[tokio::test]
        async fn test_resolve_optional_dependency_id_success() {
            let mock_api = MockKubeApi::new();
            let tenant = create_test_netbox_tenant("test-tenant", "default", Some(42), Some("http://netbox/api/tenancy/tenants/42/".to_string()));
            mock_api.store("test-tenant".to_string(), tenant);

            let reference = Some(crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTenant".to_string(),
                name: "test-tenant".to_string(),
                namespace: None,
            });

            let result = resolve_optional_dependency_id(
                &mock_api,
                reference.as_ref(),
                "NetBoxTenant",
                "tenant",
                "test-resource",
                |crd| crd.status.as_ref(),
            ).await;

            assert_eq!(result, Some(42));
        }

        #[tokio::test]
        async fn test_resolve_optional_dependency_id_none_reference() {
            let mock_api = MockKubeApi::<NetBoxTenant>::new();

            let result = resolve_optional_dependency_id(
                &mock_api,
                None,
                "NetBoxTenant",
                "tenant",
                "test-resource",
                |crd| crd.status.as_ref(),
            ).await;

            assert_eq!(result, None);
        }

        #[tokio::test]
        async fn test_resolve_optional_dependency_id_wrong_kind() {
            let mock_api = MockKubeApi::<NetBoxTenant>::new();

            let reference = Some(crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxSite".to_string(), // Wrong kind
                name: "test-tenant".to_string(),
                namespace: None,
            });

            let result = resolve_optional_dependency_id(
                &mock_api,
                reference.as_ref(),
                "NetBoxTenant",
                "tenant",
                "test-resource",
                |crd| crd.status.as_ref(),
            ).await;

            assert_eq!(result, None);
        }

        #[tokio::test]
        async fn test_resolve_optional_dependency_id_not_found() {
            let mock_api = MockKubeApi::<NetBoxTenant>::new();

            let reference = Some(crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTenant".to_string(),
                name: "missing-tenant".to_string(),
                namespace: None,
            });

            let result = resolve_optional_dependency_id(
                &mock_api,
                reference.as_ref(),
                "NetBoxTenant",
                "tenant",
                "test-resource",
                |crd| crd.status.as_ref(),
            ).await;

            assert_eq!(result, None);
        }

        #[tokio::test]
        async fn test_resolve_optional_dependency_id_invalid_id_zero() {
            let mock_api = MockKubeApi::new();
            let tenant = create_test_netbox_tenant("test-tenant", "default", Some(0), Some("http://netbox/api/tenancy/tenants/0/".to_string()));
            mock_api.store("test-tenant".to_string(), tenant);

            let reference = Some(crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTenant".to_string(),
                name: "test-tenant".to_string(),
                namespace: None,
            });

            let result = resolve_optional_dependency_id(
                &mock_api,
                reference.as_ref(),
                "NetBoxTenant",
                "tenant",
                "test-resource",
                |crd| crd.status.as_ref(),
            ).await;

            assert_eq!(result, None, "Should filter out invalid ID 0");
        }
    }

    // Tests for update_resource_status
    mod update_resource_status_tests {
        use super::*;
        use crate::reconcile_helpers::update_resource_status;
        use crate::kube_api_trait::mock::MockKubeApi;
        use crate::test_utils::create_test_netbox_tenant;

        #[tokio::test]
        async fn test_update_resource_status_success() {
            let mock_api = MockKubeApi::new();
            let tenant = create_test_netbox_tenant("test-tenant", "default", Some(42), Some("http://netbox/api/tenancy/tenants/42/".to_string()));
            mock_api.store("test-tenant".to_string(), tenant);

            let status_patch = serde_json::json!({
                "status": {
                    "netboxId": 42,
                    "netboxUrl": "http://netbox/api/tenancy/tenants/42/",
                    "state": "Created",
                    "error": null
                }
            });

            let result = update_resource_status(
                &mock_api,
                "test-tenant",
                "default",
                &status_patch,
                "NetBoxTenant",
                42,
            ).await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_update_resource_status_with_zero_id() {
            let mock_api = MockKubeApi::new();
            let tenant = create_test_netbox_tenant("test-tenant", "default", None, None);
            mock_api.store("test-tenant".to_string(), tenant);

            let status_patch = serde_json::json!({
                "status": {
                    "netboxId": 0,
                    "netboxUrl": "",
                    "state": "Pending",
                    "error": "Resource was deleted"
                }
            });

            let result = update_resource_status(
                &mock_api,
                "test-tenant",
                "default",
                &status_patch,
                "NetBoxTenant",
                0,
            ).await;

            assert!(result.is_ok());
        }

        // Note: Testing patch_status errors requires a mock that can return errors
        // MockKubeApi currently always succeeds. Error paths are tested in integration tests.
    }

    // Additional tests for edge cases and error paths
    mod edge_case_tests {
        use super::*;
        use crate::reconcile_helpers::{check_and_update_existing, check_existing};
        use netbox_client::{MockNetBoxClient, NetBoxClientTrait};

        #[tokio::test]
        async fn test_check_and_update_existing_update_error() {
            let mut mock_client = MockNetBoxClient::new("http://test-netbox".to_string());
            
            // Add a site that exists
            let existing_site = netbox_client::Site {
                id: 1,
                url: "http://test-netbox/api/dcim/sites/1/".to_string(),
                display: "test-site".to_string(),
                name: "test-site".to_string(),
                slug: "test-site".to_string(),
                status: netbox_client::SiteStatus::Active,
                region: None,
                group: None,
                tenant: None,
                facility: None,
                time_zone: None,
                description: Some("Old description".to_string()),
                physical_address: None,
                shipping_address: None,
                latitude: None,
                longitude: None,
                contact_name: None,
                contact_phone: None,
                contact_email: None,
                comments: None,
                tags: vec![],
                custom_fields: std::collections::HashMap::new(),
                created: "2024-01-01T00:00:00Z".to_string(),
                last_updated: "2024-01-01T00:00:00Z".to_string(),
            };
            mock_client.add_site(existing_site.clone());
            
            // Configure update to fail
            mock_client.set_update_site_error(netbox_client::NetBoxError::Api("Update failed".to_string()));

            let result = check_and_update_existing(
                &mock_client,
                1,
                "site",
                async { mock_client.get_site(netbox_client::SiteId(1)).await },
                |_| true, // Always needs update
                async { mock_client.update_site(netbox_client::SiteId(1), &netbox_client::SiteUpdateRequest {
                    name: Some("test-site".to_string()),
                    description: Some("New description".to_string()),
                    ..Default::default()
                }).await },
            ).await;

            assert!(result.is_err());
            if let Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(_))) = result {
                // Expected error type
            } else {
                panic!("Expected NetBox API error");
            }
        }

        #[tokio::test]
        async fn test_check_existing_network_error() {
            let mut mock_client = MockNetBoxClient::new("http://test-netbox".to_string());
            
            // Configure get to return network error (not NotFound)
            mock_client.set_get_site_error(netbox_client::NetBoxError::Api("Network error".to_string()));

            let result = check_existing(
                &mock_client,
                1,
                "site",
                async { mock_client.get_site(netbox_client::SiteId(1)).await },
            ).await;

            assert!(result.is_err());
            if let Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(_))) = result {
                // Expected error type - should retry, not assume deleted
            } else {
                panic!("Expected NetBox error for retry");
            }
        }

        #[tokio::test]
        async fn test_validate_status_and_drift_updated_state() {
            use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
            use netbox_client::{MockNetBoxClient, NetBoxClientTrait};
            
            let mock_client = MockNetBoxClient::new("http://test-netbox".to_string());
            let status = crds::NetBoxSiteStatus {
                netbox_id: Some(1),
                netbox_url: Some("http://test-netbox/api/dcim/sites/1/".to_string()),
                state: crds::ResourceState::Updated, // Updated state
                error: None,
                last_reconciled: None,
            };

            let result = validate_status_and_drift(
                Some(&status),
                "NetBoxSite",
                "default",
                "test-site",
                |id| {
                    let client = &mock_client;
                    async move { client.get_site(netbox_client::SiteId(id)).await }
                },
            ).await;

            // Updated state should return Recreate (not handled specially)
            assert!(matches!(result, Ok(DriftCheckResult::Recreate)));
        }

        #[tokio::test]
        async fn test_validate_status_and_drift_failed_state_network_error() {
            use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
            use netbox_client::{MockNetBoxClient, NetBoxClientTrait};
            
            let mut mock_client = MockNetBoxClient::new("http://test-netbox".to_string());
            // Configure to return network error (not NotFound)
            mock_client.set_get_site_error(netbox_client::NetBoxError::Api("Network error".to_string()));
            
            let status = crds::NetBoxSiteStatus {
                netbox_id: Some(1),
                netbox_url: Some("http://test-netbox/api/dcim/sites/1/".to_string()),
                state: crds::ResourceState::Failed,
                error: Some("Previous error".to_string()),
                last_reconciled: None,
            };

            let result = validate_status_and_drift(
                Some(&status),
                "NetBoxSite",
                "default",
                "test-site",
                |id| {
                    let client = &mock_client;
                    async move { client.get_site(netbox_client::SiteId(id)).await }
                },
            ).await;

            // Network error should be propagated (retry)
            assert!(result.is_err());
        }
    }
}
