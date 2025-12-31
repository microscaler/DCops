//! Unit tests for NetBoxRIR reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::kube_api_trait::KubeApiTrait;
    use crds::{NetBoxRIR, ResourceState};
    use std::sync::Arc;
    use chrono::Utc;

    /// Helper to create test NetBoxRIR CRD
    fn create_test_netbox_rir(
        name: &str,
        namespace: &str,
        netbox_id: Option<u64>,
    ) -> NetBoxRIR {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        
        NetBoxRIR {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxRIRSpec {
                name: name.to_string(),
                slug: Some(name.to_string()),
                description: None,
                is_private: Some(false),
                tags: None,
            },
            status: netbox_id.map(|id| crds::NetBoxRIRStatus {
                netbox_id: Some(id),
                netbox_url: Some(format!("http://test-netbox/api/ipam/rirs/{}/", id)),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        }
    }

    /// Helper to create test NetBox RIR model
    fn create_test_rir(
        id: u64,
        name: &str,
        base_url: &str,
    ) -> netbox_client::Rir {
        netbox_client::Rir {
            id,
            url: format!("{}/api/ipam/rirs/{}/", base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: name.to_string(),
            description: None,
            is_private: false,
            tags: vec![],
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_reconcile_rir_create() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create RIR CRD without status
        let mut rir = create_test_netbox_rir("ARIN", "default", None);
        rir.status = None;
        apis.rir_api.store("ARIN".to_string(), rir.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_rir(&rir).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.rir_api.as_ref().get("ARIN").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }

    #[tokio::test]
    async fn test_reconcile_rir_idempotent() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Add RIR to mock NetBox client
        let netbox_rir = create_test_rir(1, "ARIN", "http://test-netbox");
        mock_token_resolver.mock_client().add_rir(netbox_rir);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create RIR CRD with status (already created)
        let rir = create_test_netbox_rir("ARIN", "default", Some(1));
        apis.rir_api.store("ARIN".to_string(), rir.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_rir(&rir).await;
        
        // Assert: Should succeed (idempotent)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain unchanged
        let updated_crd = apis.rir_api.as_ref().get("ARIN").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should remain 1");
    }

    #[tokio::test]
    async fn test_reconcile_rir_conflict_handling() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Add RIR to mock NetBox client (simulates conflict - RIR already exists)
        let netbox_rir = create_test_rir(1, "ARIN", "http://test-netbox");
        mock_token_resolver.mock_client().add_rir(netbox_rir);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create RIR CRD without status (will try to create, get conflict, then find existing)
        let mut rir = create_test_netbox_rir("ARIN", "default", None);
        rir.status = None;
        apis.rir_api.store("ARIN".to_string(), rir.clone());
        
        // Execute: Reconcile (should handle conflict gracefully)
        let result = reconciler.reconcile_netbox_rir(&rir).await;
        
        // Assert: Should succeed (conflict handled by finding existing RIR)
        assert!(result.is_ok(), "Reconciliation should succeed after conflict: {:?}", result.err());
        
        // Assert: Status should be updated with existing NetBox ID
        let updated_crd = apis.rir_api.as_ref().get("ARIN").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should be set to existing RIR ID");
    }

    // ========== Tag Tests ==========
    
    #[tokio::test]
    async fn test_reconcile_rir_with_tags_create() {
        use crate::test_utils::{create_test_netbox_tag, create_test_nested_tag};
        
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        let mock_client = mock_token_resolver.mock_client();
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create RIR CRD with tags
        let mut rir = create_test_netbox_rir("ARIN", "default", None);
        rir.status = None;
        rir.spec.tags = Some(vec![
            crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTag".to_string(),
                name: "production".to_string(),
                namespace: Some("default".to_string()),
            },
        ]);
        apis.rir_api.store("ARIN".to_string(), rir.clone());
        
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
        let result = reconciler.reconcile_netbox_rir(&rir).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.rir_api.as_ref().get("ARIN").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_rir_with_tags_update() {
        use crate::test_utils::{create_test_netbox_tag, create_test_nested_tag};
        
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Add RIR with different tags to mock NetBox
        let mut netbox_rir = create_test_rir(1, "ARIN", "http://test-netbox");
        netbox_rir.tags = vec![create_test_nested_tag(20, "old-tag", "http://test-netbox")];
        mock_client.add_rir(netbox_rir);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create RIR CRD with status and new tags
        let mut rir = create_test_netbox_rir("ARIN", "default", Some(1));
        rir.spec.tags = Some(vec![
            crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "NetBoxTag".to_string(),
                name: "production".to_string(),
                namespace: Some("default".to_string()),
            },
        ]);
        apis.rir_api.store("ARIN".to_string(), rir.clone());
        
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
        let result = reconciler.reconcile_netbox_rir(&rir).await;
        
        // Assert: Should succeed (tags differ, so update should be triggered)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
    }
}

