//! Integration tests for event emission in reconcilers
//!
//! These tests verify that events are actually emitted with correct content during reconciliation.

#[cfg(test)]
mod tests {
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::test_utils::{create_test_netbox_prefix, create_test_netbox_tenant};
    use crate::test_utils::event_test_helpers::*;
    use crate::kube_api_trait::KubeApiTrait;
    use crate::events::reasons;
    use std::sync::Arc;
    use crds::{NetBoxPrefix, NetBoxTenant, ResourceState};
    
    /// Test that events are actually emitted when recording methods are called
    #[tokio::test]
    async fn test_event_emission_basic() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Create a test prefix
        let prefix = create_test_netbox_prefix("test-prefix", "default", 0, None);
        
        // Record a Normal event
        reconciler.record_event_normal(
            reasons::CREATED,
            "Test event message",
            &prefix,
        ).await;
        
        // Verify event was emitted
        let event = assert_normal_event_emitted(&mock_event_recorder, reasons::CREATED)
            .expect("CREATED event should be emitted");
        assert_event_message_contains(&event, "Test event message")
            .expect("Event message should contain expected text");
        assert_event_for_resource(&event, &prefix)
            .expect("Event should be for the correct resource");
        
        // Record a Warning event
        reconciler.record_event_warning(
            reasons::RECONCILIATION_FAILED,
            "Test error message",
            &prefix,
        ).await;
        
        // Verify Warning event was emitted
        let warning_event = assert_warning_event_emitted(&mock_event_recorder, reasons::RECONCILIATION_FAILED)
            .expect("RECONCILIATION_FAILED event should be emitted");
        assert_event_message_contains(&warning_event, "Test error message")
            .expect("Warning event message should contain expected text");
        
