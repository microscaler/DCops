//! Unit tests for reconcile_helpers module

#[cfg(test)]
mod tests {
    use super::super::reconcile_helpers;
    use netbox_client::NestedTag;
    
    fn create_nested_tag(id: u64, name: &str) -> NestedTag {
        NestedTag {
            id,
            url: format!("http://test/api/extras/tags/{}/", id),
            display: name.to_string(),
            name: name.to_string(),
            slug: name.to_string(),
        }
    }
    
    fn create_tag_ref(name: &str) -> crds::NetBoxResourceReference {
        crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTag".to_string(),
            name: name.to_string(),
            namespace: None,
        }
    }
    
    #[test]
    fn test_tags_differ_empty_vs_empty() {
        let existing: Vec<NestedTag> = vec![];
        let desired: Option<Vec<crds::NetBoxResourceReference>> = None;
        
        assert!(!reconcile_helpers::tags_differ(&existing, &desired), 
            "Empty tags should not differ");
    }
    
    #[test]
    fn test_tags_differ_empty_vs_some() {
        let existing: Vec<NestedTag> = vec![];
        let desired = Some(vec![create_tag_ref("tag1")]);
        
        assert!(reconcile_helpers::tags_differ(&existing, &desired), 
            "Empty existing vs some desired should differ");
    }
    
    #[test]
    fn test_tags_differ_some_vs_empty() {
        let existing = vec![create_nested_tag(1, "tag1")];
        let desired: Option<Vec<crds::NetBoxResourceReference>> = None;
        
        assert!(reconcile_helpers::tags_differ(&existing, &desired), 
            "Some existing vs empty desired should differ");
    }
    
    #[test]
    fn test_tags_differ_same_tags() {
        let existing = vec![
            create_nested_tag(1, "tag1"),
            create_nested_tag(2, "tag2"),
        ];
        let desired = Some(vec![
            create_tag_ref("tag1"),
            create_tag_ref("tag2"),
        ]);
        
        assert!(!reconcile_helpers::tags_differ(&existing, &desired), 
            "Same tags should not differ");
    }
    
    #[test]
    fn test_tags_differ_different_tags() {
        let existing = vec![create_nested_tag(1, "tag1")];
        let desired = Some(vec![create_tag_ref("tag2")]);
        
        assert!(reconcile_helpers::tags_differ(&existing, &desired), 
            "Different tags should differ");
    }
    
    #[test]
    fn test_tags_differ_different_order() {
        let existing = vec![
            create_nested_tag(1, "tag1"),
            create_nested_tag(2, "tag2"),
        ];
        let desired = Some(vec![
            create_tag_ref("tag2"),
            create_tag_ref("tag1"),
        ]);
        
        assert!(!reconcile_helpers::tags_differ(&existing, &desired), 
            "Tags in different order should not differ (order doesn't matter)");
    }
    
    #[test]
    fn test_tags_differ_extra_existing() {
        let existing = vec![
            create_nested_tag(1, "tag1"),
            create_nested_tag(2, "tag2"),
        ];
        let desired = Some(vec![create_tag_ref("tag1")]);
        
        assert!(reconcile_helpers::tags_differ(&existing, &desired), 
            "Extra existing tags should differ");
    }
    
    #[test]
    fn test_tags_differ_extra_desired() {
        let existing = vec![create_nested_tag(1, "tag1")];
        let desired = Some(vec![
            create_tag_ref("tag1"),
            create_tag_ref("tag2"),
        ]);
        
        assert!(reconcile_helpers::tags_differ(&existing, &desired), 
            "Extra desired tags should differ");
    }
    
    #[test]
    fn test_tags_differ_case_sensitive() {
        let existing = vec![create_nested_tag(1, "Tag1")];
        let desired = Some(vec![create_tag_ref("tag1")]);
        
        assert!(reconcile_helpers::tags_differ(&existing, &desired), 
            "Tags should be case-sensitive");
    }
}

