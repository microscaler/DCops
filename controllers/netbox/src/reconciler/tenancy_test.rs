//! Unit tests for NetBoxTenant reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::kube_api_trait::mock::MockKubeApi;
    use netbox_client::MockNetBoxClient;
    use crds::NetBoxTenant;
    use kube::Client;
    use k8s_openapi::api::core::v1::Secret;
    
    /// Helper to set up test data for tenant reconciliation
    fn setup_tenant_test_data() -> NetBoxTenant {
        create_test_netbox_tenant("test-tenant", "default", None, None)
    }
    
    /// Helper to create a test secret with token
    fn create_test_secret(name: &str, namespace: &str, token: &str) -> Secret {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        use std::collections::BTreeMap;
        
        let mut data = BTreeMap::new();
        // Base64 encode the token (Kubernetes secrets store base64-encoded data)
        // Using base64 crate would require adding it as a dependency
        // For now, we'll just use the raw bytes - in real tests, we'd use base64::encode
        // This is a placeholder until kube::Client mocking is implemented
        let token_bytes = token.as_bytes().to_vec();
        data.insert("token".to_string(), k8s_openapi::ByteString(token_bytes));
        
        Secret {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            data: Some(data),
            ..Default::default()
        }
    }
    
    #[tokio::test]
    async fn test_reconcile_tenant_create() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::test_utils::mock_token_resolver::TestReconcilerApis;
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        use crds::ResourceState;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        // Setup: Add secret for tenant (tenant reconciler fetches its own secret)
        mock_token_resolver.add_secret("default", "netbox-token-test-tenant", "test-token-123".to_string());
        
        // Setup: Get MockNetBoxClient
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Create reconciler
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        let TestReconcilerApis {
            tenant_api,
            ..
        } = apis;
        
        // Setup: Create test NetBoxTenant CRD (no status - needs creation)
        let mut tenant = setup_tenant_test_data();
        tenant.status = None; // Clear status to test create path
        tenant_api.store("test-tenant".to_string(), tenant.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_tenant(&tenant).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = tenant_api.get("test-tenant").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_tenant_update() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::test_utils::mock_token_resolver::TestReconcilerApis;
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        use crds::ResourceState;
        use netbox_client::Tenant;
        use chrono::Utc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-test-tenant", "test-token-123".to_string());
        
        // Setup: Get MockNetBoxClient
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Add tenant to mock NetBox (already exists with old description)
        let netbox_tenant = Tenant {
            id: 1,
            url: format!("{}/api/tenancy/tenants/1/", netbox_url),
            display: "test-tenant".to_string(),
            name: "test-tenant".to_string(),
            slug: "test-tenant".to_string(),
            description: Some("Old description".to_string()),
            comments: Some(String::new()),
            group: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        mock_client.add_tenant(netbox_tenant);
        
        // Setup: Create reconciler
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        let TestReconcilerApis {
            tenant_api,
            ..
        } = apis;
        
        // Setup: Create test NetBoxTenant CRD with status and updated description
        let mut tenant = create_test_netbox_tenant(
            "test-tenant",
            "default",
            Some(1),
            Some(format!("{}/api/tenancy/tenants/1/", netbox_url)),
        );
        tenant.spec.description = Some("Updated description".to_string()); // Changed description
        tenant_api.store("test-tenant".to_string(), tenant.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_tenant(&tenant).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Tenant should be updated in NetBox (verify via trait)
        use netbox_client::NetBoxClientTrait;
        let netbox_client = reconciler.token_resolver
            .create_client_with_token("test-token-123".to_string())
            .unwrap();
        let updated_tenant = netbox_client.get_tenant(netbox_client::TenantId(1)).await.unwrap();
        assert_eq!(updated_tenant.description, Some("Updated description".to_string()), "Tenant description should be updated");
    }
    
    #[tokio::test]
    async fn test_reconcile_tenant_idempotent() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::test_utils::mock_token_resolver::TestReconcilerApis;
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        use crds::ResourceState;
        use netbox_client::Tenant;
        use chrono::Utc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-test-tenant", "test-token-123".to_string());
        
        // Setup: Get MockNetBoxClient
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Add tenant to mock NetBox (already exists)
        let netbox_tenant = Tenant {
            id: 1,
            url: format!("{}/api/tenancy/tenants/1/", netbox_url),
            display: "test-tenant".to_string(),
            name: "test-tenant".to_string(),
            slug: "test-tenant".to_string(),
            description: Some("Primary tenant for datacenter operations".to_string()),
            comments: Some(String::new()),
            group: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        mock_client.add_tenant(netbox_tenant);
        
        // Setup: Create reconciler
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        let TestReconcilerApis {
            tenant_api,
            ..
        } = apis;
        
        // Setup: Create tenant with status (already created, no changes)
        let tenant = create_test_netbox_tenant(
            "test-tenant",
            "default",
            Some(1),
            Some(format!("{}/api/tenancy/tenants/1/", netbox_url)),
        );
        tenant_api.store("test-tenant".to_string(), tenant.clone());
        
        // Execute: Reconcile (should be idempotent - no changes needed)
        let result = reconciler.reconcile_netbox_tenant(&tenant).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should still be correct (idempotent - no update needed)
        let updated_crd = tenant_api.get("test-tenant").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should still be 1");
        assert_eq!(status.state, ResourceState::Created, "State should still be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_tenant_conflict_handling() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::test_utils::mock_token_resolver::TestReconcilerApis;
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        use crds::ResourceState;
        use netbox_client::Tenant;
        use chrono::Utc;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-test-tenant", "test-token-123".to_string());
        
        // Setup: Get MockNetBoxClient
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Add tenant to mock NetBox (simulating it already exists - conflict scenario)
        let netbox_tenant = Tenant {
            id: 1,
            url: format!("{}/api/tenancy/tenants/1/", netbox_url),
            display: "test-tenant".to_string(),
            name: "test-tenant".to_string(),
            slug: "test-tenant".to_string(),
            description: Some("Primary tenant for datacenter operations".to_string()),
            comments: Some(String::new()),
            group: None,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        mock_client.add_tenant(netbox_tenant);
        
        // Setup: Create reconciler
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        let TestReconcilerApis {
            tenant_api,
            ..
        } = apis;
        
        // Setup: Create tenant CRD without status (will try to create, but tenant already exists)
        let mut tenant = setup_tenant_test_data();
        tenant.status = None; // No status - will try to create
        tenant_api.store("test-tenant".to_string(), tenant.clone());
        
        // Execute: Reconcile (should handle conflict by finding existing tenant)
        let result = reconciler.reconcile_netbox_tenant(&tenant).await;
        
        // Assert: Should succeed (conflict handled via idempotency query)
        assert!(result.is_ok(), "Reconciliation should succeed after conflict handling: {:?}", result.err());
        
        // Assert: Status should be updated with existing NetBox ID
        let updated_crd = tenant_api.get("test-tenant").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should be set to existing tenant ID");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_tenant_drift_detection() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::test_utils::mock_token_resolver::TestReconcilerApis;
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        use crds::ResourceState;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-test-tenant", "test-token-123".to_string());
        
        // Setup: Create reconciler
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        let TestReconcilerApis {
            tenant_api,
            ..
        } = apis;
        
        // Setup: Create tenant CRD with status (tenant already exists)
        let mut tenant = setup_tenant_test_data();
        tenant.status = Some(crds::NetBoxTenantStatus {
            netbox_id: Some(1),
            netbox_url: Some(format!("{}/api/tenancy/tenants/1/", netbox_url)),
            state: ResourceState::Created,
            error: None,
            last_reconciled: None,
        });
        tenant_api.store("test-tenant".to_string(), tenant.clone());
        
        // IMPORTANT: Do NOT add the tenant to mock NetBox client (simulating drift - tenant was deleted)
        // This will cause validate_status_and_drift to detect the tenant is missing and trigger recreation
        
        // Execute: Reconcile (should detect drift and recreate tenant)
        let result = reconciler.reconcile_netbox_tenant(&tenant).await;
        
        // Assert: Should succeed (tenant will be recreated)
        assert!(result.is_ok(), "Reconciliation should succeed after drift detection: {:?}", result.err());
        
        // Assert: Status should be updated with new NetBox ID (tenant was recreated)
        let updated_crd = tenant_api.get("test-tenant").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set after recreation");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_tenant_secret_not_found() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::test_utils::mock_token_resolver::TestReconcilerApis;
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        use crds::ResourceState;
        
        // Setup: Create mock TokenResolver (but don't add the secret)
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url));
        // Intentionally NOT adding the secret to simulate secret not found
        
        // Setup: Create reconciler
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        let TestReconcilerApis {
            tenant_api,
            ..
        } = apis;
        
        // Setup: Create tenant with invalid secret reference
        let mut tenant = setup_tenant_test_data();
        tenant.status = None; // Clear status to test create path
        tenant_api.store("test-tenant".to_string(), tenant.clone());
        
        // Execute: Reconcile (should fail because secret is not found)
        let result = reconciler.reconcile_netbox_tenant(&tenant).await;
        
        // Assert: Should fail with secret not found error
        assert!(result.is_err(), "Reconciliation should fail when secret is not found");
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("Secret") || error_msg.contains("not found") || error_msg.contains("404"), 
                "Error should mention missing secret: {}", error_msg);
        
        // Assert: Status should be updated with error
        let updated_crd = tenant_api.get("test-tenant").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.state, ResourceState::Failed, "State should be Failed");
        assert!(status.error.is_some(), "Error should be set in status");
    }
    
    #[tokio::test]
    async fn test_reconcile_tenant_token_decode_error() {
        use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
        use crate::test_utils::mock_token_resolver::TestReconcilerApis;
        use crate::kube_api_trait::KubeApiTrait;
        use std::sync::Arc;
        use crds::ResourceState;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url));
        
        // Setup: Add secret with invalid token (empty token to simulate decode error)
        // The MockSecretFetcher will return the token as-is, but NetBoxClient creation will fail
        // Actually, empty token will fail validation in the reconciler
        mock_token_resolver.add_secret("default", "netbox-token-test-tenant", "".to_string());
        
        // Setup: Create reconciler
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        let TestReconcilerApis {
            tenant_api,
            ..
        } = apis;
        
        // Setup: Create tenant
        let mut tenant = setup_tenant_test_data();
        tenant.status = None; // Clear status to test create path
        tenant_api.store("test-tenant".to_string(), tenant.clone());
        
        // Execute: Reconcile (should fail because token is empty)
        let result = reconciler.reconcile_netbox_tenant(&tenant).await;
        
        // Assert: Should fail with token decode/validation error
        assert!(result.is_err(), "Reconciliation should fail when token is empty");
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("empty") || error_msg.contains("token") || error_msg.contains("Token"), 
                "Error should mention empty/invalid token: {}", error_msg);
        
        // Assert: Status should be updated with error
        let updated_crd = tenant_api.get("test-tenant").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.state, ResourceState::Failed, "State should be Failed");
        assert!(status.error.is_some(), "Error should be set in status");
    }
}

