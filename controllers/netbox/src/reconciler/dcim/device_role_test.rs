//! Unit tests for NetBoxDeviceRole reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::kube_api_trait::KubeApiTrait;
    use crds::{NetBoxDeviceRole, ResourceState};
    use std::sync::Arc;
    use chrono::Utc;

    /// Helper to create test NetBoxDeviceRole CRD
    fn create_test_netbox_device_role(
        name: &str,
        namespace: &str,
        netbox_id: Option<u64>,
    ) -> NetBoxDeviceRole {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        
        NetBoxDeviceRole {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxDeviceRoleSpec {
                name: name.to_string(),
                slug: Some(name.to_string()),
                color: Some("9e9e9e".to_string()),
                vm_role: false,
                description: None,
                comments: None,
                tags: None,
            },
            status: netbox_id.map(|id| crds::NetBoxDeviceRoleStatus {
                netbox_id: Some(id),
                netbox_url: Some(format!("http://test-netbox/api/dcim/device-roles/{}/", id)),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        }
    }

    /// Helper to create test NetBox DeviceRole model
    fn create_test_device_role(
        id: u64,
        name: &str,
        base_url: &str,
    ) -> netbox_client::DeviceRole {
        netbox_client::DeviceRole {
            id,
            url: format!("{}/api/dcim/device-roles/{}/", base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: name.to_string(),
            color: Some("9e9e9e".to_string()),
            vm_role: false,
            description: None,
            comments: None,
            device_count: 0,
            virtualmachine_count: 0,
            tags: vec![],
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_reconcile_device_role_create() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create device role CRD without status
        let mut device_role = create_test_netbox_device_role("test-device-role", "default", None);
        device_role.status = None;
        apis.device_role_api.store("test-device-role".to_string(), device_role.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_device_role(&device_role).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.device_role_api.as_ref().get("test-device-role").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }

    #[tokio::test]
    async fn test_reconcile_device_role_idempotent() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Add device role to mock NetBox client
        let netbox_device_role = create_test_device_role(1, "test-device-role", "http://test-netbox");
        mock_token_resolver.mock_client().add_device_role(netbox_device_role);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create device role CRD with status (already created)
        let device_role = create_test_netbox_device_role("test-device-role", "default", Some(1));
        apis.device_role_api.store("test-device-role".to_string(), device_role.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_device_role(&device_role).await;
        
        // Assert: Should succeed (idempotent)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain unchanged
        let updated_crd = apis.device_role_api.as_ref().get("test-device-role").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should remain 1");
    }

    // ========== Tag Tests ==========
    
    #[tokio::test]
    async fn test_reconcile_device_role_with_tags_create() {
        use crate::test_utils::{create_test_netbox_tag, create_test_nested_tag};
        
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        let mock_client = mock_token_resolver.mock_client();
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create device role CRD with tags
        let mut device_role = create_test_netbox_device_role("test-device-role", "default", None);
        device_role.status = None;
        device_role.spec.tags = Some(vec![
            crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTag".to_string(),
                name: "production".to_string(),
                namespace: Some("default".to_string()),
            },
        ]);
        apis.device_role_api.store("test-device-role".to_string(), device_role.clone());
        
        // Setup: Create tag CRD with status
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
        let result = reconciler.reconcile_netbox_device_role(&device_role).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.device_role_api.as_ref().get("test-device-role").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_device_role_with_tags_update() {
        use crate::test_utils::{create_test_netbox_tag, create_test_nested_tag};
        
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Add device role with different tags to mock NetBox
        let mut netbox_device_role = create_test_device_role(1, "test-device-role", "http://test-netbox");
        netbox_device_role.tags = vec![create_test_nested_tag(20, "old-tag", "http://test-netbox")];
        mock_client.add_device_role(netbox_device_role);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create device role CRD with status and new tags
        let mut device_role = create_test_netbox_device_role("test-device-role", "default", Some(1));
        device_role.spec.tags = Some(vec![
            crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTag".to_string(),
                name: "production".to_string(),
                namespace: Some("default".to_string()),
            },
        ]);
        apis.device_role_api.store("test-device-role".to_string(), device_role.clone());
        
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
        let result = reconciler.reconcile_netbox_device_role(&device_role).await;
        
        // Assert: Should succeed (tags differ, so update should be triggered)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
    }
}

