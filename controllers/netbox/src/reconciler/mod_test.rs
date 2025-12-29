//! Tests for the Reconciler module

#[cfg(test)]
mod tests {
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::test_utils::{create_test_netbox_prefix, create_test_prefix};
    use crate::kube_api_trait::KubeApiTrait;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_get_backoff_for_resource() {
        let netbox_url = "http://netbox.test".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        // Test initial backoff (should be 60 seconds for first error)
        let (backoff, error_count) = reconciler.get_backoff_for_resource("test-resource");
        assert_eq!(error_count, 0);
        assert!(backoff >= 60 && backoff <= 600); // Between 1 min and 10 min

        // Test that same resource returns same state
        let (backoff2, error_count2) = reconciler.get_backoff_for_resource("test-resource");
        assert_eq!(error_count, error_count2);
        assert_eq!(backoff, backoff2);
    }

    #[tokio::test]
    async fn test_increment_error() {
        let netbox_url = "http://netbox.test".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        let resource_key = "test-resource";

        // Initially no errors
        let (_, error_count) = reconciler.get_backoff_for_resource(resource_key);
        assert_eq!(error_count, 0);

        // Increment error
        reconciler.increment_error(resource_key);

        // Check error count increased
        let (_, error_count) = reconciler.get_backoff_for_resource(resource_key);
        assert_eq!(error_count, 1);

        // Increment again
        reconciler.increment_error(resource_key);
        let (_, error_count) = reconciler.get_backoff_for_resource(resource_key);
        assert_eq!(error_count, 2);
    }

    #[tokio::test]
    async fn test_reset_error() {
        let netbox_url = "http://netbox.test".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        let resource_key = "test-resource";

        // Increment errors
        reconciler.increment_error(resource_key);
        reconciler.increment_error(resource_key);
        let (_, error_count) = reconciler.get_backoff_for_resource(resource_key);
        assert_eq!(error_count, 2);

        // Reset errors
        reconciler.reset_error(resource_key);

        // Check error count reset
        let (_, error_count) = reconciler.get_backoff_for_resource(resource_key);
        assert_eq!(error_count, 0);
    }

    #[tokio::test]
    async fn test_startup_reconciliation_no_prefixes() {
        let netbox_url = "http://netbox.test".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        // Run startup reconciliation with no prefixes
        let result = reconciler.startup_reconciliation().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_startup_reconciliation_prefix_with_netbox_id() {
        let netbox_url = "http://netbox.test".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        // Create a prefix that already has a netbox_id
        let prefix = create_test_netbox_prefix(
            "test-prefix",
            "default",
            100, // Already has netbox_id
            Some("http://netbox.test/api/ipam/prefixes/100/".to_string()),
        );
        apis.prefix_api.store("test-prefix".to_string(), prefix);

        // Run startup reconciliation
        let result = reconciler.startup_reconciliation().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_startup_reconciliation_prefix_without_netbox_id_found() {
        let netbox_url = "http://netbox.test".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));

        // Add secret for tenant (required for token resolution)
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());

        // Add the prefix to the mock NetBox client
        let netbox_prefix = create_test_prefix(
            100,
            "192.168.1.0/24",
            "http://netbox.test",
        );
        mock_token_resolver.mock_client().add_prefix(netbox_prefix);

        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        // Create a prefix without netbox_id (status is None)
        // The prefix spec must match the NetBox prefix we added above (192.168.1.0/24)
        let mut prefix = create_test_netbox_prefix(
            "test-prefix",
            "default",
            0, // Will be set by startup reconciliation
            None,
        );
        prefix.status = None; // Clear status to test startup reconciliation
        // Ensure the prefix spec matches what we added to NetBox
        prefix.spec.prefix = "192.168.1.0/24".to_string();
        apis.prefix_api.store("test-prefix".to_string(), prefix);

        // Run startup reconciliation
        let result = reconciler.startup_reconciliation().await;
        assert!(result.is_ok());

