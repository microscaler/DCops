//! Unit tests for NetBoxDeviceType reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::kube_api_trait::KubeApiTrait;
    use crds::{NetBoxDeviceType, ResourceState};
    use std::sync::Arc;
    use chrono::Utc;

    /// Helper to create test NetBoxDeviceType CRD
    fn create_test_netbox_device_type(
        name: &str,
        namespace: &str,
        manufacturer_name: &str,
        model: &str,
        netbox_id: Option<u64>,
    ) -> NetBoxDeviceType {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        
        NetBoxDeviceType {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxDeviceTypeSpec {
                manufacturer: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxManufacturer".to_string(),
                    name: manufacturer_name.to_string(),
                    namespace: Some(namespace.to_string()),
                },
                model: model.to_string(),
                slug: Some(model.to_string()),
                part_number: None,
                u_height: 1.0,
                is_full_depth: false,
                description: None,
                comments: None,
                tags: None,
            },
            status: netbox_id.map(|id| crds::NetBoxDeviceTypeStatus {
                netbox_id: Some(id),
                netbox_url: Some(format!("http://test-netbox/api/dcim/device-types/{}/", id)),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        }
    }

    /// Helper to create test NetBox DeviceType model
    fn create_test_device_type(
        id: u64,
        manufacturer_id: u64,
        model: &str,
        base_url: &str,
    ) -> netbox_client::DeviceType {
        use netbox_client::NestedManufacturer;
        
        netbox_client::DeviceType {
            id,
            url: format!("{}/api/dcim/device-types/{}/", base_url, id),
            display: model.to_string(),
            manufacturer: NestedManufacturer {
                id: manufacturer_id,
                url: format!("{}/api/dcim/manufacturers/{}/", base_url, manufacturer_id),
                display: "Test Manufacturer".to_string(),
                name: "test-manufacturer".to_string(),
                slug: "test-manufacturer".to_string(),
            },
            model: model.to_string(),
            slug: model.to_string(),
            part_number: None,
            u_height: 1.0,
            is_full_depth: false,
            description: None,
            comments: None,
            device_count: 0,
            tags: vec![],
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_reconcile_device_type_create() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Add manufacturer to mock NetBox client
        let manufacturer = crate::test_utils::create_test_manufacturer(1, "test-manufacturer", "http://test-netbox");
        mock_token_resolver.mock_client().add_manufacturer(manufacturer);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver.clone());
        
        // Setup: Create manufacturer CRD (required dependency)
        let manufacturer_crd = crate::test_utils::create_test_netbox_manufacturer("test-manufacturer", "default", Some(1));
        apis.manufacturer_api.store("test-manufacturer".to_string(), manufacturer_crd);
        
        // Setup: Create device type CRD without status
        let mut device_type = create_test_netbox_device_type("test-device-type", "default", "test-manufacturer", "Test Model", None);
        device_type.status = None;
        apis.device_type_api.store("test-device-type".to_string(), device_type.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_device_type(&device_type).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.device_type_api.as_ref().get("test-device-type").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }

    #[tokio::test]
    async fn test_reconcile_device_type_idempotent() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Add manufacturer to mock NetBox client
        let manufacturer = crate::test_utils::create_test_manufacturer(1, "test-manufacturer", "http://test-netbox");
        mock_token_resolver.mock_client().add_manufacturer(manufacturer);
        
        // Setup: Add device type to mock NetBox client
        let netbox_device_type = create_test_device_type(1, 1, "Test Model", "http://test-netbox");
        mock_token_resolver.mock_client().add_device_type(netbox_device_type);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver.clone());
        
        // Setup: Create manufacturer CRD (required dependency)
        let manufacturer_crd = crate::test_utils::create_test_netbox_manufacturer("test-manufacturer", "default", Some(1));
        apis.manufacturer_api.store("test-manufacturer".to_string(), manufacturer_crd);
        
        // Setup: Create device type CRD with status (already created)
        let device_type = create_test_netbox_device_type("test-device-type", "default", "test-manufacturer", "Test Model", Some(1));
        apis.device_type_api.store("test-device-type".to_string(), device_type.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_device_type(&device_type).await;
        
        // Assert: Should succeed (idempotent)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain unchanged
        let updated_crd = apis.device_type_api.as_ref().get("test-device-type").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should remain 1");
    }

    #[tokio::test]
    async fn test_reconcile_device_type_manufacturer_not_found() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver.clone());
        
        // Setup: Create device type CRD with manufacturer that doesn't exist
        let mut device_type = create_test_netbox_device_type("test-device-type", "default", "nonexistent-manufacturer", "Test Model", None);
        device_type.status = None;
        apis.device_type_api.store("test-device-type".to_string(), device_type.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_device_type(&device_type).await;
        
        // Assert: Should fail with InvalidConfig error (manufacturer not found)
        assert!(result.is_err(), "Reconciliation should fail when manufacturer not found");
        match result.unwrap_err() {
            crate::error::ControllerError::InvalidConfig(_) => {
                // Expected error type
            }
            e => panic!("Expected InvalidConfig error, got: {:?}", e),
        }
    }

    // ========== Tag Tests ==========
    
    #[tokio::test]
    async fn test_reconcile_device_type_with_tags_create() {
        use crate::test_utils::{create_test_netbox_tag, create_test_nested_tag};
        
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Add manufacturer to mock NetBox client
        let manufacturer = crate::test_utils::create_test_manufacturer(1, "test-manufacturer", "http://test-netbox");
        mock_client.add_manufacturer(manufacturer);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver.clone());
        
        // Setup: Create manufacturer CRD (required dependency)
        let manufacturer_crd = crate::test_utils::create_test_netbox_manufacturer("test-manufacturer", "default", Some(1));
        apis.manufacturer_api.store("test-manufacturer".to_string(), manufacturer_crd);
        
        // Setup: Create device type CRD with tags
        let mut device_type = create_test_netbox_device_type("test-device-type", "default", "test-manufacturer", "Test Model", None);
        device_type.status = None;
        device_type.spec.tags = Some(vec![
            crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTag".to_string(),
                name: "production".to_string(),
                namespace: Some("default".to_string()),
            },
        ]);
        apis.device_type_api.store("test-device-type".to_string(), device_type.clone());
        
        // Setup: Create tag CRDs with status
        let tag1 = create_test_netbox_tag("production", "default", Some(10));
        apis.tag_api.store("production".to_string(), tag1);
        
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
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_device_type(&device_type).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.device_type_api.as_ref().get("test-device-type").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_device_type_with_tags_update() {
        use crate::test_utils::{create_test_netbox_tag, create_test_nested_tag};
        
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Add manufacturer to mock NetBox client
        let manufacturer = crate::test_utils::create_test_manufacturer(1, "test-manufacturer", "http://test-netbox");
        mock_client.add_manufacturer(manufacturer);
        
        // Setup: Add device type with different tags to mock NetBox
        let mut netbox_device_type = create_test_device_type(1, 1, "Test Model", "http://test-netbox");
        netbox_device_type.tags = vec![create_test_nested_tag(20, "old-tag", "http://test-netbox")];
        mock_client.add_device_type(netbox_device_type);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver.clone());
        
        // Setup: Create manufacturer CRD (required dependency)
        let manufacturer_crd = crate::test_utils::create_test_netbox_manufacturer("test-manufacturer", "default", Some(1));
        apis.manufacturer_api.store("test-manufacturer".to_string(), manufacturer_crd);
        
        // Setup: Create device type CRD with status and new tags
        let mut device_type = create_test_netbox_device_type("test-device-type", "default", "test-manufacturer", "Test Model", Some(1));
        device_type.spec.tags = Some(vec![
            crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTag".to_string(),
                name: "production".to_string(),
                namespace: Some("default".to_string()),
            },
        ]);
        apis.device_type_api.store("test-device-type".to_string(), device_type.clone());
        
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
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_device_type(&device_type).await;
        
        // Assert: Should succeed (tags differ, so update should be triggered)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
    }
}

