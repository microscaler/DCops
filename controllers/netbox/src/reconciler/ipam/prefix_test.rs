//! Unit tests for NetBoxPrefix reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crds::{NetBoxPrefix, NetBoxTenant, PrefixState};
    
    /// Helper to set up test data for prefix reconciliation
    fn setup_prefix_test_data() -> (NetBoxPrefix, NetBoxTenant) {
        // Create test tenant with status (required dependency)
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        
        // Create test prefix CRD
        let mut prefix = create_test_netbox_prefix("test-prefix", "default", 0, None);
        prefix.status = None; // Clear status to test create path
        prefix.spec.tenant = crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "datacenter-tenant".to_string(),
            namespace: Some("default".to_string()),
        };
        
        (prefix, tenant)
    }
    
    #[tokio::test]
    async fn test_reconcile_prefix_create() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver with MockNetBoxClient
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        // Setup: Add secret for tenant
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Get MockNetBoxClient to set up test data
        let _mock_client = mock_token_resolver.mock_client();
        
        // Setup: Create test data
        let (mut prefix, tenant) = setup_prefix_test_data();
        
        // Setup: Create reconciler with MockTokenResolver
        // This returns both the reconciler and the unboxed APIs so we can store test data
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store test data in the APIs before reconciliation
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.prefix_api.store("test-prefix".to_string(), prefix.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_prefix(&prefix).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        // Get the updated prefix from the API (same instance used by reconciler)
        use crate::kube_api_trait::KubeApiTrait;
        let updated_crd = apis.prefix_api.get("test-prefix").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, PrefixState::Created, "State should be Created");
    }
    
    #[tokio::test]
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_prefix_update() {
        // TODO: Test prefix update scenario
        // 1. Create prefix with status (already created)
        // 2. Modify spec (e.g., description)
        // 3. Reconcile
        // 4. Verify update was called
    }
    
    #[tokio::test]
    async fn test_reconcile_prefix_idempotent() {
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
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
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
        
        // Setup: Add prefix to mock NetBox (already exists)
        let netbox_prefix = create_test_prefix(1, "192.168.1.0/24", &netbox_url);
        mock_client.add_prefix(netbox_prefix);
        
        // Setup: Create prefix with status (already created)
        let mut prefix = create_test_netbox_prefix(
            "test-prefix",
            "default",
            1,
            Some("http://test-netbox/api/ipam/prefixes/1/".to_string()),
        );
        prefix.spec.tenant = crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "datacenter-tenant".to_string(),
            namespace: Some("default".to_string()),
        };
        apis.prefix_api.store("test-prefix".to_string(), prefix.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_prefix(&prefix).await;
        
        // Assert: Should succeed (idempotent - no update needed)
        assert!(result.is_ok(), "Reconciliation should succeed when prefix already exists");
        
        // Verify status is still correct
        let updated_crd = apis.prefix_api.get("test-prefix").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should still be 1");
    }
    
    #[tokio::test]
    async fn test_reconcile_prefix_tenant_not_found() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use std::sync::Arc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Create reconciler
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create prefix with reference to non-existent tenant
        let mut prefix = create_test_netbox_prefix("test-prefix", "default", 0, None);
        prefix.status = None;
        prefix.spec.tenant = crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "non-existent-tenant".to_string(),
            namespace: Some("default".to_string()),
        };
        apis.prefix_api.store("test-prefix".to_string(), prefix.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_prefix(&prefix).await;
        
        // Assert: Should fail with tenant not found error
        assert!(result.is_err(), "Reconciliation should fail when tenant not found");
    }
    
    #[tokio::test]
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_prefix_drift_detection() {
        // TODO: Test drift detection
        // 1. Create prefix with status
        // 2. Delete prefix in NetBox (simulate drift)
        // 3. Reconcile
        // 4. Verify status is cleared and prefix is recreated
    }
}