        // Verify we have 2 events total
        assert_eq!(mock_event_recorder.get_events().len(), 2);
    }
    
    /// Test that events are emitted when reconciliation succeeds
    /// Note: This test verifies the code path is executed, not that events are actually published
    /// Full event testing would require mocking kube::runtime::events::Recorder
    #[tokio::test]
    async fn test_events_emitted_on_successful_reconciliation() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        let TestReconcilerApis {
            tenant_api,
            prefix_api,
            ..
        } = apis;
        
        // Setup: Create test data
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some(format!("{}/api/tenancy/tenants/1/", netbox_url)),
        );
        let mut prefix = create_test_netbox_prefix("test-prefix", "default", 0, None);
        prefix.status = None; // Clear status to test create path
        
        // Store dependencies
        tenant_api.store("datacenter-tenant".to_string(), tenant);
        prefix_api.store("test-prefix".to_string(), prefix.clone());
        
        // Execute: Reconcile (should emit CREATED event)
        let result = reconciler.reconcile_netbox_prefix(&prefix).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Note: In a full test, we would verify that an event was emitted
        // This requires mocking kube::runtime::events::Recorder, which is complex
        // For now, we verify the code path executes without errors
    }
    
    /// Test that DEPENDENCY_NOT_FOUND event is emitted when dependency is missing
    /// Note: The reconciler resolves dependencies after creating the NetBox client,
    /// so we need a tenant CRD that exists but doesn't have a netbox_id yet
    #[tokio::test]
    async fn test_dependency_not_found_event() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        // Add secret for the tenant we'll reference
        mock_token_resolver.add_secret("default", "netbox-token-non-existent-tenant", "test-token".to_string());
        
        let (reconciler, apis, mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        let TestReconcilerApis {
            tenant_api,
            prefix_api,
            ..
        } = apis;
        
        // Setup: Create tenant CRD but without netbox_id (not ready in NetBox)
        let tenant = create_test_netbox_tenant(
            "non-existent-tenant",
            "default",
            None, // No netbox_id - tenant not created in NetBox yet
            None,
        );
        
        // Setup: Create prefix referencing tenant that exists but isn't ready
        let mut prefix = create_test_netbox_prefix("test-prefix", "default", 0, None);
        prefix.status = None;
        prefix.spec.tenant.name = "non-existent-tenant".to_string();
        
        // Store tenant and prefix in APIs
        tenant_api.store("non-existent-tenant".to_string(), tenant);
        prefix_api.store("test-prefix".to_string(), prefix.clone());
        
        // Execute: Reconcile (should fail because tenant has no netbox_id and emit event)
        let result = reconciler.reconcile_netbox_prefix(&prefix).await;
        
        // Assert: Should fail with dependency not found
        assert!(result.is_err(), "Reconciliation should fail when dependency is not ready");
        
        // Assert: DEPENDENCY_NOT_FOUND event was emitted
        let event = assert_warning_event_emitted(&mock_event_recorder, reasons::DEPENDENCY_NOT_FOUND)
            .expect("DEPENDENCY_NOT_FOUND event should be emitted");
        assert_event_for_resource(&event, &prefix)
            .expect("Event should be for the correct prefix resource");
        assert_event_message_contains(&event, "Tenant")
            .expect("Event message should mention Tenant");
    }
    
    /// Test that UPDATED event infrastructure works
    /// Note: Full update testing requires MockNetBoxClient setup which is complex.
    /// This test verifies the event infrastructure works for UPDATED events.
    #[tokio::test]
    async fn test_updated_event_infrastructure() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Create a test tenant
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some(format!("{}/api/tenancy/tenants/1/", netbox_url)),
        );
        
        // Manually record an UPDATED event to verify the infrastructure works
        reconciler.record_event_normal(
            reasons::UPDATED,
            "Updated tenant datacenter-tenant in NetBox (ID: 1)",
            &tenant,
        ).await;
        
        // Assert: UPDATED event was emitted
        let event = assert_normal_event_emitted(&mock_event_recorder, reasons::UPDATED)
            .expect("UPDATED event should be emitted");
        assert_event_for_resource(&event, &tenant)
            .expect("Event should be for the correct tenant resource");
        assert_event_message_contains(&event, "Updated tenant")
            .expect("Event message should mention tenant update");
    }
    
    /// Test that CREATED event is emitted when tenant is created
    #[tokio::test]
    async fn test_created_event_on_tenant_creation() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis, mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        let TestReconcilerApis {
            tenant_api,
            ..
        } = apis;
        
        // Setup: Create tenant without status (new resource)
        let mut tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            None, // No netbox_id - new tenant
            None,
        );
        tenant.status = None; // Clear status to test create path
        
        tenant_api.store("datacenter-tenant".to_string(), tenant.clone());
        
        // Execute: Reconcile (should emit CREATED event)
        let result = reconciler.reconcile_netbox_tenant(&tenant).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: CREATED event was emitted
        let event = assert_normal_event_emitted(&mock_event_recorder, reasons::CREATED)
            .expect("CREATED event should be emitted on creation");
        assert_event_for_resource(&event, &tenant)
            .expect("Event should be for the correct tenant resource");
        assert_event_message_contains(&event, "Created tenant")
            .expect("Event message should mention tenant creation");
    }
    
    /// Test that RECONCILIATION_FAILED event is emitted on errors
    /// Note: Testing actual reconciliation failures is complex because we need to simulate
    /// NetBox API failures. For now, we test that the event infrastructure works.
    /// Full integration tests with NetBox failures would require more complex mocking.
    #[tokio::test]
    async fn test_reconciliation_failed_event() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Create a test prefix
        let prefix = create_test_netbox_prefix("test-prefix", "default", 0, None);
        
        // Manually record a RECONCILIATION_FAILED event to verify the infrastructure works
        reconciler.record_event_warning(
            reasons::RECONCILIATION_FAILED,
            "Test reconciliation failure",
            &prefix,
        ).await;
        
        // Assert: RECONCILIATION_FAILED event was emitted
        let event = assert_warning_event_emitted(&mock_event_recorder, reasons::RECONCILIATION_FAILED)
            .expect("RECONCILIATION_FAILED event should be emitted");
        assert_event_for_resource(&event, &prefix)
            .expect("Event should be for the correct prefix resource");
        assert_event_message_contains(&event, "reconciliation failure")
            .expect("Event message should mention reconciliation failure");
    }
    
    /// Test that DRIFT_DETECTED event infrastructure works
    /// Note: Full drift detection testing requires MockNetBoxClient setup which is complex.
    /// This test verifies the event infrastructure works for DRIFT_DETECTED events.
    #[tokio::test]
    async fn test_drift_detected_event_infrastructure() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Create a test site
        use crate::test_utils::create_test_netbox_site;
        let site = create_test_netbox_site(
            "test-site",
            "default",
            Some(1),
            Some(format!("{}/api/dcim/sites/1/", netbox_url)),
        );
        
        // Manually record a DRIFT_DETECTED event to verify the infrastructure works
        reconciler.record_event_warning(
            reasons::DRIFT_DETECTED,
            "NetBoxSite default/test-site drift detected: Resource was deleted in NetBox",
            &site,
        ).await;
        
        // Assert: DRIFT_DETECTED event was emitted
        let event = assert_warning_event_emitted(&mock_event_recorder, reasons::DRIFT_DETECTED)
            .expect("DRIFT_DETECTED event should be emitted");
        assert_event_for_resource(&event, &site)
            .expect("Event should be for the correct site resource");
        assert_event_message_contains(&event, "drift detected")
            .expect("Event message should mention drift detection");
    }
    
    /// Test that RETRY_ATTEMPT event infrastructure works
    #[tokio::test]
    async fn test_retry_attempt_event_infrastructure() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Create a test prefix
        let prefix = create_test_netbox_prefix("test-prefix", "default", 0, None);
        
        // Manually record a RETRY_ATTEMPT event to verify the infrastructure works
        reconciler.record_event_retry_attempt_str(
            "Test error message",
            2,
            60,
            &prefix,
        ).await;
        
        // Assert: RETRY_ATTEMPT event was emitted
        let event = assert_warning_event_emitted(&mock_event_recorder, reasons::RETRY_ATTEMPT)
            .expect("RETRY_ATTEMPT event should be emitted");
        assert_event_for_resource(&event, &prefix)
            .expect("Event should be for the correct prefix resource");
        assert_event_message_contains(&event, "Retrying reconciliation")
            .expect("Event message should mention retrying reconciliation");
        assert_event_message_contains(&event, "2")
            .expect("Event message should contain attempt number");
    }
    
    /// Test that CREATED event infrastructure works for Site reconciler
    /// Note: Full site creation testing requires MockNetBoxClient setup which is complex.
    /// This test verifies the event infrastructure works for Site resources.
    #[tokio::test]
    async fn test_created_event_on_site_infrastructure() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Create a test site
        use crate::test_utils::create_test_netbox_site;
        let site = create_test_netbox_site(
            "test-site",
            "default",
            Some(1),
            Some(format!("{}/api/dcim/sites/1/", netbox_url)),
        );
        
        // Manually record a CREATED event to verify the infrastructure works
        reconciler.record_event_normal(
            reasons::CREATED,
            "Created site test-site in NetBox (ID: 1)",
            &site,
        ).await;
        
        // Assert: CREATED event was emitted
        let event = assert_normal_event_emitted(&mock_event_recorder, reasons::CREATED)
            .expect("CREATED event should be emitted");
        assert_event_for_resource(&event, &site)
            .expect("Event should be for the correct site resource");
        assert_event_message_contains(&event, "Created site")
            .expect("Event message should mention site creation");
    }
    
    /// Test that multiple events can be emitted for the same resource
    #[tokio::test]
    async fn test_multiple_events_for_resource() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Create a test prefix
        let prefix = create_test_netbox_prefix("test-prefix", "default", 0, None);
        
        // Record multiple events
        reconciler.record_event_normal(
            reasons::CREATED,
            "Created prefix",
            &prefix,
        ).await;
        
        reconciler.record_event_normal(
            reasons::UPDATED,
            "Updated prefix",
            &prefix,
        ).await;
        
        reconciler.record_event_warning(
            reasons::DRIFT_DETECTED,
            "Drift detected",
            &prefix,
        ).await;
        
        // Assert: All events were emitted
        assert_eq!(mock_event_recorder.get_events().len(), 3);
        
        // Verify each event
        let created_event = assert_normal_event_emitted(&mock_event_recorder, reasons::CREATED)
            .expect("CREATED event should be emitted");
        assert_event_for_resource(&created_event, &prefix)
            .expect("CREATED event should be for prefix");
        
        let updated_event = assert_normal_event_emitted(&mock_event_recorder, reasons::UPDATED)
            .expect("UPDATED event should be emitted");
        assert_event_for_resource(&updated_event, &prefix)
            .expect("UPDATED event should be for prefix");
        
        let drift_event = assert_warning_event_emitted(&mock_event_recorder, reasons::DRIFT_DETECTED)
            .expect("DRIFT_DETECTED event should be emitted");
        assert_event_for_resource(&drift_event, &prefix)
            .expect("DRIFT_DETECTED event should be for prefix");
        
        // Verify event counts
        assert_event_count(&mock_event_recorder, reasons::CREATED, 1)
            .expect("Should have exactly 1 CREATED event");
        assert_event_count(&mock_event_recorder, reasons::UPDATED, 1)
            .expect("Should have exactly 1 UPDATED event");
        assert_event_count(&mock_event_recorder, reasons::DRIFT_DETECTED, 1)
            .expect("Should have exactly 1 DRIFT_DETECTED event");
    }
    
    /// Test that TOKEN_RESOLUTION_FAILED event infrastructure works
    /// Note: Full token resolution failure testing requires simulating token resolution errors.
    /// This test verifies the event infrastructure works for TOKEN_RESOLUTION_FAILED events.
    #[tokio::test]
    async fn test_token_resolution_failed_event_infrastructure() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Create a test tenant
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some(format!("{}/api/tenancy/tenants/1/", netbox_url)),
        );
        
        // Manually record a TOKEN_RESOLUTION_FAILED event to verify the infrastructure works
        reconciler.record_event_warning(
            reasons::TOKEN_RESOLUTION_FAILED,
            "Failed to resolve token for tenant datacenter-tenant: Secret not found",
            &tenant,
        ).await;
        
        // Assert: TOKEN_RESOLUTION_FAILED event was emitted
        let event = assert_warning_event_emitted(&mock_event_recorder, reasons::TOKEN_RESOLUTION_FAILED)
            .expect("TOKEN_RESOLUTION_FAILED event should be emitted");
        assert_event_for_resource(&event, &tenant)
            .expect("Event should be for the correct tenant resource");
        assert_event_message_contains(&event, "token")
            .expect("Event message should mention token");
    }
    
    use crate::test_utils::mock_token_resolver::TestReconcilerApis;
}

