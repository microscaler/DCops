//! Unit tests for NetBoxSite reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crds::{NetBoxSite, NetBoxTenant, ResourceState};
    
    /// Helper to set up test data for site reconciliation
    fn setup_site_test_data() -> (NetBoxSite, NetBoxTenant) {
        // Create test tenant with status (required dependency)
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        
        // Create test site CRD
        let mut site = create_test_netbox_site("test-site", "default", None, None);
        site.status = None; // Clear status to test create path
        site.spec.tenant = crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "datacenter-tenant".to_string(),
            namespace: Some("default".to_string()),
        };
        
        (site, tenant)
    }
    
    #[tokio::test]
    async fn test_reconcile_site_create() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver with MockNetBoxClient
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        // Setup: Add secret for tenant
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Get MockNetBoxClient to set up test data
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Create test data
        let (mut site, tenant) = setup_site_test_data();
        site.status = None; // Clear status to test create path
        
        // Setup: Create reconciler with MockTokenResolver
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store test data in the APIs before reconciliation
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.site_api.store("test-site".to_string(), site.clone());
        
        // Setup: Add tenant to mock NetBox (required for create_site call)
        // The tenant is already created by the reconciler, but we need it in NetBox for the site creation
        // Since the tenant reconciler would have created it, we'll simulate that by adding it to the mock
        use netbox_client::Tenant;
        use chrono::Utc;
        let netbox_tenant = Tenant {
            id: 1,
            url: format!("{}/api/tenancy/tenants/1/", netbox_url),
            display: "Data Center Operations".to_string(),
            name: "Data Center Operations".to_string(),
            slug: "datacenter-ops".to_string(),
            description: Some("Primary tenant for datacenter operations".to_string()),
            comments: Some(String::new()),
            group: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        mock_client.add_tenant(netbox_tenant);
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_site(&site).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.site_api.get("test-site").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_site_update() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Get MockNetBoxClient
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create tenant
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        // Setup: Add tenant to mock NetBox
        use netbox_client::Tenant;
        use chrono::Utc;
        let netbox_tenant = Tenant {
            id: 1,
            url: format!("{}/api/tenancy/tenants/1/", netbox_url),
            display: "Data Center Operations".to_string(),
            name: "Data Center Operations".to_string(),
            slug: "datacenter-ops".to_string(),
            description: Some("Primary tenant for datacenter operations".to_string()),
            comments: Some(String::new()),
            group: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        mock_client.add_tenant(netbox_tenant);
        
        // Setup: Add site to mock NetBox (already exists with old description)
        use netbox_client::Site;
        let mut netbox_site = Site {
            id: 1,
            url: format!("{}/api/dcim/sites/1/", netbox_url),
            display: "test-site".to_string(),
            name: "test-site".to_string(),
            slug: "test-site".to_string(),
            status: netbox_client::SiteStatus::Active,
            facility: None,
            region: None,
            site_group: None,
            tenant: Some(netbox_client::NestedTenant {
                id: 1,
                url: format!("{}/api/tenancy/tenants/1/", netbox_url),
                display: "Data Center Operations".to_string(),
                name: "Data Center Operations".to_string(),
                slug: "datacenter-ops".to_string(),
            }),
            description: Some("Old description".to_string()),
            physical_address: None,
            shipping_address: None,
            latitude: None,
            longitude: None,
            time_zone: None,
            comments: Some(String::new()),
            tags: vec![],
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        mock_client.add_site(netbox_site.clone());
        
        // Setup: Create site CRD with updated description
        let mut site = create_test_netbox_site(
            "test-site",
            "default",
            Some(1),
            Some("http://test-netbox/api/dcim/sites/1/".to_string()),
        );
        site.spec.tenant = crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "datacenter-tenant".to_string(),
            namespace: Some("default".to_string()),
        };
        site.spec.description = Some("Updated description".to_string()); // Changed description
        apis.site_api.store("test-site".to_string(), site.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_site(&site).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated
        let updated_crd = apis.site_api.get("test-site").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should still be 1");
        
        // Assert: Site should be updated in NetBox (verify via trait)
        use netbox_client::NetBoxClientTrait;
        let netbox_client = reconciler.token_resolver
            .create_client_for_tenant("default", &updated_crd.spec.tenant)
            .await
            .unwrap();
        let updated_site = netbox_client.get_site(netbox_client::SiteId(1)).await.unwrap();
        assert_eq!(updated_site.description, Some("Updated description".to_string()), "Site description should be updated");
    }
    
    #[tokio::test]
    async fn test_reconcile_site_idempotent() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Get MockNetBoxClient
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create tenant
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        // Setup: Add tenant to mock NetBox
        use netbox_client::Tenant;
        use chrono::Utc;
        let netbox_tenant = Tenant {
            id: 1,
            url: format!("{}/api/tenancy/tenants/1/", netbox_url),
            display: "Data Center Operations".to_string(),
            name: "Data Center Operations".to_string(),
            slug: "datacenter-ops".to_string(),
            description: Some("Primary tenant for datacenter operations".to_string()),
            comments: Some(String::new()),
            group: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        mock_client.add_tenant(netbox_tenant);
        
        // Setup: Add site to mock NetBox (already exists)
        use netbox_client::Site;
        let netbox_site = Site {
            id: 1,
            url: format!("{}/api/dcim/sites/1/", netbox_url),
            display: "test-site".to_string(),
            name: "test-site".to_string(),
            slug: "test-site".to_string(),
            status: netbox_client::SiteStatus::Active,
            facility: None,
            region: None,
            site_group: None,
            tenant: Some(netbox_client::NestedTenant {
                id: 1,
                url: format!("{}/api/tenancy/tenants/1/", netbox_url),
                display: "Data Center Operations".to_string(),
                name: "Data Center Operations".to_string(),
                slug: "datacenter-ops".to_string(),
            }),
            description: None,
            physical_address: None,
            shipping_address: None,
            latitude: None,
            longitude: None,
            time_zone: None,
            comments: Some(String::new()),
            tags: vec![],
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        mock_client.add_site(netbox_site);
        
        // Setup: Create site with status (already created)
        let mut site = create_test_netbox_site(
            "test-site",
            "default",
            Some(1),
            Some("http://test-netbox/api/dcim/sites/1/".to_string()),
        );
        site.spec.tenant = crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "datacenter-tenant".to_string(),
            namespace: Some("default".to_string()),
        };
        apis.site_api.store("test-site".to_string(), site.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_site(&site).await;
        
        // Assert: Should succeed (idempotent - no update needed)
        assert!(result.is_ok(), "Reconciliation should succeed when site already exists");
        
        // Verify status is still correct
        let updated_crd = apis.site_api.get("test-site").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should still be 1");
    }
    
    #[tokio::test]
    async fn test_reconcile_site_conflict_handling() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Get MockNetBoxClient
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create tenant
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        // Setup: Add tenant to mock NetBox
        use netbox_client::Tenant;
        use chrono::Utc;
        let netbox_tenant = Tenant {
            id: 1,
            url: format!("{}/api/tenancy/tenants/1/", netbox_url),
            display: "Data Center Operations".to_string(),
            name: "Data Center Operations".to_string(),
            slug: "datacenter-ops".to_string(),
            description: Some("Primary tenant for datacenter operations".to_string()),
            comments: Some(String::new()),
            group: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        mock_client.add_tenant(netbox_tenant);
        
        // Setup: Add site to mock NetBox (simulating it already exists - conflict scenario)
        use netbox_client::Site;
        let netbox_site = Site {
            id: 1,
            url: format!("{}/api/dcim/sites/1/", netbox_url),
            display: "test-site".to_string(),
            name: "test-site".to_string(),
            slug: "test-site".to_string(),
            status: netbox_client::SiteStatus::Active,
            facility: None,
            region: None,
            site_group: None,
            tenant: Some(netbox_client::NestedTenant {
                id: 1,
                url: format!("{}/api/tenancy/tenants/1/", netbox_url),
                display: "Data Center Operations".to_string(),
                name: "Data Center Operations".to_string(),
                slug: "datacenter-ops".to_string(),
            }),
            description: None,
            physical_address: None,
            shipping_address: None,
            latitude: None,
            longitude: None,
            time_zone: None,
            comments: Some(String::new()),
            tags: vec![],
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        mock_client.add_site(netbox_site);
        
        // Setup: Create site CRD without status (will try to create, but site already exists)
        let mut site = create_test_netbox_site("test-site", "default", None, None);
        site.status = None; // No status - will try to create
        site.spec.tenant = crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "datacenter-tenant".to_string(),
            namespace: Some("default".to_string()),
        };
        apis.site_api.store("test-site".to_string(), site.clone());
        
        // Execute: Reconcile (should handle conflict by finding existing site)
        let result = reconciler.reconcile_netbox_site(&site).await;
        
        // Assert: Should succeed (conflict handled via idempotency query)
        assert!(result.is_ok(), "Reconciliation should succeed after conflict handling: {:?}", result.err());
        
        // Assert: Status should be updated with existing NetBox ID
        let updated_crd = apis.site_api.get("test-site").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should be set to existing site ID");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_site_drift_detection() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Get MockNetBoxClient
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create tenant
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        // Setup: Add tenant to mock NetBox
        use netbox_client::Tenant;
        use chrono::Utc;
        let netbox_tenant = Tenant {
            id: 1,
            url: format!("{}/api/tenancy/tenants/1/", netbox_url),
            display: "Data Center Operations".to_string(),
            name: "Data Center Operations".to_string(),
            slug: "datacenter-ops".to_string(),
            description: Some("Primary tenant for datacenter operations".to_string()),
            comments: Some(String::new()),
            group: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        mock_client.add_tenant(netbox_tenant);
        
        // Setup: Create site CRD with status (site already exists)
        let mut site = create_test_netbox_site(
            "test-site",
            "default",
            Some(1),
            Some("http://test-netbox/api/dcim/sites/1/".to_string()),
        );
        site.spec.tenant = crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "datacenter-tenant".to_string(),
            namespace: Some("default".to_string()),
        };
        apis.site_api.store("test-site".to_string(), site.clone());
        
        // IMPORTANT: Do NOT add the site to mock NetBox client (simulating drift - site was deleted)
        // This will cause validate_status_and_drift to detect the site is missing and trigger recreation
        
        // Execute: Reconcile (should detect drift and recreate site)
        let result = reconciler.reconcile_netbox_site(&site).await;
        
        // Assert: Should succeed (site will be recreated)
        assert!(result.is_ok(), "Reconciliation should succeed after drift detection: {:?}", result.err());
        
        // Assert: Status should be updated with new NetBox ID (site was recreated)
        let updated_crd = apis.site_api.get("test-site").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set after recreation");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_site_tenant_dependency() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create site with reference to non-existent tenant
        let mut site = create_test_netbox_site("test-site", "default", None, None);
        site.status = None;
        site.spec.tenant = crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "non-existent-tenant".to_string(),
            namespace: Some("default".to_string()),
        };
        apis.site_api.store("test-site".to_string(), site.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_site(&site).await;
        
        // Assert: Should fail with tenant not found error
        assert!(result.is_err(), "Reconciliation should fail when tenant not found");
    }
}

