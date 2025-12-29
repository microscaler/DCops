//! Unit tests for NetBoxDevice reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::kube_api_trait::mock::MockKubeApi;
    use netbox_client::MockNetBoxClient;
    use crds::{NetBoxDevice, NetBoxTenant, NetBoxSite, NetBoxDeviceType, NetBoxDeviceRole, ResourceState};
    use kube::Client;
    
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
                vm_role: None,
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
    #[ignore] // Ignored until kube::Client mocking is implemented
    async fn test_reconcile_device_create() {
        // Setup: Create mock NetBoxClient
        let _mock_netbox = MockNetBoxClient::new("http://test-netbox");
        
        // Setup: Create test data
        let (mut device, tenant, site, device_type, device_role) = setup_device_test_data();
        device.status = None; // Clear status to test create path
        
        // Setup: Create mock Kubernetes APIs
        let tenant_api = MockKubeApi::<NetBoxTenant>::new();
        // tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        let site_api = MockKubeApi::<NetBoxSite>::new();
        // site_api.store("test-site".to_string(), site);
        
        let device_type_api = MockKubeApi::<NetBoxDeviceType>::new();
        // device_type_api.store("test-device-type".to_string(), device_type);
        
        let device_role_api = MockKubeApi::<NetBoxDeviceRole>::new();
        // device_role_api.store("test-device-role".to_string(), device_role);
        
        let device_api = MockKubeApi::<NetBoxDevice>::new();
        // device_api.store("test-device".to_string(), device.clone());
        
        // Setup: Create reconciler
        let _kube_client = match Client::try_default().await {
            Ok(client) => client,
            Err(_) => return, // Skip test if no kube client available
        };
        
        // TODO: Uncomment once kube::Client mocking is implemented
        // let reconciler = create_test_reconciler(kube_client, "http://test-netbox".to_string());
        // 
        // // Execute: Reconcile
        // let result = reconciler.reconcile_netbox_device(&device).await;
        // 
        // // Assert: Should succeed
        // assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        // 
        // // Assert: Status should be updated with NetBox ID
        // let updated_crd = device_api.get("test-device").await.unwrap();
        // assert!(updated_crd.status.is_some(), "Status should be set");
        // let status = updated_crd.status.unwrap();
        // assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        // assert_eq!(status.state, ResourceState::Created, "State should be Created");
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

