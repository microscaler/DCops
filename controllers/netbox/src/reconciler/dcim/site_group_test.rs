//! Unit tests for NetBoxSiteGroup reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::kube_api_trait::KubeApiTrait;
    use crds::{NetBoxSiteGroup, NetBoxTenant, ResourceState};
    use std::sync::Arc;
    use chrono::Utc;

    /// Helper to create test NetBoxSiteGroup CRD
    fn create_test_netbox_site_group(
        name: &str,
        namespace: &str,
        netbox_id: Option<u64>,
        parent: Option<&str>,
    ) -> NetBoxSiteGroup {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        use crds::references::NetBoxResourceReference;
        
        NetBoxSiteGroup {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxSiteGroupSpec {
                name: name.to_string(),
                slug: Some(name.to_lowercase().replace(' ', "-")),
                parent: parent.map(|p| NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxSiteGroup".to_string(),
                    name: p.to_string(),
                    namespace: None,
                }),
                description: Some(format!("Test site group {}", name)),
                comments: None,
                tags: None,
            },
            status: netbox_id.map(|id| crds::NetBoxSiteGroupStatus {
                netbox_id: Some(id),
                netbox_url: Some(format!("http://test-netbox/api/dcim/site-groups/{}/", id)),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        }
    }

    /// Helper to create test NetBox SiteGroup model
    fn create_test_site_group(
        id: u64,
        name: &str,
        base_url: &str,
        parent_id: Option<u64>,
    ) -> netbox_client::SiteGroup {
        use netbox_client::NestedSiteGroup;
        
        netbox_client::SiteGroup {
            id,
            url: format!("{}/api/dcim/site-groups/{}/", base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: name.to_lowercase().replace(' ', "-"),
            parent: parent_id.map(|pid| NestedSiteGroup {
                id: pid,
                url: format!("{}/api/dcim/site-groups/{}/", base_url, pid),
                display: "parent-site-group".to_string(),
                name: "parent-site-group".to_string(),
                slug: "parent-site-group".to_string(),
            }),
            description: Some(format!("Test site group {}", name)),
            comments: None,
            site_count: 0,
            prefix_count: 0,
            _depth: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_reconcile_site_group_create() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        // Setup: Add tenant and secret (required for shared resource)
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        mock_token_resolver.mock_client().add_tenant(netbox_client::Tenant {
            id: 1,
            url: "http://test-netbox/api/tenancy/tenants/1/".to_string(),
            display: "datacenter-tenant".to_string(),
            name: "datacenter-tenant".to_string(),
            slug: "datacenter-tenant".to_string(),
            description: None,
            comments: None,
            group: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        });
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant in API
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        // Setup: Create site group CRD without status
        let mut site_group = create_test_netbox_site_group("test-site-group", "default", None, None);
        site_group.status = None;
        apis.site_group_api.store("test-site-group".to_string(), site_group.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_site_group(&site_group).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.site_group_api.as_ref().get("test-site-group").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }

    #[tokio::test]
    async fn test_reconcile_site_group_idempotent() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        // Setup: Add tenant and secret (required for shared resource)
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        mock_token_resolver.mock_client().add_tenant(netbox_client::Tenant {
            id: 1,
            url: "http://test-netbox/api/tenancy/tenants/1/".to_string(),
            display: "datacenter-tenant".to_string(),
            name: "datacenter-tenant".to_string(),
            slug: "datacenter-tenant".to_string(),
            description: None,
            comments: None,
            group: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        });
        
        // Setup: Add site group to mock NetBox client
        let netbox_site_group = create_test_site_group(1, "test-site-group", "http://test-netbox", None);
        mock_token_resolver.mock_client().add_site_group(netbox_site_group);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant in API
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        // Setup: Create site group CRD with status (already created)
        let site_group = create_test_netbox_site_group("test-site-group", "default", Some(1), None);
        apis.site_group_api.store("test-site-group".to_string(), site_group.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_site_group(&site_group).await;
        
        // Assert: Should succeed (idempotent)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain unchanged
        let updated_crd = apis.site_group_api.as_ref().get("test-site-group").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should remain 1");
    }

    #[tokio::test]
    async fn test_reconcile_site_group_with_parent() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        // Setup: Add tenant and secret (required for shared resource)
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        mock_token_resolver.mock_client().add_tenant(netbox_client::Tenant {
            id: 1,
            url: "http://test-netbox/api/tenancy/tenants/1/".to_string(),
            display: "datacenter-tenant".to_string(),
            name: "datacenter-tenant".to_string(),
            slug: "datacenter-tenant".to_string(),
            description: None,
            comments: None,
            group: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        });
        
        // Setup: Create parent site group (required dependency)
        let parent_site_group = create_test_netbox_site_group("parent-site-group", "default", Some(1), None);
        let parent_netbox_site_group = create_test_site_group(1, "parent-site-group", "http://test-netbox", None);
        mock_token_resolver.mock_client().add_site_group(parent_netbox_site_group);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant and parent site group in APIs
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.site_group_api.store("parent-site-group".to_string(), parent_site_group);
        
        // Setup: Create child site group CRD without status
        let mut site_group = create_test_netbox_site_group("child-site-group", "default", None, Some("parent-site-group"));
        site_group.status = None;
        apis.site_group_api.store("child-site-group".to_string(), site_group.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_site_group(&site_group).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.site_group_api.as_ref().get("child-site-group").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
}

