//! Trait-based abstraction for Kubernetes API operations
//!
//! This module provides a trait-based abstraction over `kube::Api<T>` to enable
//! mocking for unit tests. The `KubeApiTrait<T>` trait abstracts the operations
//! needed by reconcilers, and both real `Api<T>` instances and mocks can implement it.
//!
//! ## Critical: Real Cluster Operation Preserved
//!
//! **This trait-based approach does NOT replace real cluster operation.**
//!
//! - **Production**: `KubeApiWrapper` is a thin delegation layer that forwards all calls
//!   directly to the underlying `kube::Api<T>`. There is zero performance overhead - just
//!   a function call indirection.
//! - **Watcher**: Still uses real `Api<T>` directly (unchanged, not affected by trait)
//! - **Integration Tests**: Continue to work with real Kubernetes clusters
//! - **All real cluster functionality remains 100% intact**
//!
//! The wrapper exists solely to enable unit testing with mocks, not to replace
//! real cluster operations.

#[cfg(test)]
pub mod mock;

use crate::error::ControllerError;
use kube::api::{ListParams, Patch, PatchParams};
use kube::Resource;

/// Trait for Kubernetes API operations
///
/// This trait abstracts the operations needed by reconcilers, enabling both
/// real `kube::Api<T>` instances and mocks to be used interchangeably.
#[async_trait::async_trait]
pub trait KubeApiTrait<T>: Send + Sync
where
    T: Resource + Clone + Send + Sync + 'static,
    <T as Resource>::DynamicType: Send + Sync,
{
    /// Get a resource by name
    async fn get(&self, name: &str) -> Result<T, kube::Error>;

    /// Patch the status subresource
    async fn patch_status(
        &self,
        name: &str,
        params: &PatchParams,
        patch: &Patch<serde_json::Value>,
    ) -> Result<T, kube::Error>;

    /// List resources with optional parameters
    async fn list(&self, params: &ListParams) -> Result<kube::api::ObjectList<T>, kube::Error>;
}

/// Wrapper around `kube::Api<T>` that implements `KubeApiTrait<T>`
///
/// This allows real `Api<T>` instances to be used through the trait interface.
///
/// ## Critical: Zero Overhead Delegation
///
/// This wrapper is a **thin delegation layer** that forwards all calls directly
/// to the underlying `kube::Api<T>`. It provides:
/// - **Zero performance overhead** - just a function call indirection
/// - **100% compatibility** - all kube-rs behavior preserved
/// - **Full real cluster operation** - all calls go to real Kubernetes API
///
/// The wrapper exists solely to enable unit testing with mocks, not to replace
/// real cluster operations.
pub struct KubeApiWrapper<T> {
    api: kube::Api<T>,
}

impl<T> KubeApiWrapper<T>
where
    T: Resource + Clone + Send + Sync + 'static,
    <T as Resource>::DynamicType: Send + Sync,
{
    /// Create a new wrapper from a `kube::Api<T>`
    pub fn new(api: kube::Api<T>) -> Self {
        Self { api }
    }

    /// Get the underlying `Api<T>` (for cases where direct access is needed)
    pub fn inner(&self) -> &kube::Api<T> {
        &self.api
    }
}

#[async_trait::async_trait]
impl<T> KubeApiTrait<T> for KubeApiWrapper<T>
where
    T: Resource + Clone + Send + Sync + 'static,
    <T as Resource>::DynamicType: Send + Sync,
{
    async fn get(&self, name: &str) -> Result<T, kube::Error> {
        self.api.get(name).await
    }

    async fn patch_status(
        &self,
        name: &str,
        params: &PatchParams,
        patch: &Patch<serde_json::Value>,
    ) -> Result<T, kube::Error> {
        self.api.patch_status(name, params, patch).await
    }

    async fn list(&self, params: &ListParams) -> Result<kube::api::ObjectList<T>, kube::Error> {
        self.api.list(params).await
    }
}

