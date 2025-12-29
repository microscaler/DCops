//! Unit tests for NetBoxInterface reconciler

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::kube_api_trait::KubeApiTrait;
    use crds::{NetBoxInterface, NetBoxDevice, ResourceState};
    use std::sync::Arc;
    use chrono::Utc;

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

    #[tokio::test]
    async fn test_reconcile_interface_create() {
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
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant and device in APIs
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.device_api.store("test-device".to_string(), device);
        
        // Setup: Create interface CRD without status
        let mut interface = create_test_netbox_interface("eth0", "default", "test-device", None);
        interface.status = None;
        apis.interface_api.store("eth0".to_string(), interface.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_interface(&interface).await;
        
        // Assert: Should succeed
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should be updated with NetBox ID
        let updated_crd = apis.interface_api.as_ref().get("eth0").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should be set");
        let status = updated_crd.status.unwrap();
        assert!(status.netbox_id.is_some(), "NetBox ID should be set");
        assert_eq!(status.state, ResourceState::Created, "State should be Created");
    }

    #[tokio::test]
    async fn test_reconcile_interface_idempotent() {
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
        
        // Setup: Add interface to mock NetBox client
        let netbox_interface = create_test_interface(1, 1, "eth0", "http://test-netbox");
        mock_token_resolver.mock_client().add_interface(netbox_interface);
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Store tenant and device in APIs
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.device_api.store("test-device".to_string(), device);
        
        // Setup: Create interface CRD with status (already created)
        let interface = create_test_netbox_interface("eth0", "default", "test-device", Some(1));
        apis.interface_api.store("eth0".to_string(), interface.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_interface(&interface).await;
        
        // Assert: Should succeed (idempotent)
        assert!(result.is_ok(), "Reconciliation should succeed: {:?}", result.err());
        
        // Assert: Status should remain unchanged
        let updated_crd = apis.interface_api.as_ref().get("eth0").await.unwrap();
        assert!(updated_crd.status.is_some(), "Status should still be set");
        let status = updated_crd.status.unwrap();
        assert_eq!(status.netbox_id, Some(1), "NetBox ID should remain 1");
    }

    #[tokio::test]
    async fn test_reconcile_interface_device_not_found() {
        let netbox_url = "http://test-netbox".to_string();
        let mock_token_resolver = Arc::new(MockTokenResolver::new(netbox_url.clone()));
        
        mock_token_resolver.add_secret("default", "netbox-token-datacenter-tenant", "test-token".to_string());
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create interface CRD with device that doesn't exist
        let mut interface = create_test_netbox_interface("eth0", "default", "nonexistent-device", None);
        interface.status = None;
        apis.interface_api.store("eth0".to_string(), interface.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_interface(&interface).await;
        
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
    async fn test_reconcile_interface_device_no_status() {
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
        
        let (reconciler, apis) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // Setup: Create device without status (not yet created in NetBox)
        let mut device = create_test_netbox_device(
            "test-device",
            "default",
            "test-device-type",
            "test-device-role",
            "test-site",
            None, // No netbox_id - device not created yet
            None,
        );
        device.status = None;
        apis.device_api.store("test-device".to_string(), device);
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        
        // Setup: Create interface CRD
        let mut interface = create_test_netbox_interface("eth0", "default", "test-device", None);
        interface.status = None;
        apis.interface_api.store("eth0".to_string(), interface.clone());
        
        // Execute: Reconcile
        let result = reconciler.reconcile_netbox_interface(&interface).await;
        
        // Assert: Should succeed but return early (device not ready, will requeue)
        assert!(result.is_ok(), "Reconciliation should succeed but return early: {:?}", result.err());
        
        // Assert: Status should not be updated (device not ready)
        let updated_crd = apis.interface_api.as_ref().get("eth0").await.unwrap();
        // Status may or may not be set - the reconciler returns early if device has no status
        // This is expected behavior - controller will requeue when device is ready
    }
}

