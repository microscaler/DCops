//! Unit tests for NetBoxDevice reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::test_utils::*;
    use crate::kube_api_trait::KubeApiTrait;
    use std::sync::Arc;
    use crds::{NetBoxDevice, NetBoxTenant, NetBoxSite, NetBoxDeviceType, NetBoxDeviceRole, ResourceState};
    
    /// Helper to set up test data for device reconciliation
    fn setup_device_test_data() -> (NetBoxDevice, NetBoxTenant, NetBoxSite, NetBoxDeviceType, NetBoxDeviceRole) {
        // Create test tenant with status (required dependency)
        let tenant = create_test_netbox_tenant(
            "datacenter-tenant",
            "default",
            Some(1),
            Some("http://test-netbox/api/tenancy/tenants/1/".to_string()),
        );
        
        // Create test site with status (required dependency)
        let site = create_test_netbox_site(
            "test-site",
            "default",
            Some(1),
            Some("http://test-netbox/api/dcim/sites/1/".to_string()),
        );
        
        // Create test device type (would need helper function)
        let device_type = NetBoxDeviceType {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("test-device-type".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxDeviceTypeSpec {
                manufacturer: crds::NetBoxResourceReference {
                    api_group: "dcops.microscaler.io".to_string(),
                    kind: "NetBoxManufacturer".to_string(),
                    name: "test-manufacturer".to_string(),
                    namespace: Some("default".to_string()),
                },
                model: "Test Model".to_string(),
                slug: None,
                part_number: None,
                u_height: 1.0, // f64, not Option
                is_full_depth: false, // bool, not Option
                description: None,
                comments: None,
            },
            status: Some(crds::NetBoxDeviceTypeStatus {
                netbox_id: Some(1),
                netbox_url: Some("http://test-netbox/api/dcim/device-types/1/".to_string()),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        };
        
        // Create test device role (would need helper function)
        let device_role = NetBoxDeviceRole {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("test-device-role".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxDeviceRoleSpec {
                name: "test-device-role".to_string(),
                slug: None,
                color: None,
                vm_role: false,
                description: None,
                comments: None,
            },
            status: Some(crds::NetBoxDeviceRoleStatus {
                netbox_id: Some(1),
                netbox_url: Some("http://test-netbox/api/dcim/device-roles/1/".to_string()),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        };
        
        // Create test device CRD
        let mut device = create_test_netbox_device(
            "test-device",
            "default",
            "test-device-type",
            "test-device-role",
            "test-site",
            None,
            None,
        );
        device.status = None; // Clear status to test create path
        
        (device, tenant, site, device_type, device_role)
    }
    
    #[tokio::test]
    async fn test_reconcile_device_create() {
        use crate::test_utils::mock_token_resolver::TestReconcilerApis;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Get mock client before creating reconciler (needed to set up NetBox data)
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Add device type to mock NetBox client (needed for create_device to work)
        use netbox_client::{DeviceType, NestedManufacturer};
        use chrono::Utc;
        let manufacturer_id = 1;
        let device_type_netbox = DeviceType {
            id: 1,
            url: format!("{}/api/dcim/device-types/1/", netbox_url),
            display: "Test Model".to_string(),
            manufacturer: NestedManufacturer {
                id: manufacturer_id,
                url: format!("{}/api/dcim/manufacturers/{}/", netbox_url, manufacturer_id),
                display: "Test Manufacturer".to_string(),
                name: "Test Manufacturer".to_string(),
                slug: "test-manufacturer".to_string(),
            },
            model: "Test Model".to_string(),
            slug: "test-model".to_string(),
            part_number: None,
            u_height: 1.0,
            is_full_depth: false,
            description: None,
            comments: None,
            device_count: 0,
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        mock_client.add_device_type(device_type_netbox);
        
        // Setup: Create reconciler
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        let TestReconcilerApis {
            tenant_api,
            site_api,
            device_type_api,
            device_role_api,
            device_api,
            ..
        } = apis;
        
        // Setup: Create test data
        let (mut device, tenant, site, device_type, device_role) = setup_device_test_data();
        device.status = None; // Clear status to test create path
        
        // Store dependencies in mock APIs
        tenant_api.store("datacenter-tenant".to_string(), tenant);
        site_api.store("test-site".to_string(), site);
        device_type_api.store("test-device-type".to_string(), device_type);
        device_role_api.store("test-device-role".to_string(), device_role);
        device_api.store("test-device".to_string(), device.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_device(&device).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = device_api.get("test-device").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_device_idempotent() {
        // TODO: Test idempotent reconciliation
        // 1. Create device with status
        // 2. Reconcile without changes
        // 3. Verify no update was called (resource already up-to-date)
    }
    
    #[tokio::test]
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_device_drift_detection() {
        // TODO: Test drift detection
        // 1. Create device with status
        // 2. Delete device in NetBox (simulate drift)
        // 3. Reconcile
        // 4. Verify status is cleared and device is recreated
    }
    
    #[tokio::test]
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_device_dependency_resolution() {
        // TODO: Test dependency resolution
        // 1. Create device with references to DeviceType, DeviceRole, Site
        // 2. One dependency doesn't exist yet
        // 3. Reconcile should fail with dependency error
    }
    
    #[tokio::test]
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_device_with_primary_ip() {
        // TODO: Test device creation with primary IP assignment
        // 1. Create device with primary_ip4 reference
        // 2. Reconcile
        // 3. Verify device is created and primary IP is assigned
    }
}

