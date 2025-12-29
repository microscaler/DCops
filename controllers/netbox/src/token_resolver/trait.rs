//! TokenResolverTrait - Trait for token resolution and client creation
//!
//! This trait allows for dependency injection and mocking in tests.

use crate::token_resolver::TokenResolutionError;
use crds::NetBoxResourceReference;
use kube::Client;
use netbox_client::NetBoxClient;

/// Trait for resolving NetBox API tokens and creating clients
///
/// This trait abstracts token resolution to enable:
/// - Dependency injection
/// - Mocking in unit tests
/// - Different implementations (real TokenResolver, MockTokenResolver, etc.)
#[async_trait::async_trait]
pub trait TokenResolverTrait: Send + Sync {
    /// Create a NetBoxClient with resolved token for a tenant
    ///
    /// This is the SINGLE POINT of NetBoxClient creation with tenant tokens.
    /// All tenant-specific client creation flows through this method.
    ///
    /// # Arguments
    /// * `namespace` - Namespace where the resource exists
    /// * `tenant_ref` - Reference to the NetBoxTenant CRD
    ///
    /// # Returns
    /// A NetBoxClient instance configured with the tenant's token
    async fn create_client_for_tenant(
        &self,
        namespace: &str,
        tenant_ref: &NetBoxResourceReference,
    ) -> Result<NetBoxClient, TokenResolutionError>;

    /// Create a NetBoxClient for a shared resource
    ///
    /// Shared resources don't have a tenant reference, so we need to resolve
    /// the tenant by finding a resource that references this shared resource.
    ///
    /// # Arguments
    /// * `namespace` - Namespace where the shared resource exists
    /// * `resource_kind` - Kind of the shared resource (e.g., "NetBoxManufacturer")
    /// * `resource_name` - Name of the shared resource CRD
    ///
    /// # Returns
    /// A NetBoxClient instance configured with the resolved tenant's token
    async fn create_client_for_shared_resource(
        &self,
        namespace: &str,
        resource_kind: &str,
        resource_name: &str,
    ) -> Result<NetBoxClient, TokenResolutionError>;

    /// Get a reference to the kube client
    ///
    /// This is needed for special cases like NetBoxTenant reconciler
    /// which needs to read the secret directly to avoid circular dependencies.
    fn kube_client(&self) -> &Client;

    /// Get the NetBox URL
    ///
    /// This is needed for creating NetBoxClient instances directly
    /// (used in special cases like NetBoxTenant reconciler).
    fn netbox_url(&self) -> &str;
}

