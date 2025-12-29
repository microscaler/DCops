//! Unit tests for NetBoxRegion reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::kube_api_trait::KubeApiTrait;
    use crds::{NetBoxRegion, NetBoxTenant, ResourceState};
    use std::sync::Arc;
    use chrono::Utc;

    /// Helper to create test NetBoxRegion CRD
    fn create_test_netbox_region(
        name: &str,
        namespace: &str,
        netbox_id: Option<u64>,
        parent: Option<&str>,
    ) -> NetBoxRegion {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        use crds::references::NetBoxResourceReference;
        
        NetBoxRegion {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxRegionSpec {
                name: name.to_string(),
                slug: Some(name.to_lowercase().replace(' ', "-")),
                parent: parent.map(|p| NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxRegion".to_string(),
                    name: p.to_string(),
                    namespace: None,
                }),
                description: Some(format!("Test region {}", name)),
            },
            status: netbox_id.map(|id| crds::NetBoxRegionStatus {
                netbox_id: Some(id),
                netbox_url: Some(format!("http://test-netbox/api/dcim/regions/{}/", id)),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        }
    }

    /// Helper to create test NetBox Region model
    fn create_test_region(
        id: u64,
        name: &str,
        base_url: &str,
        parent_id: Option<u64>,
    ) -> netbox_client::Region {
        use netbox_client::NestedRegion;
        
        netbox_client::Region {
            id,
            url: format!("{}/api/dcim/regions/{}/", base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: name.to_lowercase().replace(' ', "-"),
            parent: parent_id.map(|pid| NestedRegion {
                id: pid,
                url: format!("{}/api/dcim/regions/{}/", base_url, pid),
                display: "parent-region".to_string(),
                name: "parent-region".to_string(),
                slug: "parent-region".to_string(),
            }),
            description: Some(format!("Test region {}", name)),
            comments: None,
            site_count: 0,
            prefix_count: 0,
            _depth: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_reconcile_region_create() {
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
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant in API
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        // Setup: Create region CRD without status
        let mut region = create_test_netbox_region("test-region", "default", None, None);
        region.status = None;
        apis.region_api.store("test-region".to_string(), region.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_region(&region).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.region_api.as_ref().get("test-region").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }

    #[tokio::test]
    async fn test_reconcile_region_idempotent() {
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
        
        // Setup: Add region to mock NetBox client
        let netbox_region = create_test_region(1, "test-region", "http://test-netbox", None);
        mock_token_resolver.mock_client().add_region(netbox_region);
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant in API
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        // Setup: Create region CRD with status (already created)
        let region = create_test_netbox_region("test-region", "default", Some(1), None);
        apis.region_api.store("test-region".to_string(), region.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_region(&region).await;
        
        // Assert: Should succeed (idempotent)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain unchanged
        let updated_crd = apis.region_api.as_ref().get("test-region").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should remain 1");
    }

    #[tokio::test]
    async fn test_reconcile_region_with_parent() {
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
        
        // Setup: Create parent region (required dependency)
        let parent_region = create_test_netbox_region("parent-region", "default", Some(1), None);
        let parent_netbox_region = create_test_region(1, "parent-region", "http://test-netbox", None);
        mock_token_resolver.mock_client().add_region(parent_netbox_region);
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant and parent region in APIs
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.region_api.store("parent-region".to_string(), parent_region);
        
        // Setup: Create child region CRD without status
        let mut region = create_test_netbox_region("child-region", "default", None, Some("parent-region"));
        region.status = None;
        apis.region_api.store("child-region".to_string(), region.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_region(&region).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.region_api.as_ref().get("child-region").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
}