        // Verify the prefix now has a netbox_id in status
        let updated_prefix = apis.prefix_api.as_ref().get("test-prefix").await.unwrap();
        assert!(updated_prefix.status.as_ref().unwrap().netbox_id.is_some());
        assert_eq!(updated_prefix.status.as_ref().unwrap().netbox_id, Some(100));
    }

    #[tokio::test]
    async fn test_startup_reconciliation_prefix_without_netbox_id_not_found() {
        let netbox_url = "http://netbox.test".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        // Create a prefix without netbox_id that doesn't exist in NetBox
        let mut prefix = create_test_netbox_prefix(
            "test-prefix",
            "default",
            0,
            None,
        );
        prefix.status = None; // Clear status to test startup reconciliation
        apis.prefix_api.store("test-prefix".to_string(), prefix);

        // Run startup reconciliation
        let result = reconciler.startup_reconciliation().await;
        assert!(result.is_ok()); // Should succeed even if prefix not found

        // Verify the prefix still has no netbox_id
        let updated_prefix = apis.prefix_api.as_ref().get("test-prefix").await.unwrap();
        assert!(updated_prefix.status.as_ref().map(|s| s.netbox_id.is_none()).unwrap_or(true));
    }

    #[tokio::test]
    async fn test_startup_reconciliation_token_resolution_failure() {
        let netbox_url = "http://netbox.test".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        // Create a prefix with a tenant that doesn't exist (will cause token resolution failure)
        let mut prefix = create_test_netbox_prefix(
            "test-prefix",
            "default",
            0,
            None,
        );
        prefix.status = None; // Clear status
        prefix.spec.tenant.name = "nonexistent-tenant".to_string(); // This tenant doesn't exist
        apis.prefix_api.store("test-prefix".to_string(), prefix);

        // Run startup reconciliation - should handle token resolution failure gracefully
        let result = reconciler.startup_reconciliation().await;
        assert!(result.is_ok()); // Should succeed, just skip the prefix
    }

    #[tokio::test]
    async fn test_backoff_sequence_progression() {
        let netbox_url = "http://netbox.test".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        let resource_key = "test-resource";

        // Test Fibonacci backoff sequence: 60s, 60s, 120s, 180s, 300s, 480s, 600s (max)
        // Note: get_backoff_for_resource advances the sequence, so we need to call it after incrementing
        reconciler.increment_error(resource_key);
        let (backoff1, count1) = reconciler.get_backoff_for_resource(resource_key);
        assert_eq!(count1, 1);
        assert_eq!(backoff1, 60); // 1 minute = 60 seconds (first call advances to first value)

        reconciler.increment_error(resource_key);
        let (backoff2, count2) = reconciler.get_backoff_for_resource(resource_key);
        assert_eq!(count2, 2);
        assert_eq!(backoff2, 60); // Still 1 minute (second value, sequence advanced)

        reconciler.increment_error(resource_key);
        let (backoff3, count3) = reconciler.get_backoff_for_resource(resource_key);
        assert_eq!(count3, 3);
        assert_eq!(backoff3, 120); // 2 minutes = 120 seconds (sequence advanced)

        reconciler.increment_error(resource_key);
        let (backoff4, count4) = reconciler.get_backoff_for_resource(resource_key);
        assert_eq!(count4, 4);
        assert_eq!(backoff4, 180); // 3 minutes = 180 seconds (sequence advanced)

        reconciler.increment_error(resource_key);
        let (backoff5, count5) = reconciler.get_backoff_for_resource(resource_key);
        assert_eq!(count5, 5);
        assert_eq!(backoff5, 300); // 5 minutes = 300 seconds (sequence advanced)
    }

    #[tokio::test]
    async fn test_backoff_max_cap() {
        let netbox_url = "http://netbox.test".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        let resource_key = "test-resource";

        // Increment errors until we hit max (10 minutes = 600 seconds)
        // Note: get_backoff_for_resource advances the sequence each time it's called
        for i in 1..=7 {
            reconciler.increment_error(resource_key);
            let (backoff, count) = reconciler.get_backoff_for_resource(resource_key);
            assert_eq!(count, i);
            assert!(backoff <= 600, "Backoff should not exceed 600 seconds (10 minutes), got {} for count {}", backoff, count);
        }

        // After hitting max (7th error should be at 600s), should stay at max
        reconciler.increment_error(resource_key);
        let (backoff, _) = reconciler.get_backoff_for_resource(resource_key);
        assert_eq!(backoff, 600);
    }

    #[tokio::test]
    async fn test_backoff_reset_after_success() {
        let netbox_url = "http://netbox.test".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        let resource_key = "test-resource";

        // Increment errors (each increment increases error_count, get_backoff advances sequence)
        reconciler.increment_error(resource_key);
        reconciler.increment_error(resource_key);
        reconciler.increment_error(resource_key);
        let (backoff_before, count_before) = reconciler.get_backoff_for_resource(resource_key);
        assert_eq!(count_before, 3);
        // After 3 increments and 1 get_backoff call, we're at sequence position 1 (60s)
        // But the sequence advances each time get_backoff is called
        assert!(backoff_before >= 60 && backoff_before <= 120);

        // Reset (simulating successful reconciliation)
        reconciler.reset_error(resource_key);

        // Should be back to initial state (error_count = 0, sequence reset)
        let (backoff_after, count_after) = reconciler.get_backoff_for_resource(resource_key);
        assert_eq!(count_after, 0);
        assert_eq!(backoff_after, 60); // Back to 1 minute (first value after reset)
    }

    #[tokio::test]
    async fn test_multiple_resources_independent_backoff() {
        let netbox_url = "http://netbox.test".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        // Test that different resources have independent backoff states
        reconciler.increment_error("resource1");
        reconciler.increment_error("resource1");
        reconciler.increment_error("resource1");

        reconciler.increment_error("resource2");

        let (backoff1, count1) = reconciler.get_backoff_for_resource("resource1");
        let (backoff2, count2) = reconciler.get_backoff_for_resource("resource2");

        assert_eq!(count1, 3);
        // get_backoff advances sequence, so after 3 increments and 1 get_backoff, we're at position 1 (60s)
        assert!(backoff1 >= 60 && backoff1 <= 120);
        assert_eq!(count2, 1);
        assert_eq!(backoff2, 60); // 1 minute (first value)

        // Reset one resource, other should be unaffected
        reconciler.reset_error("resource1");
        let (backoff1_after, count1_after) = reconciler.get_backoff_for_resource("resource1");
        let (backoff2_after, count2_after) = reconciler.get_backoff_for_resource("resource2");

        assert_eq!(count1_after, 0);
        assert_eq!(backoff1_after, 60); // Reset to first value
        assert_eq!(count2_after, 1); // Unchanged
        // resource2's sequence was advanced by previous get_backoff call, so next value is 60s
        assert_eq!(backoff2_after, 60);
    }
}

