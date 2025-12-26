//! Mock implementation of KubeApiTrait for unit testing
//!
//! This module provides a mock implementation that stores resources in memory
//! and can be used for unit testing reconcilers without a real Kubernetes cluster.

#[cfg(test)]
use crate::kube_api_trait::KubeApiTrait;
#[cfg(test)]
use kube::api::{ListParams, Patch, PatchParams};
#[cfg(test)]
use kube::Resource;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::{Arc, Mutex};

/// Mock implementation of KubeApiTrait for testing
#[cfg(test)]
pub struct MockKubeApi<T> {
    resources: Arc<Mutex<HashMap<String, T>>>,
}

#[cfg(test)]
impl<T> MockKubeApi<T>
where
    T: Resource + Clone + Send + Sync + 'static,
    <T as Resource>::DynamicType: Send + Sync,
{
    /// Create a new mock API with empty resource store
    pub fn new() -> Self {
        Self {
            resources: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Store a resource in the mock
    pub fn store(&self, name: String, resource: T) {
        let mut resources = self.resources.lock().unwrap();
        resources.insert(name, resource);
    }

    /// Get a resource from the mock (for test setup)
    pub fn get_resource(&self, name: &str) -> Option<T> {
        let resources = self.resources.lock().unwrap();
        resources.get(name).cloned()
    }

    /// Clear all resources (for test cleanup)
    pub fn clear(&self) {
        let mut resources = self.resources.lock().unwrap();
        resources.clear();
    }
}

#[cfg(test)]
impl<T> Default for MockKubeApi<T>
where
    T: Resource + Clone + Send + Sync + 'static,
    <T as Resource>::DynamicType: Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
#[cfg(test)]
impl<T> KubeApiTrait<T> for MockKubeApi<T>
where
    T: Resource + Clone + Send + Sync + 'static,
    <T as Resource>::DynamicType: Send + Sync,
{
    async fn get(&self, name: &str) -> Result<T, kube::Error> {
        let resources = self.resources.lock().unwrap();
        resources
            .get(name)
            .cloned()
            .ok_or_else(|| kube::Error::Api(kube::error::ErrorResponse {
                code: 404,
                message: format!("Resource '{}' not found", name),
                reason: "NotFound".to_string(),
                status: "Failure".to_string(),
            }))
    }

    async fn patch_status(
        &self,
        name: &str,
        _params: &PatchParams,
        patch: &Patch<serde_json::Value>,
    ) -> Result<T, kube::Error> {
        let mut resources = self.resources.lock().unwrap();
        let resource = resources
            .get_mut(name)
            .ok_or_else(|| kube::Error::Api(kube::error::ErrorResponse {
                code: 404,
                message: format!("Resource '{}' not found", name),
                reason: "NotFound".to_string(),
                status: "Failure".to_string(),
            }))?;

        // Apply the patch to the resource's status
        // For Merge patches, we merge the status JSON
        match patch {
            Patch::Merge(patch_json) => {
                if let Some(status_value) = patch_json.get("status") {
                    // Update the resource's status field
                    // This is a simplified implementation - in reality, we'd need to
                    // properly merge the status using serde_json
                    let resource_json = serde_json::to_value(resource).map_err(|e| {
                        kube::Error::SerdeError(format!("Failed to serialize resource: {}", e))
                    })?;
                    let mut merged = resource_json;
                    merged["status"] = status_value.clone();
                    *resource = serde_json::from_value(merged).map_err(|e| {
                        kube::Error::SerdeError(format!("Failed to deserialize patched resource: {}", e))
                    })?;
                }
            }
            _ => {
                return Err(kube::Error::Api(kube::error::ErrorResponse {
                    code: 400,
                    message: "Only Merge patches are supported in mock".to_string(),
                    reason: "BadRequest".to_string(),
                    status: "Failure".to_string(),
                }));
            }
        }

        Ok(resource.clone())
    }

    async fn list(&self, _params: &ListParams) -> Result<kube::api::ObjectList<T>, kube::Error> {
        let resources = self.resources.lock().unwrap();
        let items: Vec<T> = resources.values().cloned().collect();
        Ok(kube::api::ObjectList {
            items,
            metadata: kube::core::ObjectMeta::default(),
        })
    }
}

