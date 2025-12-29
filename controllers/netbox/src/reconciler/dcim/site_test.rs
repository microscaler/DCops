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
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
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
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_site_update() {
        // TODO: Test site update scenario
        // 1. Create site with status (already created)
        // 2. Modify spec (e.g., description, physical_address)
        // 3. Reconcile
        // 4. Verify update_site was called
    }
    
    #[tokio::test]
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_site_idempotent() {
        // TODO: Test idempotent reconciliation
        // 1. Create site with status
        // 2. Reconcile without changes
        // 3. Verify no update was called (resource already up-to-date)
    }
    
    #[tokio::test]
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_site_conflict_handling() {
        // TODO: Test conflict handling (GitOps idempotency)
        // 1. Try to create site
        // 2. Simulate conflict error
        // 3. Verify reconciler queries for existing site
        // 4. Verify existing site is reused
    }
    
    #[tokio::test]
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_site_drift_detection() {
        // TODO: Test drift detection
        // 1. Create site with status
        // 2. Delete site in NetBox (simulate drift)
        // 3. Reconcile
        // 4. Verify status is cleared and site is recreated
    }
    
    #[tokio::test]
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_site_tenant_dependency() {
        // TODO: Test tenant dependency resolution
        // 1. Create site with tenant reference
        // 2. Tenant doesn't exist yet
        // 3. Reconcile should fail with dependency error
    }
}

