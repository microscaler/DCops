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
                tags: None,
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
                tags: None,
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
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
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
    async fn test_reconcile_device_idempotent() {
        use crate::test_utils::mock_token_resolver::TestReconcilerApis;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Get mock client before creating reconciler
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Add device type to mock NetBox client
        use netbox_client::{DeviceType, NestedManufacturer, Device, NestedDeviceType, NestedDeviceRole, NestedSite, DeviceStatus};
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
        
        // Setup: Add existing device to mock NetBox (simulating already created)
        use netbox_client::NestedTenant;
        let existing_device = Device {
            id: 1,
            url: format!("{}/api/dcim/devices/1/", netbox_url),
            display: "test-device".to_string(),
            name: Some("test-device".to_string()),
            device_type: NestedDeviceType {
                id: 1,
                url: format!("{}/api/dcim/device-types/1/", netbox_url),
                display: "Test Model".to_string(),
                model: "Test Model".to_string(),
                manufacturer: NestedManufacturer {
                    id: manufacturer_id,
                    url: format!("{}/api/dcim/manufacturers/{}/", netbox_url, manufacturer_id),
                    display: "Test Manufacturer".to_string(),
                    name: "Test Manufacturer".to_string(),
                    slug: "test-manufacturer".to_string(),
                },
            },
            device_role: Some(NestedDeviceRole {
                id: 1,
                url: format!("{}/api/dcim/device-roles/1/", netbox_url),
                display: "test-device-role".to_string(),
                name: "test-device-role".to_string(),
                slug: "test-device-role".to_string(),
            }),
            tenant: Some(NestedTenant {
                id: 1,
                url: format!("{}/api/tenancy/tenants/1/", netbox_url),
                display: "datacenter-tenant".to_string(),
                name: "datacenter-tenant".to_string(),
                slug: "datacenter-tenant".to_string(),
            }),
            platform: None,
            site: Some(NestedSite {
                id: 1,
                url: format!("{}/api/dcim/sites/1/", netbox_url),
                display: "test-site".to_string(),
                name: "test-site".to_string(),
                slug: "test-site".to_string(),
            }),
            location: None,
            status: DeviceStatus::Active,
            serial: None,
            asset_tag: None,
            primary_ip4: None,
            primary_ip6: None,
            description: None,
            comments: None,
            tags: vec![],
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        mock_client.add_device(existing_device);
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        let TestReconcilerApis {
            tenant_api,
            site_api,
            device_type_api,
            device_role_api,
            device_api,
            ..
        } = apis;
        
        // Setup: Create test data with status (device already exists)
        let (device, tenant, site, device_type, device_role) = setup_device_test_data();
        let mut device_with_status = device.clone();
        device_with_status.status = Some(crds::NetBoxDeviceStatus {
            netbox_id: Some(1),
            netbox_url: Some(format!("{}/api/dcim/devices/1/", netbox_url)),
            state: ResourceState::Created,
            error: None,
            last_reconciled: None,
        });
        
        // Store dependencies in mock APIs
        tenant_api.store("datacenter-tenant".to_string(), tenant);
        site_api.store("test-site".to_string(), site);
        device_type_api.store("test-device-type".to_string(), device_type);
        device_role_api.store("test-device-role".to_string(), device_role);
        device_api.store("test-device".to_string(), device_with_status.clone());
        
        // Execute: Reconcile (should be idempotent - no changes needed)
        let result = reconciler.reconcile_netbox_device(&device_with_status).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should still be correct (idempotent - no update needed)
        let updated_crd = device_api.get("test-device").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should still be 1");
        assert_eq!(status.state, ResourceState::Created, "State should still be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_device_drift_detection() {
        use crate::test_utils::mock_token_resolver::TestReconcilerApis;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Get mock client before creating reconciler
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Add device type to mock NetBox client
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
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        let TestReconcilerApis {
            tenant_api,
            site_api,
            device_type_api,
            device_role_api,
            device_api,
            ..
        } = apis;
        
        // Setup: Create test data with status (device already exists)
        let (device, tenant, site, device_type, device_role) = setup_device_test_data();
        let mut device_with_status = device.clone();
        device_with_status.status = Some(crds::NetBoxDeviceStatus {
            netbox_id: Some(1),
            netbox_url: Some(format!("{}/api/dcim/devices/1/", netbox_url)),
            state: ResourceState::Created,
            error: None,
            last_reconciled: None,
        });
        
        // Store dependencies in mock APIs
        tenant_api.store("datacenter-tenant".to_string(), tenant);
        site_api.store("test-site".to_string(), site);
        device_type_api.store("test-device-type".to_string(), device_type);
        device_role_api.store("test-device-role".to_string(), device_role);
        device_api.store("test-device".to_string(), device_with_status.clone());
        
        // IMPORTANT: Do NOT add the device to mock NetBox client (simulating drift - device was deleted)
        // This will cause validate_status_and_drift to detect the device is missing and trigger recreation
        
        // Execute: Reconcile (should detect drift and recreate device)
        let result = reconciler.reconcile_netbox_device(&device_with_status).await;
        
        // Assert: Should succeed (device will be recreated)
        assert!(result.is_ok(), "Reconciliation should succeed after drift detection: {:?}", result.err());
        
        // Assert: Status should be updated with new NetBox ID (device was recreated)
        let updated_crd = device_api.get("test-device").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set after recreation");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }
    
    #[tokio::test]
    async fn test_reconcile_device_dependency_resolution() {
        use crate::test_utils::mock_token_resolver::TestReconcilerApis;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        let TestReconcilerApis {
            tenant_api,
            site_api,
            device_type_api: _, // Intentionally not used - testing missing dependency
            device_role_api,
            device_api,
            ..
        } = apis;
        
        // Setup: Create test data
        let (mut device, tenant, site, _device_type, device_role) = setup_device_test_data();
        device.status = None; // Clear status to test create path
        
        // Store only some dependencies (missing device_type to simulate dependency not found)
        tenant_api.store("datacenter-tenant".to_string(), tenant);
        site_api.store("test-site".to_string(), site);
        // Intentionally NOT storing device_type to simulate missing dependency
        device_role_api.store("test-device-role".to_string(), device_role);
        device_api.store("test-device".to_string(), device.clone());
        
        // Execute: Reconcile (should fail because device_type dependency is missing)
        let result = reconciler.reconcile_netbox_device(&device).await;
        
        // Assert: Should fail with dependency error
        assert!(result.is_err(), "Reconciliation should fail when device_type dependency is missing");
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("DeviceType") || error_msg.contains("device_type") || error_msg.contains("not found") || error_msg.contains("dependency"), 
                "Error should mention missing DeviceType dependency: {}", error_msg);
    }
    
    #[tokio::test]
    async fn test_reconcile_device_with_primary_ip() {
        use crate::test_utils::mock_token_resolver::TestReconcilerApis;
        use std::str::FromStr;
        use ipnet::IpNet;
        
        // Setup: Create mock TokenResolver
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        // Setup: Get mock client before creating reconciler
        let mock_client = mock_token_resolver.mock_client();
        
        // Setup: Add device type to mock NetBox client
        use netbox_client::{DeviceType, NestedManufacturer, IPAddress, IPAddressStatus};
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
        
        // Setup: Create IP address in NetBox (for primary IP assignment)
        let primary_ip_net = IpNet::from_str("192.168.1.10/32").unwrap();
        let primary_ip = IPAddress {
            id: 100,
            url: format!("{}/api/ipam/ip-addresses/100/", netbox_url),
            display: "192.168.1.10/32".to_string(),
            family: 4, // IPv4
            address: primary_ip_net.clone(),
            vrf: None,
            tenant: None,
            status: IPAddressStatus::Active,
            role: None,
            assigned_object_type: None,
            assigned_object_id: None,
            assigned_object: None,
            nat_inside: None,
            nat_outside: vec![],
            dns_name: String::new(),
            description: String::new(),
            comments: String::new(),
            tags: vec![],
            custom_fields: serde_json::json!({}),
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        };
        mock_client.add_ip_address(primary_ip);
        
        // Setup: Create reconciler
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        let TestReconcilerApis {
            tenant_api,
            site_api,
            device_type_api,
            device_role_api,
            device_api,
            ip_claim_api,
            ..
        } = apis;
        
        // Setup: Create IPClaim with status (for primary IP reference)
        let ip_claim = create_test_ip_claim(
            "test-ip-claim",
            "default",
            "test-ip-pool",
            None,
            "test-device",
            None,
            Some("192.168.1.10/32"),
        );
        let mut ip_claim_with_status = ip_claim.clone();
        ip_claim_with_status.status = Some(crds::IPClaimStatus {
            ip: Some("192.168.1.10/32".to_string()),
            state: crds::AllocationState::Allocated,
            netbox_ip_ref: Some(format!("{}/api/ipam/ip-addresses/100/", netbox_url)),
            last_reconciled: None,
            error: None,
        });
        ip_claim_api.store("test-ip-claim".to_string(), ip_claim_with_status);
        
        // Setup: Create test device with primary IP reference
        let (mut device, tenant, site, device_type, device_role) = setup_device_test_data();
        device.status = None; // Clear status to test create path
        device.spec.primary_ip4 = Some(crds::PrimaryIPReference {
            ip_claim_ref: Some(crds::NetBoxResourceReference {
                api_group: "dcops.microscaler.io".to_string(),
                kind: "IPClaim".to_string(),
                name: "test-ip-claim".to_string(),
                namespace: Some("default".to_string()),
            }),
            ip_address: None,
        });
        
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
        
        // Assert: Device should have primary IP assigned (verify via mock client query)
        // The device should be created with primary_ip4_id = 100
        // We verify this by checking that reconciliation succeeded, which means the device
        // was created with the primary IP reference resolved from the IPClaim
        let device_id = status.netbox_id.unwrap();
        use netbox_client::NetBoxClientTrait;
        let netbox_client = reconciler.token_resolver
            .create_client_for_tenant("default", &updated_crd.spec.tenant)
            .await
            .unwrap();
        let created_device = netbox_client.get_device(netbox_client::DeviceId(device_id)).await.unwrap();
        assert!(created_device.primary_ip4.is_some(), "Device should have primary_ip4 assigned");
        assert_eq!(created_device.primary_ip4.as_ref().unwrap().id, 100, "Device should have primary_ip4 with ID 100");
    }
}

