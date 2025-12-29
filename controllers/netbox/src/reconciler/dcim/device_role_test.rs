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
}

