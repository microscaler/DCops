//! Unit tests for NetBoxPrefix reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::kube_api_trait::mock::MockKubeApi;
    use crate::reconciler::Reconciler;
    use netbox_client::{MockNetBoxClient, NetBoxClientTrait};
    use crds::{NetBoxPrefix, NetBoxTenant, PrefixState};
    use kube::Client;
    
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
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Create test data
        let (mut prefix, tenant) = setup_prefix_test_data();
        
        // Setup: Create mock Kubernetes APIs
        let mut tenant_api = MockKubeApi::<NetBoxTenant>::new();
        tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        let mut prefix_api = MockKubeApi::<NetBoxPrefix>::new();
        prefix_api.store("test-prefix".to_string(), prefix.clone());
        
        // Setup: Create reconciler with MockTokenResolver
        let reconciler = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_prefix(&prefix).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = prefix_api.get("test-prefix").await.unwrap();
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
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_prefix_idempotent() {
        // TODO: Test idempotent reconciliation
        // 1. Create prefix with status
        // 2. Reconcile without changes
        // 3. Verify no update was called (resource already up-to-date)
    }
    
    #[tokio::test]
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_prefix_conflict_handling() {
        // TODO: Test conflict handling (GitOps idempotency)
        // 1. Try to create prefix
        // 2. Simulate conflict error
        // 3. Verify reconciler queries for existing prefix
        // 4. Verify existing prefix is reused
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

