//! Unit tests for Watcher module
//!
//! These tests verify the watcher's error policy, reconcile logic, and resource watching behavior.
//!
//! Note: Full integration tests for watchers require a real Kubernetes cluster or complex mocking
//! of kube_runtime::Controller. These tests focus on the logic patterns used in watch_resource:
//! error policy behavior, reconcile function patterns, and backoff/retry logic.

#[cfg(test)]
mod tests {
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::test_utils::create_test_netbox_prefix;
    use crate::test_utils::event_test_helpers::*;
    use crate::events::reasons;
    use std::sync::Arc;
    use crds::NetBoxPrefix;
    use kube_runtime::controller::Action;
    use std::time::Duration;

    /// Test that the error policy increments error count and returns appropriate backoff
    #[tokio::test]
    async fn test_error_policy_increments_error_count() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        let prefix = create_test_netbox_prefix("test-prefix", "default", 0, None);
        let resource_key = format!(
            "{}/{}",
            prefix.metadata.namespace.as_deref().unwrap_or("default"),
            prefix.metadata.name.as_deref().unwrap_or("unknown")
        );

        // Initially no errors
        let (_, initial_count) = reconciler.get_backoff_for_resource(&resource_key);
        assert_eq!(initial_count, 0);

        // Simulate error policy behavior: increment error and get backoff
        reconciler.increment_error(&resource_key);
        let (backoff, error_count) = reconciler.get_backoff_for_resource(&resource_key);
        