// Tests for validate_status_and_drift()
#[cfg(test)]
mod validate_status_and_drift_tests {
    use crate::reconcile_helpers::{self, DriftCheckResult};
    use crate::error::ControllerError;
    use crds::{NetBoxSiteStatus, ResourceState};
    use netbox_client::{Site, NetBoxError};

    fn create_test_site(id: u64) -> Site {
        Site {
            id,
            url: format!("http://test/api/dcim/sites/{}/", id),
            display: format!("Test Site {}", id),
            name: format!("test-site-{}", id),
            slug: format!("test-site-{}", id),
            status: netbox_client::SiteStatus::Active,
            region: None,
            tenant: None,
            facility: None,
            time_zone: None,
            description: None,
            physical_address: None,
            shipping_address: None,
            latitude: None,
            longitude: None,
            contact_name: None,
            contact_phone: None,
            contact_email: None,
            asn: None,
            comments: String::new(),
            tags: vec![],
            custom_fields: None,
            created: String::new(),
            last_updated: String::new(),
            circuit_count: 0,
            device_count: 0,
            prefix_count: 0,
            rack_count: 0,
            virtualmachine_count: 0,
            vlan_count: 0,
        }
    }

    fn create_status(state: ResourceState, netbox_id: Option<u64>) -> NetBoxSiteStatus {
        NetBoxSiteStatus {
            netbox_id,
            netbox_url: netbox_id.map(|id| format!("http://test/api/dcim/sites/{}/", id)),
            state,
            error: None,
        }
    }

    #[tokio::test]
    async fn test_validate_status_and_drift_no_status() {
        let result = reconcile_helpers::validate_status_and_drift::<Site, _, _>(
            None::<&NetBoxSiteStatus>,
            "NetBoxSite",
            "default",
            "test-site",
            |_| async { unreachable!() },
        ).await;

        assert!(result.is_ok());
        match result.unwrap() {
            DriftCheckResult::Recreate => {}
            _ => panic!("Expected Recreate for no status"),
        }
    }

    #[tokio::test]
    async fn test_validate_status_and_drift_failed_with_invalid_id() {
        let status = create_status(ResourceState::Failed, Some(0));
        let result = reconcile_helpers::validate_status_and_drift::<Site, _, _>(
            Some(&status),
            "NetBoxSite",
            "default",
            "test-site",
            |_| async { unreachable!() },
        ).await;

        assert!(result.is_ok());
        match result.unwrap() {
            DriftCheckResult::StatusCleared { message } => {
                assert!(message.contains("invalid netbox_id"));
            }
            _ => panic!("Expected StatusCleared for Failed with invalid ID"),
        }
    }

    #[tokio::test]
    async fn test_validate_status_and_drift_failed_with_valid_id_exists() {
        let status = create_status(ResourceState::Failed, Some(42));
        let site = create_test_site(42);
        
        let result = reconcile_helpers::validate_status_and_drift::<Site, _, _>(
            Some(&status),
            "NetBoxSite",
            "default",
            "test-site",
            |id| {
                let site = create_test_site(id);
                async move { Ok(site) }
            },
        ).await;

        assert!(result.is_ok());
        match result.unwrap() {
            DriftCheckResult::UseExisting(resource) => {
                assert_eq!(resource.id, 42);
            }
            _ => panic!("Expected UseExisting for Failed with valid ID and existing resource"),
        }
    }

    #[tokio::test]
    async fn test_validate_status_and_drift_failed_with_valid_id_not_found() {
        let status = create_status(ResourceState::Failed, Some(42));
        
        let result = reconcile_helpers::validate_status_and_drift::<Site, _, _>(
            Some(&status),
            "NetBoxSite",
            "default",
            "test-site",
            |_| async { Err(NetBoxError::NotFound("Site not found".to_string())) },
        ).await;

        assert!(result.is_ok());
        match result.unwrap() {
            DriftCheckResult::StatusCleared { message } => {
                assert!(message.contains("doesn't exist"));
            }
            _ => panic!("Expected StatusCleared for Failed with valid ID but resource not found"),
        }
    }

