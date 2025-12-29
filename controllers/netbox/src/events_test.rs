//! Tests for Kubernetes Events support

#[cfg(test)]
mod tests {
    use crate::events::reasons;
    
    #[test]
    fn test_event_reasons_constants() {
        // Verify all event reason constants are defined
        assert_eq!(reasons::CREATED, "Created");
        assert_eq!(reasons::UPDATED, "Updated");
        assert_eq!(reasons::DELETED, "Deleted");
        assert_eq!(reasons::RECONCILIATION_FAILED, "ReconciliationFailed");
        assert_eq!(reasons::DEPENDENCY_NOT_FOUND, "DependencyNotFound");
        assert_eq!(reasons::DRIFT_DETECTED, "DriftDetected");
        assert_eq!(reasons::TOKEN_RESOLUTION_FAILED, "TokenResolutionFailed");
        assert_eq!(reasons::RETRY_ATTEMPT, "RetryAttempt");
        assert_eq!(reasons::STARTUP_MAPPED, "StartupMapped");
    }
    
    #[test]
    fn test_event_reasons_are_non_empty() {
        // Verify all event reasons are non-empty strings
        assert!(!reasons::CREATED.is_empty());
        assert!(!reasons::UPDATED.is_empty());
        assert!(!reasons::DELETED.is_empty());
        assert!(!reasons::RECONCILIATION_FAILED.is_empty());
        assert!(!reasons::DEPENDENCY_NOT_FOUND.is_empty());
        assert!(!reasons::DRIFT_DETECTED.is_empty());
        assert!(!reasons::TOKEN_RESOLUTION_FAILED.is_empty());
        assert!(!reasons::RETRY_ATTEMPT.is_empty());
        assert!(!reasons::STARTUP_MAPPED.is_empty());
    }
}

