//! Helper functions for ISC Kea DHCP server management in tests
//!
//! This module provides utilities for:
//! - Starting Kea containers
//! - Configuring Kea via Control Agent REST API
//! - Setting up DHCP subnets and pools
//! - Managing static reservations

use super::docker_test_container::DockerTestContainer;
use super::docker_helpers::{create_container_with_ports, PortMapping, wait_for_health_check};
use bollard::Docker;
use serde_json::{json, Value};
use tracing::{debug, info, warn};

/// Kea Control Agent API client
pub struct KeaControlAgent {
    base_url: String,
    client: reqwest::Client,
}

impl KeaControlAgent {
    /// Create a new Kea Control Agent client
    ///
    /// # Arguments
    ///
    /// * `host` - Kea Control Agent host (e.g., "localhost")
    /// * `port` - Kea Control Agent port (default: 8000)
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            base_url: format!("http://{}:{}", host, port),
            client: reqwest::Client::new(),
        }
    }

    /// Execute a Kea command via Control Agent API
    ///
    /// # Arguments
    ///
    /// * `command` - Kea command name (e.g., "config-set", "config-get", "config-test")
    /// * `service` - Service name (e.g., ["dhcp4"])
    /// * `arguments` - Command arguments as JSON value
    ///
    /// # Returns
    ///
    /// Returns the response JSON or an error
    pub async fn execute_command(
        &self,
        command: &str,
        service: Vec<&str>,
        arguments: Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let request = json!({
            "command": command,
            "service": service,
            "arguments": arguments
        });

        debug!("Kea API request: {}", serde_json::to_string_pretty(&request)?);

        let response = self
            .client
            .post(&self.base_url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Kea API error: {} - {}", status, body).into());
        }

        let result: Value = response.json().await?;
        debug!("Kea API response: {}", serde_json::to_string_pretty(&result)?);

        // Check for errors in Kea response
        if let Some(result_array) = result.as_array() {
            for item in result_array {
                if let Some(text) = item.get("text") {
                    if let Some(text_str) = text.as_str() {
                        if text_str.contains("error") || text_str.contains("failed") {
                            return Err(format!("Kea command error: {}", text_str).into());
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Get current Kea configuration
    pub async fn get_config(&self) -> Result<Value, Box<dyn std::error::Error>> {
        self.execute_command("config-get", vec!["dhcp4"], json!({})).await
    }

    /// Test Kea configuration without applying it
    pub async fn test_config(&self, config: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        self.execute_command(
            "config-test",
            vec!["dhcp4"],
            json!({
                "Dhcp4": config
            }),
        )
        .await
    }

    /// Apply Kea configuration
    pub async fn set_config(&self, config: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        self.execute_command(
            "config-set",
            vec!["dhcp4"],
            json!({
                "Dhcp4": config
            }),
        )
        .await
    }
}

/// Kea DHCP subnet configuration
#[derive(Debug, Clone)]
pub struct KeaSubnet {
    pub subnet: String, // CIDR notation (e.g., "192.168.1.0/24")
    pub pools: Vec<KeaPool>,
    pub reservations: Vec<KeaReservation>,
}

/// Kea DHCP pool configuration
#[derive(Debug, Clone)]
pub struct KeaPool {
    pub pool: String, // Range notation (e.g., "192.168.1.100-192.168.1.200")
}

/// Kea DHCP reservation configuration
#[derive(Debug, Clone)]
pub struct KeaReservation {
    pub ip_address: String, // IP address (e.g., "192.168.1.100")
    pub hw_address: String, // MAC address (e.g., "aa:bb:cc:dd:ee:ff")
}

/// Start an ISC Kea DHCP server container
///
/// # Arguments
///
/// * `docker` - Docker client instance
/// * `image` - Kea Docker image (default: "iscorg/kea:latest")
/// * `control_agent_port` - Port for Kea Control Agent (default: 8000)
///
/// # Returns
///
/// Returns a `DockerTestContainer` wrapper and a `KeaControlAgent` client
pub async fn start_kea_container(
    docker: &Docker,
    image: Option<&str>,
    control_agent_port: Option<u16>,
) -> Result<(DockerTestContainer, KeaControlAgent), Box<dyn std::error::Error>> {
    let image = image.unwrap_or("iscorg/kea:latest");
    let control_agent_port = control_agent_port.unwrap_or(8000);

    info!("Starting ISC Kea DHCP server container: {}", image);

    // Create container with port mappings
    let ports = vec![
        PortMapping::new(67, None),                    // DHCP server port (UDP)
        PortMapping::new(control_agent_port, Some(8000)), // Control Agent (TCP)
    ];

    let container = create_container_with_ports(docker, image, ports, None).await?;
    container.start().await?;

    // Wait for Kea Control Agent to be ready
    let health_url = format!("http://localhost:8000");
    wait_for_health_check(&container, &health_url, Some(30), Some(1000)).await?;

    info!("Kea Control Agent is ready at {}", health_url);

    // Create Kea Control Agent client
    let kea_client = KeaControlAgent::new("localhost", 8000);

    Ok((container, kea_client))
}

/// Configure a DHCP subnet in Kea
///
/// # Arguments
///
/// * `kea_client` - Kea Control Agent client
/// * `subnet` - Subnet configuration
///
/// # Returns
///
/// Returns `Ok(())` if configuration was successful
pub async fn configure_kea_subnet(
    kea_client: &KeaControlAgent,
    subnet: &KeaSubnet,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Configuring Kea subnet: {}", subnet.subnet);

    // Get current configuration
    let current_config = kea_client.get_config().await?;

    // Extract current subnets or create new config
    let mut dhcp4_config = if let Some(dhcp4) = current_config.get("arguments").and_then(|a| a.get("Dhcp4")) {
        dhcp4.clone()
    } else {
        json!({
            "interfaces-config": {
                "interfaces": ["*"]
            },
            "lease-database": {
                "type": "memfile"
            },
            "subnet4": []
        })
    };

    // Build subnet configuration
    let mut subnet_config = json!({
        "subnet": subnet.subnet,
        "pools": subnet.pools.iter().map(|p| json!({
            "pool": p.pool
        })).collect::<Vec<_>>(),
        "reservations": subnet.reservations.iter().map(|r| json!({
            "ip-address": r.ip_address,
            "hw-address": r.hw_address
        })).collect::<Vec<_>>()
    });

    // Add subnet to configuration
    let subnets = dhcp4_config
        .get_mut("subnet4")
        .and_then(|s| s.as_array_mut())
        .ok_or("subnet4 not found in Kea config")?;

    // Check if subnet already exists
    let subnet_exists = subnets.iter().any(|s| {
        s.get("subnet")
            .and_then(|v| v.as_str())
            .map(|s| s == subnet.subnet)
            .unwrap_or(false)
    });

    if subnet_exists {
        warn!("Subnet {} already exists, updating", subnet.subnet);
        // Update existing subnet (simplified - in production, merge pools/reservations)
        for s in subnets.iter_mut() {
            if s.get("subnet").and_then(|v| v.as_str()) == Some(&subnet.subnet) {
                *s = subnet_config;
                break;
            }
        }
    } else {
        subnets.push(subnet_config);
    }

    // Test configuration before applying
    info!("Testing Kea configuration...");
    kea_client.test_config(&dhcp4_config).await?;

    // Apply configuration
    info!("Applying Kea configuration...");
    kea_client.set_config(&dhcp4_config).await?;

    info!("Kea subnet {} configured successfully", subnet.subnet);
    Ok(())
}

/// Add a static reservation to an existing Kea subnet
///
/// # Arguments
///
/// * `kea_client` - Kea Control Agent client
/// * `subnet` - Subnet CIDR (e.g., "192.168.1.0/24")
/// * `reservation` - Reservation to add
///
/// # Returns
///
/// Returns `Ok(())` if reservation was added successfully
pub async fn add_kea_reservation(
    kea_client: &KeaControlAgent,
    subnet: &str,
    reservation: &KeaReservation,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Adding Kea reservation: {} -> {}", reservation.hw_address, reservation.ip_address);

    // Get current configuration
    let current_config = kea_client.get_config().await?;
    let mut dhcp4_config = current_config
        .get("arguments")
        .and_then(|a| a.get("Dhcp4"))
        .ok_or("Dhcp4 config not found")?
        .clone();

    // Find the subnet and add reservation
    let subnets = dhcp4_config
        .get_mut("subnet4")
        .and_then(|s| s.as_array_mut())
        .ok_or("subnet4 not found")?;

    for subnet_config in subnets.iter_mut() {
        if subnet_config.get("subnet").and_then(|v| v.as_str()) == Some(subnet) {
            let reservations = subnet_config
                .get_mut("reservations")
                .and_then(|r| r.as_array_mut())
                .unwrap_or_else(|| {
                    let arr = json!([]);
                    subnet_config["reservations"] = arr;
                    subnet_config["reservations"].as_array_mut().unwrap()
                });

            // Check if reservation already exists
            let exists = reservations.iter().any(|r| {
                r.get("hw-address")
                    .and_then(|v| v.as_str())
                    .map(|h| h == reservation.hw_address)
                    .unwrap_or(false)
            });

            if !exists {
                reservations.push(json!({
                    "ip-address": reservation.ip_address,
                    "hw-address": reservation.hw_address
                }));
            }

            // Test and apply configuration
            kea_client.test_config(&dhcp4_config).await?;
            kea_client.set_config(&dhcp4_config).await?;

            info!("Kea reservation added successfully");
            return Ok(());
        }
    }

    Err(format!("Subnet {} not found in Kea configuration", subnet).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Server, Matcher};

    #[tokio::test]
    async fn test_kea_control_agent_new() {
        let agent = KeaControlAgent::new("localhost", 8000);
        assert_eq!(agent.base_url, "http://localhost:8000");
    }

    #[tokio::test]
    async fn test_kea_control_agent_execute_command_success() {
        let mut server = Server::new_async().await;
        
        // Mock successful Kea API response
        let mock = server
            .mock("POST", "/")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 0, "text": "Configuration found.", "arguments": {"Dhcp4": {"interfaces-config": {"interfaces": ["*"]}}}}]"#)
            .create();

        let host = server.host();
        let port = server.port();
        let agent = KeaControlAgent::new(host, port);
        let result = agent.execute_command("config-get", vec!["dhcp4"], json!({})).await;

        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(value.is_array());
        
        mock.assert();
    }

    #[tokio::test]
    async fn test_kea_control_agent_execute_command_http_error() {
        let mut server = Server::new_async().await;
        
        // Mock HTTP error response
        let mock = server
            .mock("POST", "/")
            .with_status(500)
            .with_body("Internal Server Error")
            .create();

        let host = server.host();
        let port = server.port();
        let agent = KeaControlAgent::new(host, port);
        let result = agent.execute_command("config-get", vec!["dhcp4"], json!({})).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Kea API error"));
        
        mock.assert();
    }

    #[tokio::test]
    async fn test_kea_control_agent_execute_command_kea_error() {
        let mut server = Server::new_async().await;
        
        // Mock Kea error response (HTTP 200 but error in response)
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 1, "text": "Configuration error: invalid subnet"}]"#)
            .create();

        let host = server.host();
        let port = server.port();
        let agent = KeaControlAgent::new(host, port);
        let result = agent.execute_command("config-set", vec!["dhcp4"], json!({})).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Kea command error"));
        
        mock.assert();
    }

    #[tokio::test]
    async fn test_kea_control_agent_get_config() {
        let mut server = Server::new_async().await;
        
        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 0, "text": "Configuration found.", "arguments": {"Dhcp4": {"subnet4": []}}}]"#)
            .create();

        let host = server.host();
        let port = server.port();
        let agent = KeaControlAgent::new(host, port);
        let result = agent.get_config().await;

        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.get("arguments").is_some());
        
        mock.assert();
    }

    #[tokio::test]
    async fn test_kea_control_agent_test_config() {
        let mut server = Server::new_async().await;
        
        let test_config = json!({
            "interfaces-config": {"interfaces": ["*"]},
            "subnet4": [{"subnet": "192.168.1.0/24"}]
        });

        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 0, "text": "Configuration is valid"}]"#)
            .create();

        let host = server.host();
        let port = server.port();
        let agent = KeaControlAgent::new(host, port);
        let result = agent.test_config(&test_config).await;

        assert!(result.is_ok());
        
        mock.assert();
    }

    #[tokio::test]
    async fn test_kea_control_agent_set_config() {
        let mut server = Server::new_async().await;
        
        let test_config = json!({
            "interfaces-config": {"interfaces": ["*"]},
            "subnet4": [{"subnet": "192.168.1.0/24"}]
        });

        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 0, "text": "Configuration successful"}]"#)
            .create();

        let host = server.host();
        let port = server.port();
        let agent = KeaControlAgent::new(host, port);
        let result = agent.set_config(&test_config).await;

        assert!(result.is_ok());
        
        mock.assert();
    }

    #[tokio::test]
    #[ignore] // Requires Docker - run with: cargo test -- --ignored
    async fn test_start_kea_container() {
        if std::env::var("E2E_DOCKER").is_err() {
            println!("Skipping: set E2E_DOCKER=1 to enable Docker e2e test");
            return;
        }

        super::super::docker_helpers::require_docker().await;

        let docker = Docker::connect_with_local_defaults().unwrap();
        let (_container, kea_client) = start_kea_container(&docker, None, None).await.unwrap();

        // Test getting configuration
        let config = kea_client.get_config().await.unwrap();
        println!("Kea config: {}", serde_json::to_string_pretty(&config).unwrap());
    }

    #[tokio::test]
    #[ignore] // Requires Docker - run with: cargo test -- --ignored
    async fn test_configure_kea_subnet() {
        if std::env::var("E2E_DOCKER").is_err() {
            println!("Skipping: set E2E_DOCKER=1 to enable Docker e2e test");
            return;
        }

        super::super::docker_helpers::require_docker().await;

        let docker = Docker::connect_with_local_defaults().unwrap();
        let (_container, kea_client) = start_kea_container(&docker, None, None).await.unwrap();

        // Configure a test subnet
        let subnet = KeaSubnet {
            subnet: "192.168.1.0/24".to_string(),
            pools: vec![KeaPool {
                pool: "192.168.1.100-192.168.1.200".to_string(),
            }],
            reservations: vec![],
        };

        configure_kea_subnet(&kea_client, &subnet).await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Docker - run with: cargo test -- --ignored
    async fn test_kea_static_reservation() {
        if std::env::var("E2E_DOCKER").is_err() {
            println!("Skipping: set E2E_DOCKER=1 to enable Docker e2e test");
            return;
        }

        super::super::docker_helpers::require_docker().await;

        let docker = Docker::connect_with_local_defaults().unwrap();
        let (_container, kea_client) = start_kea_container(&docker, None, None).await.unwrap();

        // First, configure a subnet
        let subnet = KeaSubnet {
            subnet: "192.168.1.0/24".to_string(),
            pools: vec![KeaPool {
                pool: "192.168.1.100-192.168.1.200".to_string(),
            }],
            reservations: vec![],
        };

        configure_kea_subnet(&kea_client, &subnet).await.unwrap();

        // Add a static reservation
        let reservation = KeaReservation {
            ip_address: "192.168.1.100".to_string(),
            hw_address: "aa:bb:cc:dd:ee:ff".to_string(),
        };

        add_kea_reservation(&kea_client, "192.168.1.0/24", &reservation)
            .await
            .unwrap();

        // Verify reservation was added by getting config
        let config = kea_client.get_config().await.unwrap();
        println!("Kea config with reservation: {}", serde_json::to_string_pretty(&config).unwrap());
    }

    #[tokio::test]
    async fn test_add_kea_reservation_duplicate() {
        let mut server = Server::new_async().await;
        
        // Mock config with existing subnet and reservation
        let existing_config = json!([{
            "result": 0,
            "text": "Configuration found.",
            "arguments": {
                "Dhcp4": {
                    "subnet4": [{
                        "subnet": "192.168.1.0/24",
                        "reservations": [{
                            "ip-address": "192.168.1.100",
                            "hw-address": "aa:bb:cc:dd:ee:ff"
                        }]
                    }]
                }
            }
        }]);

        let get_config_mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&existing_config).unwrap())
            .create();

        // Mock test_config and set_config (should be called even for duplicate)
        let test_config_mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 0, "text": "Configuration is valid"}]"#)
            .create();

        let set_config_mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 0, "text": "Configuration successful"}]"#)
            .create();

        let host = server.host();
        let port = server.port();
        let agent = KeaControlAgent::new(host, port);
        
        let reservation = KeaReservation {
            ip_address: "192.168.1.100".to_string(),
            hw_address: "aa:bb:cc:dd:ee:ff".to_string(),
        };

        // Adding duplicate reservation should succeed (it checks and skips)
        let result = add_kea_reservation(&agent, "192.168.1.0/24", &reservation).await;
        
        // Should succeed (duplicate is silently skipped)
        assert!(result.is_ok());
        
        get_config_mock.assert();
        test_config_mock.assert();
        set_config_mock.assert();
    }

    #[tokio::test]
    async fn test_add_kea_reservation_subnet_not_found() {
        let mut server = Server::new_async().await;
        
        // Mock config without the target subnet
        let config = json!([{
            "result": 0,
            "text": "Configuration found.",
            "arguments": {
                "Dhcp4": {
                    "subnet4": [{
                        "subnet": "10.0.0.0/24"
                    }]
                }
            }
        }]);

        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&config).unwrap())
            .create();

        let host = server.host();
        let port = server.port();
        let agent = KeaControlAgent::new(host, port);
        
        let reservation = KeaReservation {
            ip_address: "192.168.1.100".to_string(),
            hw_address: "aa:bb:cc:dd:ee:ff".to_string(),
        };

        let result = add_kea_reservation(&agent, "192.168.1.0/24", &reservation).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Subnet 192.168.1.0/24 not found"));
        
        mock.assert();
    }

    #[tokio::test]
    async fn test_add_kea_reservation_missing_dhcp4_config() {
        let mut server = Server::new_async().await;
        
        // Mock config without Dhcp4
        let config = json!([{
            "result": 0,
            "text": "Configuration found.",
            "arguments": {}
        }]);

        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&config).unwrap())
            .create();

        let host = server.host();
        let port = server.port();
        let agent = KeaControlAgent::new(host, port);
        
        let reservation = KeaReservation {
            ip_address: "192.168.1.100".to_string(),
            hw_address: "aa:bb:cc:dd:ee:ff".to_string(),
        };

        let result = add_kea_reservation(&agent, "192.168.1.0/24", &reservation).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Dhcp4 config not found"));
        
        mock.assert();
    }

    #[tokio::test]
    async fn test_configure_kea_subnet_duplicate_updates() {
        let mut server = Server::new_async().await;
        
        // Mock config with existing subnet
        let existing_config = json!([{
            "result": 0,
            "text": "Configuration found.",
            "arguments": {
                "Dhcp4": {
                    "subnet4": [{
                        "subnet": "192.168.1.0/24",
                        "pools": [{"pool": "192.168.1.100-192.168.1.150"}]
                    }]
                }
            }
        }]);

        let get_config_mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&existing_config).unwrap())
            .create();

        let test_config_mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 0, "text": "Configuration is valid"}]"#)
            .create();

        let set_config_mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 0, "text": "Configuration successful"}]"#)
            .create();

        let host = server.host();
        let port = server.port();
        let agent = KeaControlAgent::new(host, port);
        
        // Configure same subnet with different pool
        let subnet = KeaSubnet {
            subnet: "192.168.1.0/24".to_string(),
            pools: vec![KeaPool {
                pool: "192.168.1.200-192.168.1.250".to_string(),
            }],
            reservations: vec![],
        };

        let result = configure_kea_subnet(&agent, &subnet).await;
        
        // Should succeed (updates existing subnet)
        assert!(result.is_ok());
        
        get_config_mock.assert();
        test_config_mock.assert();
        set_config_mock.assert();
    }

    #[tokio::test]
    async fn test_configure_kea_subnet_test_config_failure() {
        let mut server = Server::new_async().await;
        
        // Mock config get
        let get_config_mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 0, "text": "Configuration found.", "arguments": {"Dhcp4": {"subnet4": []}}}]"#)
            .create();

        // Mock test_config failure (invalid configuration)
        let test_config_mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 1, "text": "Configuration error: invalid subnet format"}]"#)
            .create();

        let host = server.host();
        let port = server.port();
        let agent = KeaControlAgent::new(host, port);
        
        let subnet = KeaSubnet {
            subnet: "192.168.1.0/24".to_string(),
            pools: vec![KeaPool {
                pool: "192.168.1.100-192.168.1.200".to_string(),
            }],
            reservations: vec![],
        };

        let result = configure_kea_subnet(&agent, &subnet).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Kea command error"));
        
        get_config_mock.assert();
        test_config_mock.assert();
    }

    #[tokio::test]
    async fn test_configure_kea_subnet_set_config_failure() {
        let mut server = Server::new_async().await;
        
        // Mock config get
        let get_config_mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 0, "text": "Configuration found.", "arguments": {"Dhcp4": {"subnet4": []}}}]"#)
            .create();

        // Mock test_config success
        let test_config_mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 0, "text": "Configuration is valid"}]"#)
            .create();

        // Mock set_config failure
        let set_config_mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 1, "text": "Failed to apply configuration"}]"#)
            .create();

        let host = server.host();
        let port = server.port();
        let agent = KeaControlAgent::new(host, port);
        
        let subnet = KeaSubnet {
            subnet: "192.168.1.0/24".to_string(),
            pools: vec![KeaPool {
                pool: "192.168.1.100-192.168.1.200".to_string(),
            }],
            reservations: vec![],
        };

        let result = configure_kea_subnet(&agent, &subnet).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Kea command error"));
        
        get_config_mock.assert();
        test_config_mock.assert();
        set_config_mock.assert();
    }

    #[tokio::test]
    async fn test_configure_kea_subnet_missing_arguments() {
        let mut server = Server::new_async().await;
        
        // Mock config without arguments (empty config)
        let get_config_mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 0, "text": "Configuration found.", "arguments": {}}]"#)
            .create();

        let test_config_mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 0, "text": "Configuration is valid"}]"#)
            .create();

        let set_config_mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"result": 0, "text": "Configuration successful"}]"#)
            .create();

        let host = server.host();
        let port = server.port();
        let agent = KeaControlAgent::new(host, port);
        
        let subnet = KeaSubnet {
            subnet: "192.168.1.0/24".to_string(),
            pools: vec![KeaPool {
                pool: "192.168.1.100-192.168.1.200".to_string(),
            }],
            reservations: vec![],
        };

        // Should succeed (creates new config structure)
        let result = configure_kea_subnet(&agent, &subnet).await;
        
        assert!(result.is_ok());
        
        get_config_mock.assert();
        test_config_mock.assert();
        set_config_mock.assert();
    }

    #[tokio::test]
    async fn test_add_kea_reservation_missing_subnet4() {
        let mut server = Server::new_async().await;
        
        // Mock config with Dhcp4 but no subnet4
        let config = json!([{
            "result": 0,
            "text": "Configuration found.",
            "arguments": {
                "Dhcp4": {
                    "interfaces-config": {"interfaces": ["*"]}
                }
            }
        }]);

        let mock = server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&config).unwrap())
            .create();

        let host = server.host();
        let port = server.port();
        let agent = KeaControlAgent::new(host, port);
        
        let reservation = KeaReservation {
            ip_address: "192.168.1.100".to_string(),
            hw_address: "aa:bb:cc:dd:ee:ff".to_string(),
        };

        let result = add_kea_reservation(&agent, "192.168.1.0/24", &reservation).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("subnet4 not found"));
        
        mock.assert();
    }
}

