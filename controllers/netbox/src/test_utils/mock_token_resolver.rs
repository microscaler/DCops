//! Mock TokenResolver for unit testing
//!
//! This module provides a mock implementation of TokenResolver that doesn't require
//! a real kube::Client. It stores secrets in memory and returns them when requested.

#[cfg(test)]
use crate::token_resolver::{TokenResolver, TokenResolutionError};
#[cfg(test)]
use crds::NetBoxResourceReference;
#[cfg(test)]
use netbox_client::NetBoxClient;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::{Arc, Mutex};

/// Mock TokenResolver for testing
///
/// This mock stores secrets in memory and returns them when requested.
/// It doesn't require a real kube::Client, making it suitable for unit tests.
#[cfg(test)]
pub struct MockTokenResolver {
    netbox_url: String,
    secrets: Arc<Mutex<HashMap<String, String>>>, // namespace/secret_name -> token
}

#[cfg(test)]
impl MockTokenResolver {
    /// Create a new mock TokenResolver
    pub fn new(netbox_url: String) -> Self {
        Self {
            netbox_url,
            secrets: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Add a secret token to the mock
    ///
    /// # Arguments
    /// * `namespace` - Namespace where the secret exists
    /// * `secret_name` - Name of the secret
    /// * `token` - The NetBox API token
    pub fn add_secret(&self, namespace: &str, secret_name: &str, token: String) {
        let key = format!("{}/{}", namespace, secret_name);
        let mut secrets = self.secrets.lock().unwrap();
        secrets.insert(key, token);
    }
    
    /// Resolve token for a tenant reference (mock implementation)
    ///
    /// This simulates the token resolution process by looking up the secret
    /// in the in-memory store. The tenant_ref.name is used to construct the
    /// secret name (following the pattern: netbox-token-{tenant-name}).
    pub async fn resolve_token(
        &self,
        namespace: &str,
        tenant_ref: &NetBoxResourceReference,
    ) -> Result<String, TokenResolutionError> {
        // Simulate secret name pattern: netbox-token-{tenant-name}
        let secret_name = format!("netbox-token-{}", tenant_ref.name);
        let key = format!("{}/{}", namespace, secret_name);
        
        let secrets = self.secrets.lock().unwrap();
        secrets.get(&key)
            .cloned()
            .ok_or_else(|| {
                TokenResolutionError::SecretNotFound(format!(
                    "Secret {} not found in namespace {} (mock)",
                    secret_name, namespace
                ))
            })
    }
    
    /// Create a NetBoxClient with resolved token for a tenant (mock implementation)
    pub async fn create_client_for_tenant(
        &self,
        namespace: &str,
        tenant_ref: &NetBoxResourceReference,
    ) -> Result<NetBoxClient, TokenResolutionError> {
        let token = self.resolve_token(namespace, tenant_ref).await?;
        
        NetBoxClient::new(self.netbox_url.clone(), token)
            .map_err(|e| {
                TokenResolutionError::ClientCreation(e)
            })
    }
    
    /// Get the main tenant reference (datacenter-tenant)
    pub fn get_main_tenant_reference(&self) -> NetBoxResourceReference {
        NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "datacenter-tenant".to_string(),
            namespace: None,
        }
    }
    
    /// Get the NetBox URL
    pub fn netbox_url(&self) -> &str {
        &self.netbox_url
    }
}

/// Helper to create a test reconciler with MockTokenResolver
///
/// This creates a Reconciler with a MockTokenResolver instead of a real TokenResolver,
/// allowing tests to run without a real kube::Client.
#[cfg(test)]
pub fn create_test_reconciler_with_mock_token_resolver(
    mock_token_resolver: Arc<MockTokenResolver>,
) -> crate::reconciler::Reconciler {
    use crate::kube_api_trait::mock::MockKubeApi;
    use crate::reconciler::Reconciler;
    use crds::*;
    
    Reconciler::new(
        // We need to wrap MockTokenResolver in a way that Reconciler accepts
        // For now, this is a placeholder - we'll need to refactor Reconciler
        // to accept a trait instead of concrete TokenResolver
        // TODO: Refactor Reconciler to use a TokenResolverTrait
        // For now, this function is a placeholder showing the intended API
        mock_token_resolver, // This won't compile yet - needs trait refactoring
        // IPAM APIs
        MockKubeApi::new(), // netbox_prefix_api
        MockKubeApi::new(), // netbox_role_api
        MockKubeApi::new(), // netbox_tag_api
        MockKubeApi::new(), // netbox_aggregate_api
        MockKubeApi::new(), // netbox_vlan_api
        MockKubeApi::new(), // netbox_rir_api
        // Tenancy APIs
        MockKubeApi::new(), // netbox_tenant_api
        // DCIM APIs
        MockKubeApi::new(), // netbox_site_api
        MockKubeApi::new(), // netbox_device_role_api
        MockKubeApi::new(), // netbox_manufacturer_api
        MockKubeApi::new(), // netbox_platform_api
        MockKubeApi::new(), // netbox_device_type_api
        MockKubeApi::new(), // netbox_device_api
        MockKubeApi::new(), // netbox_interface_api
        MockKubeApi::new(), // netbox_mac_address_api
        MockKubeApi::new(), // netbox_region_api
        MockKubeApi::new(), // netbox_site_group_api
        MockKubeApi::new(), // netbox_location_api
        // Custom CRDs
        MockKubeApi::new(), // ip_pool_api
        MockKubeApi::new(), // ip_claim_api
    )
}

