//! Unit tests for NetBoxVLAN reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::kube_api_trait::KubeApiTrait;
    use crds::{NetBoxVLAN, ResourceState, VlanStatus};
    use std::sync::Arc;
    use chrono::Utc;

    /// Helper to create test NetBoxVLAN CRD
    fn create_test_netbox_vlan(
        name: &str,
        namespace: &str,
        vid: u16,
        tenant_name: &str,
        site_name: Option<&str>,
        netbox_id: Option<u64>,
    ) -> NetBoxVLAN {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        
        NetBoxVLAN {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxVLANSpec {
                vid,
                name: name.to_string(),
                site: site_name.map(|s| crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxSite".to_string(),
                    name: s.to_string(),
                    namespace: Some(namespace.to_string()),
                }),
                group: None,
                tenant: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxTenant".to_string(),
                    name: tenant_name.to_string(),
                    namespace: Some(namespace.to_string()),
                },
                role: None,
                status: VlanStatus::Active,
                description: None,
                comments: None,
            },
            status: netbox_id.map(|id| crds::NetBoxVLANStatus {
                netbox_id: Some(id),
                netbox_url: Some(format!("http://test-netbox/api/ipam/vlans/{}/", id)),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        }
    }

    /// Helper to create test NetBox Vlan model
    fn create_test_vlan(
        id: u64,
        vid: u16,
        name: &str,
        tenant_id: u64,
        site_id: Option<u64>,
        base_url: &str,
    ) -> netbox_client::Vlan {
        use netbox_client::{NestedTenant, NestedSite, VlanStatus as NetBoxVlanStatus};
        
        netbox_client::Vlan {
            id,
            url: format!("{}/api/ipam/vlans/{}/", base_url, id),
            display: name.to_string(),
            site: site_id.map(|s_id| NestedSite {
                id: s_id,
                url: format!("{}/api/dcim/sites/{}/", base_url, s_id),
                display: "test-site".to_string(),
                name: "test-site".to_string(),
                slug: "test-site".to_string(),
            }),
            group: None,
            vid,
            name: name.to_string(),
            tenant: Some(NestedTenant {
                id: tenant_id,
                url: format!("{}/api/tenancy/tenants/{}/", base_url, tenant_id),
                display: "datacenter-tenant".to_string(),
                name: "datacenter-tenant".to_string(),
                slug: "datacenter-tenant".to_string(),
            }),
            status: NetBoxVlanStatus::Active,
            role: None,
            description: String::new(),
            comments: String::new(),
            tags: vec![],
            custom_fields: serde_json::json!({}),
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_reconcile_vlan_create() {
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
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant in API
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        // Setup: Create VLAN CRD without status
        let mut vlan = create_test_netbox_vlan("test-vlan", "default", 100, "datacenter-tenant", None, None);
        vlan.status = None;
        apis.vlan_api.store("test-vlan".to_string(), vlan.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_vlan(&vlan).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.vlan_api.as_ref().get("test-vlan").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }

    #[tokio::test]
    async fn test_reconcile_vlan_idempotent() {
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
        
        // Setup: Add VLAN to mock NetBox client
        let netbox_vlan = create_test_vlan(1, 100, "test-vlan", 1, None, "http://test-netbox");
        mock_token_resolver.mock_client().add_vlan(netbox_vlan);
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant in API
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        // Setup: Create VLAN CRD with status (already created)
        let vlan = create_test_netbox_vlan("test-vlan", "default", 100, "datacenter-tenant", None, Some(1));
        apis.vlan_api.store("test-vlan".to_string(), vlan.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_vlan(&vlan).await;
        
        // Assert: Should succeed (idempotent)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain unchanged
        let updated_crd = apis.vlan_api.as_ref().get("test-vlan").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should remain 1");
    }

    #[tokio::test]
    async fn test_reconcile_vlan_tenant_not_found() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-nonexistent-tenant", "test-token".to_string());
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create VLAN CRD with tenant that doesn't exist
        let mut vlan = create_test_netbox_vlan("test-vlan", "default", 100, "nonexistent-tenant", None, None);
        vlan.status = None;
        apis.vlan_api.store("test-vlan".to_string(), vlan.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_vlan(&vlan).await;
        
        // Assert: Should fail with InvalidConfig error (tenant not found)
        assert!(result.is_err(), "Reconciliation should fail when tenant not found");
        match result.unwrap_err() {
            crate::error::ControllerError::InvalidConfig(_) => {
                // Expected error type
            }
            e => panic!("Expected InvalidConfig error, got: {:?}", e),
        }
    }
}

