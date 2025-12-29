//! Unit tests for NetBoxMACAddress reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::kube_api_trait::KubeApiTrait;
    use crds::{NetBoxMACAddress, NetBoxDevice, NetBoxInterface, NetBoxTenant, ResourceState};
    use std::sync::Arc;
    use chrono::Utc;

    /// Helper to create test NetBoxMACAddress CRD
    fn create_test_netbox_mac_address(
        name: &str,
        namespace: &str,
        mac_address: &str,
        interface: &str, // Format: "device-name/interface-name"
        netbox_id: Option<u64>,
    ) -> NetBoxMACAddress {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        
        NetBoxMACAddress {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxMACAddressSpec {
                mac_address: mac_address.to_string(),
                interface: interface.to_string(),
                description: Some(format!("Test MAC address {}", name)),
                comments: None,
            },
            status: netbox_id.map(|id| crds::NetBoxMACAddressStatus {
                netbox_id: Some(id),
                netbox_url: Some(format!("http://test-netbox/api/dcim/mac-addresses/{}/", id)),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        }
    }

    /// Helper to create test NetBoxInterface CRD
    fn create_test_netbox_interface(
        name: &str,
        namespace: &str,
        device_name: &str,
        netbox_id: Option<u64>,
    ) -> NetBoxInterface {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        
        NetBoxInterface {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: crds::NetBoxInterfaceSpec {
                device: device_name.to_string(),
                name: name.to_string(),
                r#type: "1000base-t".to_string(),
                enabled: true,
                mac_address: None,
                mtu: None,
                description: None,
            },
            status: netbox_id.map(|id| crds::NetBoxInterfaceStatus {
                netbox_id: Some(id),
                netbox_url: Some(format!("http://test-netbox/api/dcim/interfaces/{}/", id)),
                state: ResourceState::Created,
                error: None,
                last_reconciled: None,
            }),
        }
    }

    /// Helper to create test NetBox Interface model
    fn create_test_interface(
        id: u64,
        device_id: u64,
        name: &str,
        base_url: &str,
    ) -> netbox_client::Interface {
        use netbox_client::NestedDevice;
        
        netbox_client::Interface {
            id,
            url: format!("{}/api/dcim/interfaces/{}/", base_url, id),
            display: name.to_string(),
            device: NestedDevice {
                id: device_id,
                url: format!("{}/api/dcim/devices/{}/", base_url, device_id),
                display: "test-device".to_string(),
                name: "test-device".to_string(),
            },
            vdcs: vec![],
            module: None,
            name: name.to_string(),
            label: None,
            r#type: "1000base-t".to_string(),
            enabled: true,
            parent: None,
            bridge: None,
            lag: None,
            mac_address: None,
            mtu: None,
            description: Some(String::new()),
            ip_addresses: vec![],
            tags: vec![],
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        }
    }

    /// Helper to create test NetBox MACAddress model
    fn create_test_mac_address(
        id: u64,
        mac_address: &str,
        interface_id: u64,
        base_url: &str,
    ) -> netbox_client::MACAddress {
        netbox_client::MACAddress {
            id,
            url: format!("{}/api/dcim/mac-addresses/{}/", base_url, id),
            display: mac_address.to_string(),
            mac_address: mac_address.to_string(),
            assigned_object_type: Some("dcim.interface".to_string()),
            assigned_object_id: Some(interface_id),
            assigned_object: None,
            description: Some(format!("Test MAC address {}", mac_address)),
            comments: None,
            tags: vec![],
            created: Utc::now().to_rfc3339(),
            last_updated: Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_reconcile_mac_address_create() {
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
        
        // Setup: Create device (required dependency)
        let device = create_test_netbox_device(
            "test-device",
            "default",
            "test-device-type",
            "test-device-role",
            "test-site",
            Some(1),
            Some("http://test-netbox/api/dcim/devices/1/".to_string()),
        );
        use netbox_client::{Device, NestedDeviceType, NestedManufacturer, NestedDeviceRole, NestedTenant, NestedSite, DeviceStatus};
        mock_token_resolver.mock_client().add_device(Device {
            id: 1,
            url: "http://test-netbox/api/dcim/devices/1/".to_string(),
            display: "test-device".to_string(),
            name: Some("test-device".to_string()),
            device_type: NestedDeviceType {
                id: 1,
                url: "http://test-netbox/api/dcim/device-types/1/".to_string(),
                display: "Test Model".to_string(),
                model: "Test Model".to_string(),
                manufacturer: NestedManufacturer {
                    id: 1,
                    url: "http://test-netbox/api/dcim/manufacturers/1/".to_string(),
                    display: "Test Manufacturer".to_string(),
                    name: "test-manufacturer".to_string(),
                    slug: "test-manufacturer".to_string(),
                },
            },
            device_role: Some(NestedDeviceRole {
                id: 1,
                url: "http://test-netbox/api/dcim/device-roles/1/".to_string(),
                display: "test-device-role".to_string(),
                name: "test-device-role".to_string(),
                slug: "test-device-role".to_string(),
            }),
            tenant: Some(NestedTenant {
                id: 1,
                url: "http://test-netbox/api/tenancy/tenants/1/".to_string(),
                display: "datacenter-tenant".to_string(),
                name: "datacenter-tenant".to_string(),
                slug: "datacenter-tenant".to_string(),
            }),
            platform: None,
            site: Some(NestedSite {
                id: 1,
                url: "http://test-netbox/api/dcim/sites/1/".to_string(),
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
        });
        
        // Setup: Create interface (required dependency)
        let interface = create_test_netbox_interface("test-device-eth0", "default", "test-device", Some(1));
        let netbox_interface = create_test_interface(1, 1, "eth0", "http://test-netbox");
        mock_token_resolver.mock_client().add_interface(netbox_interface);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant, device, and interface in APIs
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.device_api.store("test-device".to_string(), device);
        apis.interface_api.store("test-device-eth0".to_string(), interface);
        
        // Setup: Create MAC address CRD without status
        let mut mac_address = create_test_netbox_mac_address("test-mac", "default", "aa:bb:cc:dd:ee:ff", "test-device/eth0", None);
        mac_address.status = None;
        apis.mac_address_api.store("test-mac".to_string(), mac_address.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_mac_address(&mac_address).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.mac_address_api.as_ref().get("test-mac").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }

    #[tokio::test]
    async fn test_reconcile_mac_address_idempotent() {
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
        
        // Setup: Create device (required dependency)
        let device = create_test_netbox_device(
            "test-device",
            "default",
            "test-device-type",
            "test-device-role",
            "test-site",
            Some(1),
            Some("http://test-netbox/api/dcim/devices/1/".to_string()),
        );
        use netbox_client::{Device, NestedDeviceType, NestedManufacturer, NestedDeviceRole, NestedTenant, NestedSite, DeviceStatus};
        mock_token_resolver.mock_client().add_device(Device {
            id: 1,
            url: "http://test-netbox/api/dcim/devices/1/".to_string(),
            display: "test-device".to_string(),
            name: Some("test-device".to_string()),
            device_type: NestedDeviceType {
                id: 1,
                url: "http://test-netbox/api/dcim/device-types/1/".to_string(),
                display: "Test Model".to_string(),
                model: "Test Model".to_string(),
                manufacturer: NestedManufacturer {
                    id: 1,
                    url: "http://test-netbox/api/dcim/manufacturers/1/".to_string(),
                    display: "Test Manufacturer".to_string(),
                    name: "test-manufacturer".to_string(),
                    slug: "test-manufacturer".to_string(),
                },
            },
            device_role: Some(NestedDeviceRole {
                id: 1,
                url: "http://test-netbox/api/dcim/device-roles/1/".to_string(),
                display: "test-device-role".to_string(),
                name: "test-device-role".to_string(),
                slug: "test-device-role".to_string(),
            }),
            tenant: Some(NestedTenant {
                id: 1,
                url: "http://test-netbox/api/tenancy/tenants/1/".to_string(),
                display: "datacenter-tenant".to_string(),
                name: "datacenter-tenant".to_string(),
                slug: "datacenter-tenant".to_string(),
            }),
            platform: None,
            site: Some(NestedSite {
                id: 1,
                url: "http://test-netbox/api/dcim/sites/1/".to_string(),
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
        });
        
        // Setup: Create interface (required dependency)
        let interface = create_test_netbox_interface("test-device-eth0", "default", "test-device", Some(1));
        let netbox_interface = create_test_interface(1, 1, "eth0", "http://test-netbox");
        mock_token_resolver.mock_client().add_interface(netbox_interface);
        
        // Setup: Add MAC address to mock NetBox client
        let netbox_mac_address = create_test_mac_address(1, "aa:bb:cc:dd:ee:ff", 1, "http://test-netbox");
        mock_token_resolver.mock_client().add_mac_address("aa:bb:cc:dd:ee:ff".to_string(), netbox_mac_address);
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant, device, and interface in APIs
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.device_api.store("test-device".to_string(), device);
        apis.interface_api.store("test-device-eth0".to_string(), interface);
        
        // Setup: Create MAC address CRD with status (already created)
        let mac_address = create_test_netbox_mac_address("test-mac", "default", "aa:bb:cc:dd:ee:ff", "test-device/eth0", Some(1));
        apis.mac_address_api.store("test-mac".to_string(), mac_address.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_mac_address(&mac_address).await;
        
        // Assert: Should succeed (idempotent)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain unchanged
        let updated_crd = apis.mac_address_api.as_ref().get("test-mac").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should remain 1");
    }

    #[tokio::test]
    async fn test_reconcile_mac_address_device_not_found() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create MAC address CRD with device that doesn't exist
        let mut mac_address = create_test_netbox_mac_address("test-mac", "default", "aa:bb:cc:dd:ee:ff", "nonexistent-device/eth0", None);
        mac_address.status = None;
        apis.mac_address_api.store("test-mac".to_string(), mac_address.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_mac_address(&mac_address).await;
        
        // Assert: Should fail with InvalidConfig error (device not found)
        assert!(result.is_err(), "Reconciliation should fail when device not found");
        match result.unwrap_err() {
            crate::error::ControllerError::InvalidConfig(_) => {
                // Expected error type
            }
            e => panic!("Expected InvalidConfig error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_reconcile_mac_address_invalid_interface_format() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create MAC address CRD with invalid interface format (missing '/')
        let mut mac_address = create_test_netbox_mac_address("test-mac", "default", "aa:bb:cc:dd:ee:ff", "invalid-format", None);
        mac_address.status = None;
        apis.mac_address_api.store("test-mac".to_string(), mac_address.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_mac_address(&mac_address).await;
        
        // Assert: Should fail with InvalidConfig error (invalid interface format)
        assert!(result.is_err(), "Reconciliation should fail with invalid interface format");
        match result.unwrap_err() {
            crate::error::ControllerError::InvalidConfig(_) => {
                // Expected error type
            }
            e => panic!("Expected InvalidConfig error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_reconcile_mac_address_interface_not_found() {
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
        
        // Setup: Create device (required dependency)
        let device = create_test_netbox_device(
            "test-device",
            "default",
            "test-device-type",
            "test-device-role",
            "test-site",
            Some(1),
            Some("http://test-netbox/api/dcim/devices/1/".to_string()),
        );
        
        let (reconciler, apis, _mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant and device in APIs (but no interface)
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.device_api.store("test-device".to_string(), device);
        
        // Setup: Create MAC address CRD with interface that doesn't exist
        let mut mac_address = create_test_netbox_mac_address("test-mac", "default", "aa:bb:cc:dd:ee:ff", "test-device/nonexistent-interface", None);
        mac_address.status = None;
        apis.mac_address_api.store("test-mac".to_string(), mac_address.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_mac_address(&mac_address).await;
        
        // Assert: Should fail with InvalidConfig error (interface not found)
        assert!(result.is_err(), "Reconciliation should fail when interface not found");
        match result.unwrap_err() {
            crate::error::ControllerError::InvalidConfig(_) => {
                // Expected error type
            }
            e => panic!("Expected InvalidConfig error, got: {:?}", e),
        }
    }
}