    #[tokio::test]
    async fn test_validate_status_and_drift_failed_with_valid_id_error() {
        let status = create_status(ResourceState::Failed, Some(42));
        
        let result = reconcile_helpers::validate_status_and_drift::<Site, _, _>(
            Some(&status),
            "NetBoxSite",
            "default",
            "test-site",
            |_| async { Err(NetBoxError::HttpError(500, "Internal Server Error".to_string())) },
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ControllerError::NetBox(NetBoxError::HttpError(500, _)) => {}
            _ => panic!("Expected NetBox error for Failed with valid ID but API error"),
        }
    }

    #[tokio::test]
    async fn test_validate_status_and_drift_failed_no_id() {
        let status = create_status(ResourceState::Failed, None);
        
        let result = reconcile_helpers::validate_status_and_drift::<Site, _, _>(
            Some(&status),
            "NetBoxSite",
            "default",
            "test-site",
            |_| async { unreachable!() },
        ).await;

        assert!(result.is_ok());
        match result.unwrap() {
            DriftCheckResult::Recreate => {}
            _ => panic!("Expected Recreate for Failed with no ID"),
        }
    }

    #[tokio::test]
    async fn test_validate_status_and_drift_created_with_invalid_id() {
        let status = create_status(ResourceState::Created, Some(0));
        
        let result = reconcile_helpers::validate_status_and_drift::<Site, _, _>(
            Some(&status),
            "NetBoxSite",
            "default",
            "test-site",
            |_| async { unreachable!() },
        ).await;

        assert!(result.is_ok());
        match result.unwrap() {
            DriftCheckResult::StatusCleared { message } => {
                assert!(message.contains("Invalid netbox_id"));
            }
            _ => panic!("Expected StatusCleared for Created with invalid ID"),
        }
    }

    #[tokio::test]
    async fn test_validate_status_and_drift_created_with_valid_id_exists() {
        let status = create_status(ResourceState::Created, Some(42));
        let site = create_test_site(42);
        
        let result = reconcile_helpers::validate_status_and_drift::<Site, _, _>(
            Some(&status),
            "NetBoxSite",
            "default",
            "test-site",
            |id| {
                let site = create_test_site(id);
                async move { Ok(site) }
            },
        ).await;

        assert!(result.is_ok());
        match result.unwrap() {
            DriftCheckResult::UseExisting(resource) => {
                assert_eq!(resource.id, 42);
            }
            _ => panic!("Expected UseExisting for Created with valid ID and existing resource"),
        }
    }

    #[tokio::test]
    async fn test_validate_status_and_drift_created_with_valid_id_not_found() {
        let status = create_status(ResourceState::Created, Some(42));
        
        let result = reconcile_helpers::validate_status_and_drift::<Site, _, _>(
            Some(&status),
            "NetBoxSite",
            "default",
            "test-site",
            |_| async { Err(NetBoxError::NotFound("Site not found".to_string())) },
        ).await;

        assert!(result.is_ok());
        match result.unwrap() {
            DriftCheckResult::StatusCleared { message } => {
                assert!(message.contains("was deleted"));
            }
            _ => panic!("Expected StatusCleared for Created with valid ID but resource not found"),
        }
    }

    #[tokio::test]
    async fn test_validate_status_and_drift_created_with_valid_id_error() {
        let status = create_status(ResourceState::Created, Some(42));
        
        let result = reconcile_helpers::validate_status_and_drift::<Site, _, _>(
            Some(&status),
            "NetBoxSite",
            "default",
            "test-site",
            |_| async { Err(NetBoxError::HttpError(500, "Internal Server Error".to_string())) },
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ControllerError::NetBox(NetBoxError::HttpError(500, _)) => {}
            _ => panic!("Expected NetBox error for Created with valid ID but API error"),
        }
    }

    #[tokio::test]
    async fn test_validate_status_and_drift_created_no_id() {
        let status = create_status(ResourceState::Created, None);
        
        let result = reconcile_helpers::validate_status_and_drift::<Site, _, _>(
            Some(&status),
            "NetBoxSite",
            "default",
            "test-site",
            |_| async { unreachable!() },
        ).await;

        assert!(result.is_ok());
        match result.unwrap() {
            DriftCheckResult::Recreate => {}
            _ => panic!("Expected Recreate for Created with no ID"),
        }
    }

