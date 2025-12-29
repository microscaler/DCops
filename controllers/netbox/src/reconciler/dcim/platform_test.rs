//! Unit tests for NetBoxPlatform reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::kube_api_trait::KubeApiTrait;
    use crds::{NetBoxPlatform, NetBoxManufacturer, ResourceState};
    use std::sync::Arc;
    use chrono::Utc;

    /// Helper to create test NetBoxPlatform CRD
    fn create_test_netbox_platform(
        name: &str,
        namespace: &str,
        netbox_id: Option<u64>,
        manufacturer_ref: Option<&str>,
    ) -> NetBoxPlatform {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        
        NetBoxPlatform {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxPlatformSpec {
                name: name.to_string(),
                slug: Some(name.to_string()),
                manufacturer: manufacturer_ref.map(|m| crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxManufacturer".to_string(),
                    name: m.to_string(),
                    namespace: Some(namespace.to_string()),
                }),
                napalm_driver: None,
                napalm_args: None,
                description: None,
                comments: None,
            },
            status: netbox_id.map(|id| crds::NetBoxPlatformStatus {
                netbox_id: Some(id),
                netbox_url: Some(format!("http://test-netbox/api/dcim/platforms/{}/", id)),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        }
    }

    /// Helper to create test NetBox Platform model
    fn create_test_platform(
        id: u64,
        name: &str,
        base_url: &str,
        manufacturer_id: Option<u64>,
    ) -> netbox_client::Platform {
        use netbox_client::NestedManufacturer;
        
        netbox_client::Platform {
            id,
            url: format!("{}/api/dcim/platforms/{}/", base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: name.to_string(),
            manufacturer: manufacturer_id.map(|m_id| NestedManufacturer {
                id: m_id,
                url: format!("{}/api/dcim/manufacturers/{}/", base_url, m_id),
                display: "Test Manufacturer".to_string(),
                name: "test-manufacturer".to_string(),
                slug: "test-manufacturer".to_string(),
            }),
            napalm_driver: None,
            napalm_args: None,
            description: None,
            comments: None,
            device_count: 0,
            virtualmachine_count: 0,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_reconcile_platform_create() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create platform CRD without status
        let mut platform = create_test_netbox_platform("test-platform", "default", None, None);
        platform.status = None;
        apis.platform_api.store("test-platform".to_string(), platform.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_platform(&platform).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.platform_api.as_ref().get("test-platform").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }

    #[tokio::test]
    async fn test_reconcile_platform_idempotent() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Add platform to mock NetBox client
        let netbox_platform = create_test_platform(1, "test-platform", "http://test-netbox", None);
        mock_token_resolver.mock_client().add_platform(netbox_platform);
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create platform CRD with status (already created)
        let platform = create_test_netbox_platform("test-platform", "default", Some(1), None);
        apis.platform_api.store("test-platform".to_string(), platform.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_platform(&platform).await;
        
        // Assert: Should succeed (idempotent)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain unchanged
        let updated_crd = apis.platform_api.as_ref().get("test-platform").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should remain 1");
    }

    #[tokio::test]
    async fn test_reconcile_platform_with_manufacturer() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Add manufacturer to mock NetBox client
        let manufacturer = crate::test_utils::create_test_manufacturer(1, "test-manufacturer", "http://test-netbox");
        mock_token_resolver.mock_client().add_manufacturer(manufacturer);
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create manufacturer CRD (required dependency)
        let manufacturer_crd = crate::test_utils::create_test_netbox_manufacturer("test-manufacturer", "default", Some(1));
        apis.manufacturer_api.store("test-manufacturer".to_string(), manufacturer_crd);
        
        // Setup: Create platform CRD with manufacturer reference
        let mut platform = create_test_netbox_platform("test-platform", "default", None, Some("test-manufacturer"));
        platform.status = None;
        apis.platform_api.store("test-platform".to_string(), platform.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_platform(&platform).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated
        let updated_crd = apis.platform_api.as_ref().get("test-platform").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }

    #[tokio::test]
    async fn test_reconcile_platform_manufacturer_not_found() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create platform CRD with manufacturer that doesn't exist
        let mut platform = create_test_netbox_platform("test-platform", "default", None, Some("nonexistent-manufacturer"));
        platform.status = None;
        apis.platform_api.store("test-platform".to_string(), platform.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_platform(&platform).await;
        
        // Assert: Should succeed (manufacturer is optional, will be None if not found)
        // The reconciler uses resolve_optional_dependency_id which returns None if not found
        assert!(result.is_ok(), "Reconciliation should succeed even if manufacturer not found: {:?}", result.err());
    }
}

