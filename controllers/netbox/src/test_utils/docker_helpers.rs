//! Helper functions for Docker container management in tests
//!
//! This module provides utilities for:
//! - Port mapping
//! - Health checks
//! - Container execution
//! - Image building

// TODO: Update to use bollard OpenAPI-generated types when bollard API stabilizes
#[allow(deprecated)]
use bollard::container::{Config, CreateContainerOptions, HostConfig, PortBinding};
use bollard::Docker;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

use super::docker_test_container::DockerTestContainer;

/// Port mapping configuration
#[derive(Debug, Clone)]
pub struct PortMapping {
    pub container_port: u16,
    pub host_port: Option<u16>,
}

impl PortMapping {
    pub fn new(container_port: u16, host_port: Option<u16>) -> Self {
        Self {
            container_port,
            host_port,
        }
    }
}

/// Create a Docker container with port mappings
///
/// # Arguments
///
/// * `docker` - Docker client instance
/// * `image` - Docker image name
/// * `ports` - Vector of port mappings
/// * `cmd` - Optional command to run (defaults to `["sleep", "3600"]`)
///
/// # Returns
///
/// Returns a `DockerTestContainer` wrapper that will automatically clean up the container.
pub async fn create_container_with_ports(
    docker: &Docker,
    image: &str,
    ports: Vec<PortMapping>,
    cmd: Option<Vec<&str>>,
) -> Result<DockerTestContainer, bollard::errors::Error> {
    let docker_arc = Arc::new(docker.clone());
    
    // Build port bindings
    let mut port_bindings = HashMap::new();
    let mut exposed_ports: HashMap<String, HashMap<(), ()>> = HashMap::new();
    
    for port_mapping in &ports {
        let container_port_key = format!("{}/tcp", port_mapping.container_port);
        exposed_ports.insert(container_port_key.clone(), HashMap::new());
        
        let mut binding = vec![];
        if let Some(host_port) = port_mapping.host_port {
            binding.push(PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some(host_port.to_string()),
            });
        } else {
            // Auto-assign host port
            binding.push(PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: None,
            });
        }
        
        port_bindings.insert(container_port_key, Some(binding));
    }
    
    // Create container configuration
    #[allow(deprecated)]
    let config = Config {
        image: Some(image),
        cmd: Some(cmd.unwrap_or_else(|| vec!["sleep", "3600"])),
        exposed_ports: Some(exposed_ports),
        host_config: Some(HostConfig {
            port_bindings: Some(port_bindings),
            ..Default::default()
        }),
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
    
    debug!("Created Docker container with ports: {} (image: {})", container_id, image);
    
    Ok(DockerTestContainer::from_id(docker_arc, container_id))
}

/// Wait for a container to be ready by checking a health endpoint
///
/// # Arguments
///
/// * `container` - Docker container wrapper
/// * `url` - Health check URL (e.g., "http://localhost:8000/health")
/// * `max_attempts` - Maximum number of attempts (default: 30)
/// * `delay_ms` - Delay between attempts in milliseconds (default: 1000)
///
/// # Returns
///
/// Returns `Ok(())` if the health check succeeds, or an error if it times out.
pub async fn wait_for_health_check(
    container: &DockerTestContainer,
    url: &str,
    max_attempts: Option<u32>,
    delay_ms: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let max_attempts = max_attempts.unwrap_or(30);
    let delay_ms = delay_ms.unwrap_or(1000);
    
    for attempt in 1..=max_attempts {
        match reqwest::get(url).await {
            Ok(response) => {
                if response.status().is_success() {
                    debug!("Health check passed for container {}: {}", container.id(), url);
                    return Ok(());
                }
            }
            Err(e) => {
                debug!("Health check attempt {} failed for container {}: {}", attempt, container.id(), e);
            }
        }
        
        if attempt < max_attempts {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        }
    }
    
    Err(format!("Health check timeout for container {}: {}", container.id(), url).into())
}

/// Execute a command in a running container
///
/// # Arguments
///
/// * `container` - Docker container wrapper
/// * `cmd` - Command to execute (e.g., `vec!["echo", "hello"]`)
///
/// # Returns
///
/// Returns the command output as a string, or an error if execution failed.
pub async fn exec_in_container(
    container: &DockerTestContainer,
    cmd: Vec<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    use bollard::exec::CreateExecOptions;
    use futures_util::StreamExt;
    
    // Create exec instance
    let exec_options = CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        cmd: Some(cmd.iter().map(|s| s.to_string()).collect()),
        ..Default::default()
    };
    
    let exec_result = container.docker
        .create_exec(container.id(), exec_options)
        .await?;
    
    // Start exec and collect output
    let mut stream = container.docker.start_exec(&exec_result.id, None).await?;
    let mut output = String::new();
    
    while let Some(result) = stream.next().await {
        match result {
            Ok(bollard::exec::StartExecResults::Attached { output: out, input: _ }) => {
                if let Ok(text) = String::from_utf8(out) {
                    output.push_str(&text);
                }
            }
            Ok(bollard::exec::StartExecResults::Detached) => {
                break;
            }
            Err(e) => {
                return Err(format!("Exec error: {}", e).into());
            }
        }
    }
    
    Ok(output)
}

/// Check if Docker is available
///
/// Returns `true` if Docker is accessible, `false` otherwise.
pub async fn is_docker_available() -> bool {
    match Docker::connect_with_local_defaults() {
        Ok(docker) => {
            match docker.ping().await {
                Ok(_) => true,
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

/// Skip test if Docker is not available
///
/// This is a convenience function for tests that require Docker.
/// It will panic with a helpful message if Docker is not available.
pub async fn require_docker() {
    if !is_docker_available().await {
        panic!("Docker is not available. Set E2E_DOCKER=1 and ensure Docker is running.");
    }
}

use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // Requires Docker - run with: cargo test -- --ignored
    async fn test_create_container_with_ports() {
        if std::env::var("E2E_DOCKER").is_err() {
            println!("Skipping: set E2E_DOCKER=1 to enable Docker e2e test");
            return;
        }
        
        require_docker().await;
        
        let docker = Docker::connect_with_local_defaults().unwrap();
        let ports = vec![
            PortMapping::new(8000, Some(8080)),
            PortMapping::new(67, None), // Auto-assign host port
        ];
        
        let container = create_container_with_ports(&docker, "alpine:latest", ports, None).await.unwrap();
        container.start().await.unwrap();
        
        // Container will be automatically removed when dropped
    }
    
    #[tokio::test]
    async fn test_is_docker_available() {
        let available = is_docker_available().await;
        // This test always passes - just checks if Docker is available
        println!("Docker available: {}", available);
    }
}

