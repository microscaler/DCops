//! Unit tests for NetBoxLocation reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::kube_api_trait::KubeApiTrait;
    use crds::{NetBoxLocation, NetBoxSite, NetBoxTenant, ResourceState};
    use std::sync::Arc;
    use chrono::Utc;

    /// Helper to create test NetBoxLocation CRD
    fn create_test_netbox_location(
        name: &str,
        namespace: &str,
        site_name: &str,
        tenant_name: &str,
        netbox_id: Option<u64>,
        parent: Option<&str>,
    ) -> NetBoxLocation {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        use crds::references::NetBoxResourceReference;
        
        NetBoxLocation {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxLocationSpec {
                name: name.to_string(),
                slug: Some(name.to_lowercase().replace(' ', "-")),
                site: NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxSite".to_string(),
                    name: site_name.to_string(),
                    namespace: None,
                },
                tenant: NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: tenant_name.to_string(),
                    namespace: None,
                },
                parent: parent.map(|p| NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxLocation".to_string(),
                    name: p.to_string(),
                    namespace: None,
                }),
                facility: None,
                description: Some(format!("Test location {}", name)),
            },
            status: netbox_id.map(|id| crds::NetBoxLocationStatus {
                netbox_id: Some(id),
                netbox_url: Some(format!("http://test-netbox/api/dcim/locations/{}/", id)),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        }
    }

    /// Helper to create test NetBox Location model
    fn create_test_location(
        id: u64,
        site_id: u64,
        name: &str,
        base_url: &str,
        parent_id: Option<u64>,
        _tenant_id: Option<u64>,
    ) -> netbox_client::Location {
        use netbox_client::{NestedSite, NestedLocation, NestedTenant};
        
        netbox_client::Location {
            id,
            url: format!("{}/api/dcim/locations/{}/", base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: name.to_lowercase().replace(' ', "-"),
            site: NestedSite {
                id: site_id,
                url: format!("{}/api/dcim/sites/{}/", base_url, site_id),
                display: "test-site".to_string(),
                name: "test-site".to_string(),
                slug: "test-site".to_string(),
            },
            parent: parent_id.map(|pid| NestedLocation {
                id: pid,
                url: format!("{}/api/dcim/locations/{}/", base_url, pid),
                display: "parent-location".to_string(),
                name: "parent-location".to_string(),
                slug: "parent-location".to_string(),
            }),
            description: Some(format!("Test location {}", name)),
            comments: None,
            device_count: 0,
            rack_count: 0,
            _depth: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_reconcile_location_create() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        // Setup: Add tenant and secret
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
        
        // Setup: Create site (required dependency)
        let site = create_test_netbox_site(
            "test-site",
            "default",
            Some(1),
            Some("http://test-netbox/api/dcim/sites/1/".to_string()),
        );
        mock_token_resolver.mock_client().add_site(netbox_client::Site {
            id: 1,
            url: "http://test-netbox/api/dcim/sites/1/".to_string(),
            display: "test-site".to_string(),
            name: "test-site".to_string(),
            slug: "test-site".to_string(),
            status: netbox_client::SiteStatus::Active,
            region: None,
            site_group: None,
            tenant: Some(netbox_client::NestedTenant {
                id: 1,
                url: "http://test-netbox/api/tenancy/tenants/1/".to_string(),
                display: "datacenter-tenant".to_string(),
                name: "datacenter-tenant".to_string(),
                slug: "datacenter-tenant".to_string(),
            }),
            facility: None,
            time_zone: None,
            description: None,
            comments: None,
            physical_address: None,
            shipping_address: None,
            latitude: None,
            longitude: None,
            tags: vec![],
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        });
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant and site in APIs
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.site_api.store("test-site".to_string(), site);
        
        // Setup: Create location CRD without status
        let mut location = create_test_netbox_location("test-location", "default", "test-site", "datacenter-tenant", None, None);
        location.status = None;
        apis.location_api.store("test-location".to_string(), location.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_location(&location).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.location_api.as_ref().get("test-location").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }

    #[tokio::test]
    async fn test_reconcile_location_idempotent() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        // Setup: Add tenant and secret
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
        
        // Setup: Create site (required dependency)
        let site = create_test_netbox_site(
            "test-site",
            "default",
            Some(1),
            Some("http://test-netbox/api/dcim/sites/1/".to_string()),
        );
        mock_token_resolver.mock_client().add_site(netbox_client::Site {
            id: 1,
            url: "http://test-netbox/api/dcim/sites/1/".to_string(),
            display: "test-site".to_string(),
            name: "test-site".to_string(),
            slug: "test-site".to_string(),
            status: netbox_client::SiteStatus::Active,
            region: None,
            site_group: None,
            tenant: Some(netbox_client::NestedTenant {
                id: 1,
                url: "http://test-netbox/api/tenancy/tenants/1/".to_string(),
                display: "datacenter-tenant".to_string(),
                name: "datacenter-tenant".to_string(),
                slug: "datacenter-tenant".to_string(),
            }),
            facility: None,
            time_zone: None,
            description: None,
            comments: None,
            physical_address: None,
            shipping_address: None,
            latitude: None,
            longitude: None,
            tags: vec![],
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        });
        
        // Setup: Add location to mock NetBox client
        let netbox_location = create_test_location(1, 1, "test-location", "http://test-netbox", None, Some(1));
        mock_token_resolver.mock_client().add_location(netbox_location);
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant and site in APIs
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.site_api.store("test-site".to_string(), site);
        
        // Setup: Create location CRD with status (already created)
        let location = create_test_netbox_location("test-location", "default", "test-site", "datacenter-tenant", Some(1), None);
        apis.location_api.store("test-location".to_string(), location.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_location(&location).await;
        
        // Assert: Should succeed (idempotent)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain unchanged
        let updated_crd = apis.location_api.as_ref().get("test-location").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should remain 1");
    }

    #[tokio::test]
    async fn test_reconcile_location_site_not_found() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create location CRD with site that doesn't exist
        let mut location = create_test_netbox_location("test-location", "default", "nonexistent-site", "datacenter-tenant", None, None);
        location.status = None;
        apis.location_api.store("test-location".to_string(), location.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_location(&location).await;
        
        // Assert: Should fail with InvalidConfig error (site not found)
        assert!(result.is_err(), "Reconciliation should fail when site not found");
        match result.unwrap_err() {
            crate::error::ControllerError::InvalidConfig(_) => {
                // Expected error type
            }
            e => panic!("Expected InvalidConfig error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_reconcile_location_with_parent() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        // Setup: Add tenant and secret
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
        
        // Setup: Create site (required dependency)
        let site = create_test_netbox_site(
            "test-site",
            "default",
            Some(1),
            Some("http://test-netbox/api/dcim/sites/1/".to_string()),
        );
        mock_token_resolver.mock_client().add_site(netbox_client::Site {
            id: 1,
            url: "http://test-netbox/api/dcim/sites/1/".to_string(),
            display: "test-site".to_string(),
            name: "test-site".to_string(),
            slug: "test-site".to_string(),
            status: netbox_client::SiteStatus::Active,
            region: None,
            site_group: None,
            tenant: Some(netbox_client::NestedTenant {
                id: 1,
                url: "http://test-netbox/api/tenancy/tenants/1/".to_string(),
                display: "datacenter-tenant".to_string(),
                name: "datacenter-tenant".to_string(),
                slug: "datacenter-tenant".to_string(),
            }),
            facility: None,
            time_zone: None,
            description: None,
            comments: None,
            physical_address: None,
            shipping_address: None,
            latitude: None,
            longitude: None,
            tags: vec![],
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        });
        
        // Setup: Create parent location (required dependency)
        let parent_location = create_test_netbox_location("parent-location", "default", "test-site", "datacenter-tenant", Some(1), None);
        let parent_netbox_location = create_test_location(1, 1, "parent-location", "http://test-netbox", None, Some(1));
        mock_token_resolver.mock_client().add_location(parent_netbox_location);
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant, site, and parent location in APIs
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.site_api.store("test-site".to_string(), site);
        apis.location_api.store("parent-location".to_string(), parent_location);
        
        // Setup: Create child location CRD without status
        let mut location = create_test_netbox_location("child-location", "default", "test-site", "datacenter-tenant", None, Some("parent-location"));
        location.status = None;
        apis.location_api.store("child-location".to_string(), location.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_location(&location).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.location_api.as_ref().get("child-location").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
}