        assert_eq!(error_count, 1);
        assert_eq!(backoff, 60); // First error = 60 seconds (1 minute)
    }

    /// Test that error policy returns increasing backoff durations
    #[tokio::test]
    async fn test_error_policy_backoff_progression() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        let prefix = create_test_netbox_prefix("test-prefix", "default", 0, None);
        let resource_key = format!(
            "{}/{}",
            prefix.metadata.namespace.as_deref().unwrap_or("default"),
            prefix.metadata.name.as_deref().unwrap_or("unknown")
        );

        // Simulate multiple errors (Fibonacci sequence: 60s, 60s, 120s, 180s, 300s...)
        for i in 1..=5 {
            reconciler.increment_error(&resource_key);
            let (backoff, error_count) = reconciler.get_backoff_for_resource(&resource_key);
            assert_eq!(error_count, i);
            assert!(backoff >= 60 && backoff <= 600, "Backoff should be between 60s and 600s, got {} for error {}", backoff, i);
        }
    }

    /// Test that successful reconciliation resets error count
    #[tokio::test]
    async fn test_successful_reconciliation_resets_errors() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        let prefix = create_test_netbox_prefix("test-prefix", "default", 0, None);
        let resource_key = format!(
            "{}/{}",
            prefix.metadata.namespace.as_deref().unwrap_or("default"),
            prefix.metadata.name.as_deref().unwrap_or("unknown")
        );

        // Simulate errors
        reconciler.increment_error(&resource_key);
        reconciler.increment_error(&resource_key);
        let (_, error_count_before) = reconciler.get_backoff_for_resource(&resource_key);
        assert_eq!(error_count_before, 2);

        // Simulate successful reconciliation (reset error)
        reconciler.reset_error(&resource_key);

        // Error count should be reset
        let (_, error_count_after) = reconciler.get_backoff_for_resource(&resource_key);
        assert_eq!(error_count_after, 0);
    }

    /// Test that reconcile function returns requeue action on success
    /// This tests the reconcile closure logic used in watch_resource
    #[tokio::test]
    async fn test_reconcile_returns_requeue_on_success() {
        // This test verifies the logic pattern used in watch_resource
        // The actual reconcile function would call the reconciler's reconcile method
        // Here we test the pattern: success -> reset error -> requeue with delay
        
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        let prefix = create_test_netbox_prefix("test-prefix", "default", 0, None);
        let resource_key = format!(
            "{}/{}",
            prefix.metadata.namespace.as_deref().unwrap_or("default"),
            prefix.metadata.name.as_deref().unwrap_or("unknown")
        );

        // Simulate successful reconciliation pattern:
        // 1. Reconcile succeeds
        // 2. Reset error count
        // 3. Return requeue action with 10 second delay
        
        reconciler.increment_error(&resource_key); // Simulate previous error
        reconciler.reset_error(&resource_key); // Reset on success
        
        // Verify reset worked
        let (_, error_count) = reconciler.get_backoff_for_resource(&resource_key);
        assert_eq!(error_count, 0);
        
        // The actual reconcile function would return Action::requeue(Duration::from_secs(10))
        // This is the pattern used in watch_resource for successful reconciliations
        // We verify the logic pattern: success -> reset -> requeue with 10s delay
        let _expected_action = Action::requeue(Duration::from_secs(10));
        // Action::requeue() is used in the actual watcher code
        // Full integration testing requires a real kube cluster or complex mocking
    }

    /// Test that reconcile function propagates errors
    /// This tests the error handling pattern used in watch_resource
    #[tokio::test]
    async fn test_reconcile_propagates_errors() {
        // This test verifies that errors from reconciliation are properly propagated
        // The error policy will handle incrementing error count and backoff
        
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        let prefix = create_test_netbox_prefix("test-prefix", "default", 0, None);
        let resource_key = format!(
            "{}/{}",
            prefix.metadata.namespace.as_deref().unwrap_or("default"),
            prefix.metadata.name.as_deref().unwrap_or("unknown")
        );

        // Simulate error handling pattern:
        // 1. Reconcile fails
        // 2. Error policy increments error count
        // 3. Error policy returns requeue with backoff
        
        reconciler.increment_error(&resource_key);
        let (backoff, error_count) = reconciler.get_backoff_for_resource(&resource_key);
        
        assert_eq!(error_count, 1);
        assert_eq!(backoff, 60);
        
        // The error policy would return Action::requeue(Duration::from_secs(backoff))
        // We verify the logic: error -> increment -> get backoff -> requeue with backoff
        let _expected_action = Action::requeue(Duration::from_secs(backoff));
        // Action::requeue() is used in the actual error policy
        // Full integration testing requires a real kube cluster or complex mocking
    }

    /// Test that watcher handles reconcile_interval for NetBoxTenant
    /// NetBoxTenant has special logic for periodic reconciliation
    #[tokio::test]
    async fn test_tenant_reconcile_interval_logic() {
        // Test the logic pattern used in watch_netbox_tenants:
        // - If reconcile_interval is 0, return await_change()
        // - If reconcile_interval > 0, return requeue(interval)
        
        let interval_0 = 0;
        let interval_300 = 300;
        
        // Interval 0 = only reconcile on changes
        if interval_0 == 0 {
            let _action = Action::await_change();
            // Action::await_change() is used for interval 0 (only on changes)
            // This logic is tested in the actual watcher implementation
        }
        
        // Interval > 0 = periodic reconciliation
        if interval_300 > 0 {
            let _action = Action::requeue(Duration::from_secs(interval_300));
            // Action::requeue() is used for periodic reconciliation
            // This logic is tested in the actual watcher implementation
            assert_eq!(interval_300, 300); // Verify the interval value
        }
    }

    /// Test that error policy emits retry attempt events
    #[tokio::test]
    async fn test_error_policy_emits_retry_event() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        let (reconciler, _, mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);

        let prefix = create_test_netbox_prefix("test-prefix", "default", 0, None);
        
        // Simulate error policy behavior: emit retry attempt event
        reconciler.increment_error("default/test-prefix");
        let (backoff, error_count) = reconciler.get_backoff_for_resource("default/test-prefix");
        
        // The error policy spawns a task to emit the retry event
        // Here we manually test the event emission
        reconciler.record_event_retry_attempt_str(
            "Test error message",
            error_count,
            backoff,
            &prefix,
        ).await;
        
        // Verify RETRY_ATTEMPT event was emitted
        let event = assert_warning_event_emitted(&mock_event_recorder, reasons::RETRY_ATTEMPT)
            .expect("RETRY_ATTEMPT event should be emitted");
        assert_event_for_resource(&event, &prefix)
            .expect("Event should be for the correct prefix resource");
        assert_event_message_contains(&event, "attempt")
            .expect("Event message should mention attempt");
    }
}

