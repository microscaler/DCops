//! In-memory Kubernetes resource store
//!
//! Stores CRD objects in memory to simulate Kubernetes API responses.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use kube::Resource;
use serde::{Serialize, Deserialize};

/// Mock Kubernetes API store
/// 
/// This stores CRD objects in memory and can be used to simulate
/// Kubernetes API responses for testing.
#[cfg(test)]
#[derive(Clone)]
pub struct MockKubeStore {
    /// In-memory storage for CRDs by kind and name
    resources: Arc<Mutex<HashMap<String, HashMap<String, serde_json::Value>>>>,
}

#[cfg(test)]
impl MockKubeStore {
    pub fn new() -> Self {
        Self {
            resources: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get the resource key for a type
    fn resource_key<T>() -> String
    where
        T: Resource,
        T::DynamicType: Default,
    {
        let dt = T::DynamicType::default();
        format!("{}/{}", dt.kind, dt.group)
    }

    /// Store a CRD resource
    pub fn store<T>(&self, resource: &T) -> Result<(), String>
    where
        T: Resource + Serialize,
        T::DynamicType: Default,
    {
        let key = Self::resource_key::<T>();
        let name = resource.meta().name.as_ref()
            .ok_or_else(|| "Resource missing name".to_string())?;
        
        let json = serde_json::to_value(resource)
            .map_err(|e| format!("Failed to serialize resource: {}", e))?;
        
        let mut resources = self.resources.lock().unwrap();
        resources.entry(key)
            .or_insert_with(HashMap::new)
            .insert(name.clone(), json);
        
        Ok(())
    }

    /// Get a CRD resource
    pub fn get<T>(&self, name: &str) -> Result<Option<T>, String>
    where
        T: Resource + for<'de> Deserialize<'de>,
        T::DynamicType: Default,
    {
        let key = Self::resource_key::<T>();
        let resources = self.resources.lock().unwrap();
        
        if let Some(resource_map) = resources.get(&key) {
            if let Some(json) = resource_map.get(name) {
                let resource: T = serde_json::from_value(json.clone())
                    .map_err(|e| format!("Failed to deserialize resource: {}", e))?;
                return Ok(Some(resource));
            }
        }
        
        Ok(None)
    }

    /// Check if a resource exists
    pub fn exists<T>(&self, name: &str) -> bool
    where
        T: Resource,
        T::DynamicType: Default,
    {
        let key = Self::resource_key::<T>();
        let resources = self.resources.lock().unwrap();
        
        resources.get(&key)
            .and_then(|m| m.get(name))
            .is_some()
    }

    /// Clear all resources
    pub fn clear(&self) {
        let mut resources = self.resources.lock().unwrap();
        resources.clear();
    }
}

#[cfg(test)]
impl Default for MockKubeStore {
    fn default() -> Self {
        Self::new()
    }
}

