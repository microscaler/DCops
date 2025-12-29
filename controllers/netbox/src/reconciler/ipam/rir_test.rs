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
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_reconcile_rir_create() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
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
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
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
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
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
}

