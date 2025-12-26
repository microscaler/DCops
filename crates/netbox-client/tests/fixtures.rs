//! Test fixtures and utilities for integration tests
//!
//! Provides reusable test data factories and utilities for creating
//! test resources in NetBox for integration testing.

/// Create test prefix parameters for integration tests
/// 
/// Returns a tuple matching the signature of `NetBoxClientTrait::create_prefix`
pub fn create_test_prefix_params(
    prefix: &str,
    site_id: Option<u64>,
    tenant_id: Option<u64>,
) -> (&str, Option<u64>, Option<u64>, Option<u32>, Option<u64>, Option<&str>, Option<&str>, Option<Vec<serde_json::Value>>) {
    (
        prefix,
        site_id,
        tenant_id,
        None, // vlan_id
        None, // role_id
        Some("active"), // status
        Some("Test prefix created by integration test"), // description
        None, // tags
    )
}

/// Create test site parameters for integration tests
/// 
/// Returns a tuple matching the signature of `NetBoxClientTrait::create_site`
pub fn create_test_site_params(
    name: &str,
    region_id: Option<u64>,
    tenant_id: Option<u64>,
) -> (&str, Option<&str>, &str, Option<u64>, Option<u64>, Option<u64>, Option<&str>, Option<&str>, Option<&str>, Option<&str>) {
    (
        name,
        None, // slug
        "active", // status
        region_id,
        None, // site_group_id
        tenant_id,
        None, // facility
        None, // time_zone
        Some("Test site created by integration test"), // description
        None, // comments
    )
}

/// Create test tenant parameters for integration tests
/// 
/// Returns parameters for `NetBoxClientTrait::create_tenant`
/// Note: Returns owned strings to avoid lifetime issues
pub fn create_test_tenant_params(
    name: &str,
    tenant_group_id: Option<u64>,
) -> (String, String, Option<u64>, Option<String>, Option<String>) {
    (
        name.to_string(),
        name.to_lowercase().replace(' ', "-"), // slug
        tenant_group_id,
        Some("Test tenant created by integration test".to_string()), // description
        None, // comments
    )
}

/// Helper to generate unique test resource names
pub fn unique_test_name(base: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}-{}", base, timestamp)
}

/// Helper to clean up test resources
/// 
/// This is a placeholder for actual cleanup logic.
/// In a real implementation, this would:
/// 1. List resources with a test tag or prefix
/// 2. Delete them in the correct order (respecting dependencies)
/// 3. Handle errors gracefully
pub async fn cleanup_test_resources(
    _client: &netbox_client::NetBoxClient,
    _test_prefix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Implement cleanup logic
    // This would:
    // 1. Query resources with test prefix or tag
    // 2. Delete in dependency order
    // 3. Handle errors
    Ok(())
}

