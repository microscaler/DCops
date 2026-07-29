//! Kea Client - Main client struct

use crate::error::ControllerError;
use crate::kea::api::KeaApi;
use crate::kea::commands::KeaCommands;
use std::time::Duration;
use crate::types::KEA_API_TIMEOUT_SECS;

/// Kea Control Agent API client
pub struct KeaClient {
    api: KeaApi,
    commands: KeaCommands,
}

impl KeaClient {
    /// Create a new Kea Control Agent client
    ///
    /// # Arguments
    ///
    /// * `base_url` - Kea Control Agent base URL (e.g., "http://localhost:8000")
    pub fn new(base_url: String) -> Self {
        let api = KeaApi::new(base_url.clone(), Duration::from_secs(KEA_API_TIMEOUT_SECS));
        let commands = KeaCommands::new(api.clone());
        
        Self {
            api,
            commands,
        }
    }

    /// Get the API client
    pub fn api(&self) -> &KeaApi {
        &self.api
    }

    /// Get the commands client
    pub fn commands(&self) -> &KeaCommands {
        &self.commands
    }

    // Convenience methods that delegate to commands
    // These maintain backward compatibility with existing code

    /// Get current Kea configuration
    pub async fn get_config(&self) -> Result<serde_json::Value, ControllerError> {
        self.commands.config_get().await
    }

    /// Test Kea configuration without applying it
    pub async fn test_config(&self, config: &serde_json::Value) -> Result<serde_json::Value, ControllerError> {
        self.commands.config_test(config).await
    }

    /// Apply Kea configuration
    pub async fn set_config(&self, config: &serde_json::Value) -> Result<serde_json::Value, ControllerError> {
        self.commands.config_set(config).await
    }
}