    #[tokio::test]
    async fn test_validate_status_and_drift_pending_state() {
        let status = create_status(ResourceState::Pending, Some(42));
        
        let result = reconcile_helpers::validate_status_and_drift::<Site, _, _>(
            Some(&status),
            "NetBoxSite",
            "default",
            "test-site",
            |_| async { unreachable!() },
        ).await;

        assert!(result.is_ok());
        match result.unwrap() {
            DriftCheckResult::Recreate => {}
            _ => panic!("Expected Recreate for Pending state"),
        }
    }

    #[tokio::test]
    async fn test_validate_status_and_drift_updated_state() {
        let status = create_status(ResourceState::Updated, Some(42));
        
        let result = reconcile_helpers::validate_status_and_drift::<Site, _, _>(
            Some(&status),
            "NetBoxSite",
            "default",
            "test-site",
            |_| async { unreachable!() },
        ).await;

        assert!(result.is_ok());
        match result.unwrap() {
            DriftCheckResult::Recreate => {}
            _ => panic!("Expected Recreate for Updated state"),
        }
    }
}

// Tests for check_and_update_existing()
#[cfg(test)]
mod check_and_update_existing_tests {
    use crate::reconcile_helpers;
    use crate::error::ControllerError;
    use netbox_client::{Site, NetBoxError, MockNetBoxClient, NetBoxClientTrait};
    use std::sync::Arc;

    fn create_test_site(id: u64) -> Site {
        Site {
            id,
            url: format!("http://test/api/dcim/sites/{}/", id),
            display: format!("Test Site {}", id),
            name: format!("test-site-{}", id),
            slug: format!("test-site-{}", id),
            status: netbox_client::SiteStatus::Active,
            region: None,
            tenant: None,
            facility: None,
            time_zone: None,
            description: None,
            physical_address: None,
            shipping_address: None,
            latitude: None,
            longitude: None,
            contact_name: None,
            contact_phone: None,
            contact_email: None,
            asn: None,
            comments: String::new(),
            tags: vec![],
            custom_fields: None,
            created: String::new(),
            last_updated: String::new(),
            circuit_count: 0,
            device_count: 0,
            prefix_count: 0,
            rack_count: 0,
            virtualmachine_count: 0,
            vlan_count: 0,
        }
    }

    #[tokio::test]
    async fn test_check_and_update_existing_resource_exists_up_to_date() {
        let client = Arc::new(MockNetBoxClient::new()) as Arc<dyn NetBoxClientTrait>;
        let site = create_test_site(42);
        
        let result = reconcile_helpers::check_and_update_existing(
            client.as_ref(),
            42,
            "NetBoxSite",
            async { Ok(site.clone()) },
            |_| false, // No update needed
            async { unreachable!() },
        ).await;

        assert!(result.is_ok());
        let resource = result.unwrap();
        assert!(resource.is_some());
        assert_eq!(resource.unwrap().id(), 42);
    }

    #[tokio::test]
    async fn test_check_and_update_existing_resource_exists_needs_update() {
        let client = Arc::new(MockNetBoxClient::new()) as Arc<dyn NetBoxClientTrait>;
        let site = create_test_site(42);
        let updated_site = create_test_site(42);
        
        let result = reconcile_helpers::check_and_update_existing(
            client.as_ref(),
            42,
            "NetBoxSite",
            async { Ok(site.clone()) },
            |_| true, // Update needed
            async { Ok(updated_site.clone()) },
        ).await;

        assert!(result.is_ok());
        let resource = result.unwrap();
        assert!(resource.is_some());
        assert_eq!(resource.unwrap().id(), 42);
    }

