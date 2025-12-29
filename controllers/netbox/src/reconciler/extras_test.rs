//! Unit tests for NetBoxRole and NetBoxTag reconcilers

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::kube_api_trait::KubeApiTrait;
    use crds::{NetBoxRole, NetBoxTag, ResourceState};
    use std::sync::Arc;
    use chrono::Utc;

    /// Helper to create test NetBoxRole CRD
    fn create_test_netbox_role(
        name: &str,
        namespace: &str,
        netbox_id: Option<u64>,
    ) -> NetBoxRole {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        
        NetBoxRole {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxRoleSpec {
                name: name.to_string(),
                slug: Some(name.to_string()),
                description: None,
                weight: None,
                comments: None,
            },
            status: netbox_id.map(|id| crds::NetBoxRoleStatus {
                netbox_id: Some(id),
                netbox_url: Some(format!("http://test-netbox/api/extras/roles/{}/", id)),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        }
    }

    /// Helper to create test NetBoxTag CRD
    fn create_test_netbox_tag(
        name: &str,
        namespace: &str,
        netbox_id: Option<u64>,
    ) -> NetBoxTag {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        
        NetBoxTag {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxTagSpec {
                name: name.to_string(),
                slug: Some(name.to_string()),
                color: Some("9e9e9e".to_string()),
                description: None,
                comments: None,
            },
            status: netbox_id.map(|id| crds::NetBoxTagStatus {
                netbox_id: Some(id),
                netbox_url: Some(format!("http://test-netbox/api/extras/tags/{}/", id)),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        }
    }

    /// Helper to create test NetBox Role model
    fn create_test_role(
        id: u64,
        name: &str,
        base_url: &str,
    ) -> netbox_client::Role {
        netbox_client::Role {
            id,
            url: format!("{}/api/extras/roles/{}/", base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: name.to_string(),
            description: None,
            weight: None,
            comments: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        }
    }

    /// Helper to create test NetBox Tag model
    fn create_test_tag(
        id: u64,
        name: &str,
        base_url: &str,
    ) -> netbox_client::Tag {
        netbox_client::Tag {
            id,
            url: format!("{}/api/extras/tags/{}/", base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: name.to_string(),
            color: "9e9e9e".to_string(),
            description: None,
            comments: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        }
    }

    // ========== NetBoxRole Tests ==========

    #[tokio::test]
    async fn test_reconcile_role_create() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create role CRD without status
        let mut role = create_test_netbox_role("test-role", "default", None);
        role.status = None;
        apis.role_api.store("test-role".to_string(), role.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_role(&role).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.role_api.as_ref().get("test-role").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }

    #[tokio::test]
    async fn test_reconcile_role_idempotent() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Add role to mock NetBox client
        let netbox_role = create_test_role(1, "test-role", "http://test-netbox");
        mock_token_resolver.mock_client().add_role(netbox_role);
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create role CRD with status (already created)
        let role = create_test_netbox_role("test-role", "default", Some(1));
        apis.role_api.store("test-role".to_string(), role.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_role(&role).await;
        
        // Assert: Should succeed (idempotent)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain unchanged
        let updated_crd = apis.role_api.as_ref().get("test-role").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should remain 1");
    }

    #[tokio::test]
    async fn test_reconcile_role_conflict_handling() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Add role to mock NetBox client (simulates conflict - role already exists)
        let netbox_role = create_test_role(1, "test-role", "http://test-netbox");
        mock_token_resolver.mock_client().add_role(netbox_role);
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create role CRD without status (will try to create, get conflict, then find existing)
        let mut role = create_test_netbox_role("test-role", "default", None);
        role.status = None;
        apis.role_api.store("test-role".to_string(), role.clone());
        
        // Execute: Reconcile (should handle conflict gracefully)
        let result = reconciler.reconcile_netbox_role(&role).await;
        
        // Assert: Should succeed (conflict handled by finding existing role)
        assert!(result.is_ok(), "Reconciliation should succeed after conflict: {:?}", result.err());
        
        // Assert: Status should be updated with existing NetBox ID
        let updated_crd = apis.role_api.as_ref().get("test-role").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should be set to existing role ID");
    }

    // ========== NetBoxTag Tests ==========

    #[tokio::test]
    async fn test_reconcile_tag_create() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create tag CRD without status
        let mut tag = create_test_netbox_tag("test-tag", "default", None);
        tag.status = None;
        apis.tag_api.store("test-tag".to_string(), tag.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_tag(&tag).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.tag_api.as_ref().get("test-tag").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }

    #[tokio::test]
    async fn test_reconcile_tag_idempotent() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Add tag to mock NetBox client
        let netbox_tag = create_test_tag(1, "test-tag", "http://test-netbox");
        mock_token_resolver.mock_client().add_tag(netbox_tag);
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create tag CRD with status (already created)
        let tag = create_test_netbox_tag("test-tag", "default", Some(1));
        apis.tag_api.store("test-tag".to_string(), tag.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_tag(&tag).await;
        
        // Assert: Should succeed (idempotent)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain unchanged
        let updated_crd = apis.tag_api.as_ref().get("test-tag").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should remain 1");
    }

    #[tokio::test]
    async fn test_reconcile_tag_conflict_handling() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Add tag to mock NetBox client (simulates conflict - tag already exists)
        let netbox_tag = create_test_tag(1, "test-tag", "http://test-netbox");
        mock_token_resolver.mock_client().add_tag(netbox_tag);
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create tag CRD without status (will try to create, get conflict, then find existing)
        let mut tag = create_test_netbox_tag("test-tag", "default", None);
        tag.status = None;
        apis.tag_api.store("test-tag".to_string(), tag.clone());
        
        // Execute: Reconcile (should handle conflict gracefully)
        let result = reconciler.reconcile_netbox_tag(&tag).await;
        
        // Assert: Should succeed (conflict handled by finding existing tag)
        assert!(result.is_ok(), "Reconciliation should succeed after conflict: {:?}", result.err());
        
        // Assert: Status should be updated with existing NetBox ID
        let updated_crd = apis.tag_api.as_ref().get("test-tag").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should be set to existing tag ID");
    }
}

