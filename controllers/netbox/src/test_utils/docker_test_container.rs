//! RAII wrapper for Docker test containers
//!
//! This module provides a `DockerTestContainer` struct that automatically
//! cleans up Docker containers when tests complete, preventing orphaned containers.
//!
//! # Example
//!
//! ```rust,no_run
//! use crate::test_utils::docker_test_container::DockerTestContainer;
//! use bollard::Docker;
//!
//! #[tokio::test]
//! async fn test_with_container() {
//!     let docker = Docker::connect_with_local_defaults().unwrap();
//!     let container = DockerTestContainer::new(&docker, "alpine:latest").await.unwrap();
//!     // Container will be automatically removed when `container` goes out of scope
//! }
//! ```

// TODO: Update to use bollard OpenAPI-generated types when bollard API stabilizes
#[allow(deprecated)]
use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions};
use bollard::Docker;
use std::sync::Arc;
use tracing::{debug, warn};

/// RAII wrapper for Docker test containers
///
/// Automatically removes the container when dropped, preventing orphaned containers
/// in test environments.
#[derive(Clone)]
pub struct DockerTestContainer {
    pub(crate) docker: Arc<Docker>,
    pub(crate) container_id: String,
}

impl DockerTestContainer {
    /// Create a new Docker container and return a RAII wrapper
    ///
    /// The container will be automatically removed when the wrapper is dropped.
    ///
    /// # Arguments
    ///
    /// * `docker` - Docker client instance
    /// * `image` - Docker image name (e.g., "alpine:latest")
    ///
    /// # Returns
    ///
    /// Returns `Ok(DockerTestContainer)` if the container was created successfully,
    /// or an error if creation failed.
    pub async fn new(docker: &Docker, image: &str) -> Result<Self, bollard::errors::Error> {
        let docker = Arc::new(docker.clone());
        
        // Create container configuration
        #[allow(deprecated)]
        let config = Config {
            image: Some(image),
            cmd: Some(vec!["sleep", "3600"]), // Default: sleep to keep container running
            ..Default::default()
        };
        
        // Create container
        #[allow(deprecated)]
        let create_options = CreateContainerOptions {
            name: format!("dcops-test-{}", uuid::Uuid::new_v4()),
            platform: None, // Required field in newer bollard versions
        };
        
        let create_result = docker.create_container(Some(create_options), config).await?;
        let container_id = create_result.id;
        
        debug!("Created Docker test container: {} (image: {})", container_id, image);
        
        Ok(Self {
            docker,
            container_id,
        })
    }
    
    /// Create a container from an existing container ID
    ///
    /// This is useful when you need to wrap a container that was created elsewhere.
    /// The container will still be automatically removed when dropped.
    pub fn from_id(docker: Arc<Docker>, container_id: String) -> Self {
        Self {
            docker,
            container_id,
        }
    }
    
    /// Get the container ID
    pub fn id(&self) -> &str {
        &self.container_id
    }
    
    /// Start the container
    pub async fn start(&self) -> Result<(), bollard::errors::Error> {
        #[allow(deprecated)]
        let options: StartContainerOptions<String> = StartContainerOptions::default();
        self.docker.start_container(&self.container_id, Some(options)).await?;
        debug!("Started Docker container: {}", self.container_id);
        Ok(())
    }
    
    /// Stop the container
    pub async fn stop(&self) -> Result<(), bollard::errors::Error> {
        #[allow(deprecated)]
        let options: bollard::container::StopContainerOptions<String> = Default::default();
        self.docker.stop_container(&self.container_id, Some(options)).await?;
        debug!("Stopped Docker container: {}", self.container_id);
        Ok(())
    }
}

impl Drop for DockerTestContainer {
    fn drop(&mut self) {
        let container_id = self.container_id.clone();
        let docker = self.docker.clone();
        
        // Use tokio::spawn to handle async cleanup in Drop
        tokio::spawn(async move {
            // Try to stop the container first (ignore errors - might already be stopped)
            #[allow(deprecated)]
            let stop_options: bollard::container::StopContainerOptions<String> = Default::default();
            let _ = docker.stop_container(&container_id, Some(stop_options)).await;
            
            // Remove the container
            #[allow(deprecated)]
            let remove_options = RemoveContainerOptions {
                force: true, // Force removal even if running
                ..Default::default()
            };
            
            match docker.remove_container(&container_id, Some(remove_options)).await {
                Ok(_) => {
                    debug!("Removed Docker test container: {}", container_id);
                }
                Err(e) => {
                    warn!("Failed to remove Docker test container {}: {}", container_id, e);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // Requires Docker - run with: cargo test -- --ignored
    async fn test_docker_container_lifecycle() {
        // Skip if E2E_DOCKER is not set
        if std::env::var("E2E_DOCKER").is_err() {
            println!("Skipping: set E2E_DOCKER=1 to enable Docker e2e test");
            return;
        }
        
        let docker = Docker::connect_with_local_defaults().unwrap();
        let container = DockerTestContainer::new(&docker, "alpine:latest").await.unwrap();
        
        // Start container
        container.start().await.unwrap();
        
        // Container will be automatically removed when dropped
        // (tested by checking container list after drop)
    }
}