    #[tokio::test]
    async fn test_check_and_update_existing_resource_not_found() {
        let client = Arc::new(MockNetBoxClient::new()) as Arc<dyn NetBoxClientTrait>;
        
        let result = reconcile_helpers::check_and_update_existing(
            client.as_ref(),
            42,
            "NetBoxSite",
            async { Err(NetBoxError::NotFound("Site not found".to_string())) },
            |_| false,
            async { unreachable!() },
        ).await;

        assert!(result.is_ok());
        let resource = result.unwrap();
        assert!(resource.is_none()); // Signal to recreate
    }

    #[tokio::test]
    async fn test_check_and_update_existing_update_fails() {
        let client = Arc::new(MockNetBoxClient::new()) as Arc<dyn NetBoxClientTrait>;
        let site = create_test_site(42);
        
        let result = reconcile_helpers::check_and_update_existing(
            client.as_ref(),
            42,
            "NetBoxSite",
            async { Ok(site.clone()) },
            |_| true, // Update needed
            async { Err(NetBoxError::HttpError(500, "Update failed".to_string())) },
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ControllerError::NetBox(NetBoxError::HttpError(500, _)) => {}
            _ => panic!("Expected NetBox error for update failure"),
        }
    }

    #[tokio::test]
    async fn test_check_and_update_existing_get_error() {
        let client = Arc::new(MockNetBoxClient::new()) as Arc<dyn NetBoxClientTrait>;
        
        let result = reconcile_helpers::check_and_update_existing(
            client.as_ref(),
            42,
            "NetBoxSite",
            async { Err(NetBoxError::HttpError(500, "Get failed".to_string())) },
            |_| false,
            async { unreachable!() },
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ControllerError::NetBox(NetBoxError::HttpError(500, _)) => {}
            _ => panic!("Expected NetBox error for get failure"),
        }
    }
}

// Tests for resolve_required_dependency_id() and resolve_optional_dependency_id()
#[cfg(test)]
mod resolve_dependency_tests {
    use crate::reconcile_helpers;
    use crate::error::ControllerError;
    use crate::kube_api_trait::mock::MockKubeApi;
    use crds::{NetBoxTenant, NetBoxTenantStatus, ResourceState};
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

    fn create_test_tenant_crd(name: &str, netbox_id: Option<u64>) -> NetBoxTenant {
        NetBoxTenant {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxTenantSpec {
                name: name.to_string(),
                slug: name.to_string(),
                description: None,
                comments: None,
                tags: None,
            },
            status: Some(NetBoxTenantStatus {
                netbox_id,
                netbox_url: netbox_id.map(|id| format!("http://test/api/tenancy/tenants/{}/", id)),
                state: if netbox_id.is_some() {
                    ResourceState::Created
                } else {
                    ResourceState::Pending
                },
                error: None,
            }),
        }
    }

