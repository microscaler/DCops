//! Unit tests for NetBoxManufacturer reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::kube_api_trait::KubeApiTrait;
    use crds::{NetBoxManufacturer, ResourceState};
    use std::sync::Arc;
    use chrono::Utc;

    /// Helper to create test NetBoxManufacturer CRD
    fn create_test_netbox_manufacturer(
        name: &str,
        namespace: &str,
        netbox_id: Option<u64>,
    ) -> NetBoxManufacturer {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        
        NetBoxManufacturer {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxManufacturerSpec {
                name: name.to_string(),
                slug: Some(name.to_string()),
                description: None,
                tags: None,
            },
            status: netbox_id.map(|id| crds::NetBoxManufacturerStatus {
                netbox_id: Some(id),
                netbox_url: Some(format!("http://test-netbox/api/dcim/manufacturers/{}/", id)),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        }
    }

    /// Helper to create test NetBox Manufacturer model
    fn create_test_manufacturer(
        id: u64,
        name: &str,
        base_url: &str,
    ) -> netbox_client::Manufacturer {
        netbox_client::Manufacturer {
            id,
            url: format!("{}/api/dcim/manufacturers/{}/", base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: name.to_string(),
            description: None,
            devicetype_count: 0,
            inventoryitem_count: 0,
            platform_count: 0,
            tags: vec![],
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_reconcile_manufacturer_create() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create manufacturer CRD without status
        let mut manufacturer = create_test_netbox_manufacturer("test-manufacturer", "default", None);
        manufacturer.status = None;
        apis.manufacturer_api.store("test-manufacturer".to_string(), manufacturer.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_manufacturer(&manufacturer).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.manufacturer_api.as_ref().get("test-manufacturer").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }

    #[tokio::test]
    async fn test_reconcile_manufacturer_idempotent() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Add manufacturer to mock NetBox client
        let netbox_manufacturer = create_test_manufacturer(1, "test-manufacturer", "http://test-netbox");
        mock_token_resolver.mock_client().add_manufacturer(netbox_manufacturer);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create manufacturer CRD with status (already created)
        let manufacturer = create_test_netbox_manufacturer("test-manufacturer", "default", Some(1));
        apis.manufacturer_api.store("test-manufacturer".to_string(), manufacturer.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_manufacturer(&manufacturer).await;
        
        // Assert: Should succeed (idempotent)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain unchanged
        let updated_crd = apis.manufacturer_api.as_ref().get("test-manufacturer").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should remain 1");
    }

    #[tokio::test]
    async fn test_reconcile_manufacturer_conflict_handling() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Add manufacturer to mock NetBox client (simulates conflict - manufacturer already exists)
        let netbox_manufacturer = create_test_manufacturer(1, "test-manufacturer", "http://test-netbox");
        mock_token_resolver.mock_client().add_manufacturer(netbox_manufacturer);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create manufacturer CRD without status (will try to create, get conflict, then find existing)
        let mut manufacturer = create_test_netbox_manufacturer("test-manufacturer", "default", None);
        manufacturer.status = None;
        apis.manufacturer_api.store("test-manufacturer".to_string(), manufacturer.clone());
        
        // Execute: Reconcile (should handle conflict gracefully)
        let result = reconciler.reconcile_netbox_manufacturer(&manufacturer).await;
        
        // Assert: Should succeed (conflict handled by finding existing manufacturer)
        assert!(result.is_ok(), "Reconciliation should succeed after conflict: {:?}", result.err());
        
        // Assert: Status should be updated with existing NetBox ID
        let updated_crd = apis.manufacturer_api.as_ref().get("test-manufacturer").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should be set to existing manufacturer ID");
    }

    // ========== Tag Tests ==========
    
    #[tokio::test]
    async fn test_reconcile_manufacturer_with_tags_create() {
        use crate::test_utils::{create_test_netbox_tag, create_test_nested_tag};
        
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        let mock_client = mock_token_resolver.mock_client();
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create manufacturer CRD with tags
        let mut manufacturer = create_test_netbox_manufacturer("test-manufacturer", "default", None);
        manufacturer.status = None;
        manufacturer.spec.tags = Some(vec![
            crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTag".to_string(),
                name: "production".to_string(),
                namespace: Some("default".to_string()),
            },
        ]);
        apis.manufacturer_api.store("test-manufacturer".to_string(), manufacturer.clone());
        
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
        let result = reconciler.reconcile_netbox_manufacturer(&manufacturer).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.manufacturer_api.as_ref().get("test-manufacturer").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_manufacturer_with_tags_update() {
        use crate::test_utils::{create_test_netbox_tag, create_test_nested_tag};
        
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Add manufacturer with different tags to mock NetBox
        let mut netbox_manufacturer = create_test_manufacturer(1, "test-manufacturer", "http://test-netbox");
        netbox_manufacturer.tags = vec![create_test_nested_tag(20, "old-tag", "http://test-netbox")];
        mock_client.add_manufacturer(netbox_manufacturer);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create manufacturer CRD with status and new tags
        let mut manufacturer = create_test_netbox_manufacturer("test-manufacturer", "default", Some(1));
        manufacturer.spec.tags = Some(vec![
            crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTag".to_string(),
                name: "production".to_string(),
                namespace: Some("default".to_string()),
            },
        ]);
        apis.manufacturer_api.store("test-manufacturer".to_string(), manufacturer.clone());
        
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
        let result = reconciler.reconcile_netbox_manufacturer(&manufacturer).await;
        
        // Assert: Should succeed (tags differ, so update should be triggered)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
    }
}

