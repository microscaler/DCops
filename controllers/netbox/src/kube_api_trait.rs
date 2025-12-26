//! Trait-based abstraction for Kubernetes API operations
//!
//! This module provides a trait-based abstraction over `kube::Api<T>` to enable
//! mocking for unit tests. The `KubeApiTrait<T>` trait abstracts the operations
//! needed by reconcilers, and both real `Api<T>` instances and mocks can implement it.

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