    #[tokio::test]
    async fn test_resolve_required_dependency_id_success() {
        let api = MockKubeApi::new();
        let tenant_crd = create_test_tenant_crd("test-tenant", Some(42));
        api.store("test-tenant".to_string(), tenant_crd);

        let result = reconcile_helpers::resolve_required_dependency_id(
            &api,
            "test-tenant",
            "Tenant",
            "test-resource",
            |crd| crd.status.as_ref(),
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_resolve_required_dependency_id_not_found() {
        let api = MockKubeApi::new();

        let result = reconcile_helpers::resolve_required_dependency_id(
            &api,
            "missing-tenant",
            "Tenant",
            "test-resource",
            |crd| crd.status.as_ref(),
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ControllerError::InvalidConfig(msg) => {
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected InvalidConfig for missing CRD"),
        }
    }

    #[tokio::test]
    async fn test_resolve_required_dependency_id_no_status() {
        let api = MockKubeApi::new();
        let mut tenant_crd = create_test_tenant_crd("test-tenant", Some(42));
        tenant_crd.status = None;
        api.store("test-tenant".to_string(), tenant_crd);

        let result = reconcile_helpers::resolve_required_dependency_id(
            &api,
            "test-tenant",
            "Tenant",
            "test-resource",
            |crd| crd.status.as_ref(),
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ControllerError::InvalidConfig(msg) => {
                assert!(msg.contains("no status"));
            }
            _ => panic!("Expected InvalidConfig for no status"),
        }
    }

    #[tokio::test]
    async fn test_resolve_required_dependency_id_no_netbox_id() {
        let api = MockKubeApi::new();
        let tenant_crd = create_test_tenant_crd("test-tenant", None);
        api.store("test-tenant".to_string(), tenant_crd);

        let result = reconcile_helpers::resolve_required_dependency_id(
            &api,
            "test-tenant",
            "Tenant",
            "test-resource",
            |crd| crd.status.as_ref(),
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ControllerError::InvalidConfig(msg) => {
                assert!(msg.contains("not been created in NetBox yet"));
            }
            _ => panic!("Expected InvalidConfig for no netbox_id"),
        }
    }

    #[tokio::test]
    async fn test_resolve_optional_dependency_id_success() {
        let api = MockKubeApi::new();
        let tenant_crd = create_test_tenant_crd("test-tenant", Some(42));
        api.store("test-tenant".to_string(), tenant_crd);

        let reference = Some(crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "test-tenant".to_string(),
            namespace: None,
        });

        let result = reconcile_helpers::resolve_optional_dependency_id(
            &api,
            reference.as_ref(),
            "NetBoxTenant",
            "tenant",
            "test-resource",
            |crd| crd.status.as_ref(),
        ).await;

        assert_eq!(result, Some(42));
    }

    #[tokio::test]
    async fn test_resolve_optional_dependency_id_none() {
        let api = MockKubeApi::new();

        let result = reconcile_helpers::resolve_optional_dependency_id(
            &api,
            None,
            "NetBoxTenant",
            "tenant",
            "test-resource",
            |crd| crd.status.as_ref(),
        ).await;

        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_resolve_optional_dependency_id_wrong_kind() {
        let api = MockKubeApi::new();

        let reference = Some(crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxSite".to_string(), // Wrong kind
            name: "test-tenant".to_string(),
            namespace: None,
        });

        let result = reconcile_helpers::resolve_optional_dependency_id(
            &api,
            reference.as_ref(),
            "NetBoxTenant",
            "tenant",
            "test-resource",
            |crd| crd.status.as_ref(),
        ).await;

        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_resolve_optional_dependency_id_not_found() {
        let api = MockKubeApi::new();

        let reference = Some(crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "missing-tenant".to_string(),
            namespace: None,
        });

        let result = reconcile_helpers::resolve_optional_dependency_id(
            &api,
            reference.as_ref(),
            "NetBoxTenant",
            "tenant",
            "test-resource",
            |crd| crd.status.as_ref(),
        ).await;

        assert_eq!(result, None); // Optional dependencies return None on error
    }

    #[tokio::test]
    async fn test_resolve_optional_dependency_id_no_netbox_id() {
        let api = MockKubeApi::new();
        let tenant_crd = create_test_tenant_crd("test-tenant", None);
        api.store("test-tenant".to_string(), tenant_crd);

        let reference = Some(crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "test-tenant".to_string(),
            namespace: None,
        });

        let result = reconcile_helpers::resolve_optional_dependency_id(
            &api,
            reference.as_ref(),
            "NetBoxTenant",
            "tenant",
            "test-resource",
            |crd| crd.status.as_ref(),
        ).await;

        assert_eq!(result, None); // Optional dependencies return None if not ready
    }

    #[tokio::test]
    async fn test_resolve_optional_dependency_id_invalid_id_zero() {
        let api = MockKubeApi::new();
        let tenant_crd = create_test_tenant_crd("test-tenant", Some(0)); // Invalid ID
        api.store("test-tenant".to_string(), tenant_crd);

        let reference = Some(crds::NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "test-tenant".to_string(),
            namespace: None,
        });

        let result = reconcile_helpers::resolve_optional_dependency_id(
            &api,
            reference.as_ref(),
            "NetBoxTenant",
            "tenant",
            "test-resource",
            |crd| crd.status.as_ref(),
        ).await;

        assert_eq!(result, None); // Invalid ID (0) is filtered out
    }
}
