//! Unit tests for NetBoxAggregate reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::kube_api_trait::KubeApiTrait;
    use crds::{NetBoxAggregate, ResourceState};
    use std::sync::Arc;
    use std::str::FromStr;
    use ipnet::IpNet;

    /// Helper to create test NetBoxAggregate CRD
    fn create_test_netbox_aggregate(
        name: &str,
        namespace: &str,
        netbox_id: Option<u64>,
        prefix: &str,
    ) -> NetBoxAggregate {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        
        NetBoxAggregate {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxAggregateSpec {
                prefix: prefix.to_string(),
                rir: None,
                date_allocated: None,
                description: None,
                comments: None,
            },
            status: netbox_id.map(|id| crds::NetBoxAggregateStatus {
                netbox_id: Some(id),
                netbox_url: Some(format!("http://test-netbox/api/ipam/aggregates/{}/", id)),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        }
    }

    #[tokio::test]
    async fn test_reconcile_aggregate_create() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        // Setup: Add secret for shared resource (aggregates use shared resource resolution)
        // The mock resolver will use the first available secret or create a default one
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create aggregate CRD without status
        let mut aggregate = create_test_netbox_aggregate("test-aggregate", "default", None, "192.168.0.0/16");
        aggregate.status = None;
        apis.aggregate_api.store("test-aggregate".to_string(), aggregate.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_aggregate(&aggregate).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.aggregate_api.as_ref().get("test-aggregate").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }

    #[tokio::test]
    async fn test_reconcile_aggregate_idempotent() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        // Setup: Add secret
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Add aggregate to mock NetBox client
        let netbox_aggregate = crate::test_utils::create_test_aggregate(
            1,
            "192.168.0.0/16",
            "http://test-netbox",
        );
        mock_token_resolver.mock_client().add_aggregate(netbox_aggregate);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create aggregate CRD with status (already created)
        let aggregate = create_test_netbox_aggregate("test-aggregate", "default", Some(1), "192.168.0.0/16");
        apis.aggregate_api.store("test-aggregate".to_string(), aggregate.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_aggregate(&aggregate).await;
        
        // Assert: Should succeed (idempotent)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain unchanged
        let updated_crd = apis.aggregate_api.as_ref().get("test-aggregate").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should remain 1");
    }

    #[tokio::test]
    async fn test_reconcile_aggregate_invalid_prefix() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create aggregate with invalid prefix format
        let mut aggregate = create_test_netbox_aggregate("test-aggregate", "default", None, "invalid-prefix");
        aggregate.status = None;
        apis.aggregate_api.store("test-aggregate".to_string(), aggregate.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_aggregate(&aggregate).await;
        
        // Assert: Should fail with InvalidIPFormat error
        assert!(result.is_err(), "Reconciliation should fail for invalid prefix");
        match result.unwrap_err() {
            crate::error::ControllerError::InvalidIPFormat(_) => {
                // Expected error type
            }
            e => panic!("Expected InvalidIPFormat error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_reconcile_aggregate_rir_not_found() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create aggregate with RIR that doesn't exist
        let mut aggregate = create_test_netbox_aggregate("test-aggregate", "default", None, "192.168.0.0/16");
        aggregate.spec.rir = Some("nonexistent-rir".to_string());
        aggregate.status = None;
        apis.aggregate_api.store("test-aggregate".to_string(), aggregate.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_aggregate(&aggregate).await;
        
        // Assert: Should fail with InvalidConfig error (RIR not found)
        assert!(result.is_err(), "Reconciliation should fail when RIR not found");
        match result.unwrap_err() {
            crate::error::ControllerError::InvalidConfig(_) => {
                // Expected error type
            }
            e => panic!("Expected InvalidConfig error, got: {:?}", e),
        }
        
        // Assert: Status should be updated with error
        let updated_crd = apis.aggregate_api.as_ref().get("test-aggregate").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.state, ResourceState::Failed, "State should be Failed");
        assert!(status.error.is_some(), "Error should be set");
    }

    #[tokio::test]
    async fn test_reconcile_aggregate_with_rir() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Add RIR to mock NetBox client
        let rir = netbox_client::Rir {
            id: 1,
            url: "http://test-netbox/api/ipam/rirs/1/".to_string(),
            display: "ARIN".to_string(),
            name: "ARIN".to_string(),
            slug: "arin".to_string(),
            description: None,
            is_private: false,
            tags: vec![],
            created: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };
        mock_token_resolver.mock_client().add_rir(rir);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create aggregate with RIR
        let mut aggregate = create_test_netbox_aggregate("test-aggregate", "default", None, "192.168.0.0/16");
        aggregate.spec.rir = Some("ARIN".to_string());
        aggregate.status = None;
        apis.aggregate_api.store("test-aggregate".to_string(), aggregate.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_aggregate(&aggregate).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated
        let updated_crd = apis.aggregate_api.as_ref().get("test-aggregate").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
}

