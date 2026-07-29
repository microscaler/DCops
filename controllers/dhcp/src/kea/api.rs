//! Kea API - HTTP communication layer

use crate::error::ControllerError;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::debug;
use std::sync::Arc;

/// Kea API client for HTTP communication
#[derive(Clone)]
pub struct KeaApi {
    base_url: String,
    client: Arc<reqwest::Client>,
}

impl KeaApi {
    /// Create a new Kea API client
    ///
    /// # Arguments
    ///
    /// * `base_url` - Kea Control Agent base URL
    /// * `timeout` - Request timeout
    pub fn new(base_url: String, timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            base_url,
            client: Arc::new(client),
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
    ) -> Result<Value, ControllerError> {
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
            return Err(ControllerError::KeaApi(format!("HTTP {} - {}", status, body)));
        }

        let result: Value = response.json().await?;
        debug!("Kea API response: {}", serde_json::to_string_pretty(&result)?);

        // Check for errors in Kea response
        self.check_kea_response_errors(&result)?;

        Ok(result)
    }

    /// Check Kea response for errors
    ///
    /// Kea returns an array of results, one per service. Non-zero result codes indicate errors.
    fn check_kea_response_errors(&self, result: &Value) -> Result<(), ControllerError> {
        if let Some(result_array) = result.as_array() {
            for item in result_array {
                if let Some(result_code) = item.get("result") {
                    if let Some(code) = result_code.as_u64() {
                        if code != 0 {
                            // Non-zero result code indicates error
                            let text = item.get("text")
                                .and_then(|t| t.as_str())
                                .unwrap_or("Unknown error");
                            return Err(ControllerError::KeaApi(format!("Kea command error (code {}): {}", code, text)));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

