//! Kea Commands - High-level command interface
//!
//! This module provides methods for all Kea Control Agent commands.
//! Commands are organized by category for better maintainability.

use crate::error::ControllerError;
use crate::kea::api::KeaApi;
use serde_json::{json, Value};
use tracing::info;

/// Kea Commands client for executing Kea commands
pub struct KeaCommands {
    api: KeaApi,
}

impl KeaCommands {
    /// Create a new Kea Commands client
    pub fn new(api: KeaApi) -> Self {
        Self { api }
    }

    // ============================================================================
    // Configuration Management Commands
    // ============================================================================

    /// Execute config-get command - Retrieve current Kea configuration
    pub async fn config_get(&self) -> Result<Value, ControllerError> {
        self.api.execute_command("config-get", vec!["dhcp4"], json!({})).await
    }

    /// Execute config-set command - Apply new Kea configuration
    pub async fn config_set(&self, config: &Value) -> Result<Value, ControllerError> {
        info!("Applying Kea configuration via Control Agent API");
        self.api.execute_command(
            "config-set",
            vec!["dhcp4"],
            json!({
                "Dhcp4": config
            }),
        )
        .await
    }

    /// Execute config-test command - Validate configuration without applying
    pub async fn config_test(&self, config: &Value) -> Result<Value, ControllerError> {
        self.api.execute_command(
            "config-test",
            vec!["dhcp4"],
            json!({
                "Dhcp4": config
            }),
        )
        .await
    }

    /// Execute config-reload command - Reload configuration from file
    pub async fn config_reload(&self) -> Result<Value, ControllerError> {
        info!("Reloading Kea configuration from file");
        self.api.execute_command("config-reload", vec!["dhcp4"], json!({})).await
    }

    /// Execute config-write command - Write current configuration to file
    pub async fn config_write(&self, filename: Option<&str>) -> Result<Value, ControllerError> {
        let args = if let Some(fname) = filename {
            json!({ "filename": fname })
        } else {
            json!({})
        };
        info!("Writing Kea configuration to file");
        self.api.execute_command("config-write", vec!["dhcp4"], args).await
    }

    /// Execute config-hash-get command - Get configuration hash
    pub async fn config_hash_get(&self) -> Result<Value, ControllerError> {
        self.api.execute_command("config-hash-get", vec!["dhcp4"], json!({})).await
    }

    /// Execute config-backend-pull command - Pull configuration from backend
    pub async fn config_backend_pull(&self) -> Result<Value, ControllerError> {
        info!("Pulling Kea configuration from backend");
        self.api.execute_command("config-backend-pull", vec!["dhcp4"], json!({})).await
    }

    // ============================================================================
    // Server Control Commands
    // ============================================================================

    /// Execute shutdown command - Gracefully shutdown Kea server
    pub async fn shutdown(&self, exit_value: Option<i32>) -> Result<Value, ControllerError> {
        let args = if let Some(exit) = exit_value {
            json!({ "exit-value": exit })
        } else {
            json!({})
        };
        info!("Shutting down Kea server");
        self.api.execute_command("shutdown", vec!["dhcp4"], args).await
    }

    /// Execute status-get command - Get server status
    pub async fn status_get(&self) -> Result<Value, ControllerError> {
        self.api.execute_command("status-get", vec!["dhcp4"], json!({})).await
    }

    /// Execute version-get command - Get server version
    pub async fn version_get(&self) -> Result<Value, ControllerError> {
        self.api.execute_command("version-get", vec!["dhcp4"], json!({})).await
    }

    /// Execute build-report command - Get build information
    pub async fn build_report(&self) -> Result<Value, ControllerError> {
        self.api.execute_command("build-report", vec!["dhcp4"], json!({})).await
    }

    /// Execute server-tag-get command - Get server tag
    pub async fn server_tag_get(&self) -> Result<Value, ControllerError> {
        self.api.execute_command("server-tag-get", vec!["dhcp4"], json!({})).await
    }

    // ============================================================================
    // DHCP Service Control Commands
    // ============================================================================

    /// Execute dhcp-enable command - Enable DHCP service
    pub async fn dhcp_enable(&self) -> Result<Value, ControllerError> {
        info!("Enabling DHCP service");
        self.api.execute_command("dhcp-enable", vec!["dhcp4"], json!({})).await
    }

    /// Execute dhcp-disable command - Disable DHCP service
    pub async fn dhcp_disable(&self) -> Result<Value, ControllerError> {
        info!("Disabling DHCP service");
        self.api.execute_command("dhcp-disable", vec!["dhcp4"], json!({})).await
    }

    // ============================================================================
    // Lease Management Commands (requires lease_cmds hook library)
    // ============================================================================

    /// Execute lease4-get command - Get IPv4 lease information
    pub async fn lease4_get(&self, ip_address: &str) -> Result<Value, ControllerError> {
        self.api.execute_command(
            "lease4-get",
            vec!["dhcp4"],
            json!({
                "ip-address": ip_address
            }),
        )
        .await
    }

    /// Execute lease4-get-all command - Get all IPv4 leases
    pub async fn lease4_get_all(&self, subnet_id: Option<u32>) -> Result<Value, ControllerError> {
        let args = if let Some(subnet) = subnet_id {
            json!({ "subnet-id": subnet })
        } else {
            json!({})
        };
        self.api.execute_command("lease4-get-all", vec!["dhcp4"], args).await
    }

    /// Execute lease4-add command - Add IPv4 lease
    pub async fn lease4_add(&self, lease: &Value) -> Result<Value, ControllerError> {
        info!("Adding IPv4 lease");
        self.api.execute_command("lease4-add", vec!["dhcp4"], lease.clone()).await
    }

    /// Execute lease4-del command - Delete IPv4 lease
    pub async fn lease4_del(&self, ip_address: &str) -> Result<Value, ControllerError> {
        info!("Deleting IPv4 lease: {}", ip_address);
        self.api.execute_command(
            "lease4-del",
            vec!["dhcp4"],
            json!({
                "ip-address": ip_address
            }),
        )
        .await
    }

    /// Execute lease4-wipe command - Wipe all IPv4 leases
    pub async fn lease4_wipe(&self, subnet_id: Option<u32>) -> Result<Value, ControllerError> {
        let args = if let Some(subnet) = subnet_id {
            json!({ "subnet-id": subnet })
        } else {
            json!({})
        };
        info!("Wiping IPv4 leases");
        self.api.execute_command("lease4-wipe", vec!["dhcp4"], args).await
    }

    /// Execute lease4-update command - Update IPv4 lease
    pub async fn lease4_update(&self, lease: &Value) -> Result<Value, ControllerError> {
        info!("Updating IPv4 lease");
        self.api.execute_command("lease4-update", vec!["dhcp4"], lease.clone()).await
    }

    // ============================================================================
    // Subnet Management Commands (requires subnet_cmds hook library)
    // ============================================================================

    /// Execute subnet4-add command - Add IPv4 subnet
    pub async fn subnet4_add(&self, subnet: &Value) -> Result<Value, ControllerError> {
        info!("Adding IPv4 subnet");
        self.api.execute_command(
            "subnet4-add",
            vec!["dhcp4"],
            json!({
                "subnet4": [subnet]
            }),
        )
        .await
    }

    /// Execute subnet4-del command - Delete IPv4 subnet
    pub async fn subnet4_del(&self, subnet_id: u32) -> Result<Value, ControllerError> {
        info!("Deleting IPv4 subnet: {}", subnet_id);
        self.api.execute_command(
            "subnet4-del",
            vec!["dhcp4"],
            json!({
                "id": subnet_id
            }),
        )
        .await
    }

    /// Execute subnet4-delta-add command - Add subnet delta (partial update)
    pub async fn subnet4_delta_add(&self, subnet: &Value) -> Result<Value, ControllerError> {
        info!("Adding IPv4 subnet delta");
        self.api.execute_command(
            "subnet4-delta-add",
            vec!["dhcp4"],
            json!({
                "subnet4": [subnet]
            }),
        )
        .await
    }

    /// Execute subnet4-delta-del command - Delete subnet delta (partial removal)
    pub async fn subnet4_delta_del(&self, subnet: &Value) -> Result<Value, ControllerError> {
        info!("Deleting IPv4 subnet delta");
        self.api.execute_command(
            "subnet4-delta-del",
            vec!["dhcp4"],
            json!({
                "subnet4": [subnet]
            }),
        )
        .await
    }

    // ============================================================================
    // Reservation Management Commands (requires host_cmds hook library)
    // ============================================================================

    /// Execute reservation-add command - Add host reservation
    pub async fn reservation_add(&self, reservation: &Value) -> Result<Value, ControllerError> {
        info!("Adding host reservation");
        self.api.execute_command("reservation-add", vec!["dhcp4"], reservation.clone()).await
    }

    /// Execute reservation-del command - Delete host reservation
    pub async fn reservation_del(&self, reservation: &Value) -> Result<Value, ControllerError> {
        info!("Deleting host reservation");
        self.api.execute_command("reservation-del", vec!["dhcp4"], reservation.clone()).await
    }

    /// Execute reservation-get command - Get host reservation
    pub async fn reservation_get(&self, reservation: &Value) -> Result<Value, ControllerError> {
        self.api.execute_command("reservation-get", vec!["dhcp4"], reservation.clone()).await
    }

    /// Execute reservation-list command - List all host reservations
    pub async fn reservation_list(&self, subnet_id: Option<u32>) -> Result<Value, ControllerError> {
        let args = if let Some(subnet) = subnet_id {
            json!({ "subnet-id": subnet })
        } else {
            json!({})
        };
        self.api.execute_command("reservation-list", vec!["dhcp4"], args).await
    }

    // ============================================================================
    // Statistics Commands
    // ============================================================================

    /// Execute statistic-get command - Get specific statistic
    pub async fn statistic_get(&self, statistic_name: &str) -> Result<Value, ControllerError> {
        self.api.execute_command(
            "statistic-get",
            vec!["dhcp4"],
            json!({
                "name": statistic_name
            }),
        )
        .await
    }

    /// Execute statistic-get-all command - Get all statistics
    pub async fn statistic_get_all(&self) -> Result<Value, ControllerError> {
        self.api.execute_command("statistic-get-all", vec!["dhcp4"], json!({})).await
    }

    /// Execute statistic-global-get-all command - Get all global statistics
    pub async fn statistic_global_get_all(&self) -> Result<Value, ControllerError> {
        self.api.execute_command("statistic-global-get-all", vec!["dhcp4"], json!({})).await
    }

    /// Execute statistic-reset command - Reset specific statistic
    pub async fn statistic_reset(&self, statistic_name: &str) -> Result<Value, ControllerError> {
        info!("Resetting statistic: {}", statistic_name);
        self.api.execute_command(
            "statistic-reset",
            vec!["dhcp4"],
            json!({
                "name": statistic_name
            }),
        )
        .await
    }

    /// Execute statistic-reset-all command - Reset all statistics
    pub async fn statistic_reset_all(&self) -> Result<Value, ControllerError> {
        info!("Resetting all statistics");
        self.api.execute_command("statistic-reset-all", vec!["dhcp4"], json!({})).await
    }

    /// Execute statistic-remove command - Remove specific statistic
    pub async fn statistic_remove(&self, statistic_name: &str) -> Result<Value, ControllerError> {
        info!("Removing statistic: {}", statistic_name);
        self.api.execute_command(
            "statistic-remove",
            vec!["dhcp4"],
            json!({
                "name": statistic_name
            }),
        )
        .await
    }

    /// Execute statistic-remove-all command - Remove all statistics
    pub async fn statistic_remove_all(&self) -> Result<Value, ControllerError> {
        info!("Removing all statistics");
        self.api.execute_command("statistic-remove-all", vec!["dhcp4"], json!({})).await
    }

    /// Execute statistic-sample-age-set command - Set statistic sample age
    pub async fn statistic_sample_age_set(&self, statistic_name: &str, age: u32) -> Result<Value, ControllerError> {
        self.api.execute_command(
            "statistic-sample-age-set",
            vec!["dhcp4"],
            json!({
                "name": statistic_name,
                "age": age
            }),
        )
        .await
    }

    /// Execute statistic-sample-count-set command - Set statistic sample count
    pub async fn statistic_sample_count_set(&self, statistic_name: &str, count: u32) -> Result<Value, ControllerError> {
        self.api.execute_command(
            "statistic-sample-count-set",
            vec!["dhcp4"],
            json!({
                "name": statistic_name,
                "count": count
            }),
        )
        .await
    }

    // ============================================================================
    // Utility Commands
    // ============================================================================

    /// Execute leases-reclaim command - Reclaim expired leases
    pub async fn leases_reclaim(&self) -> Result<Value, ControllerError> {
        info!("Reclaiming expired leases");
        self.api.execute_command("leases-reclaim", vec!["dhcp4"], json!({})).await
    }

    /// Execute subnet4-select-test command - Test subnet selection
    pub async fn subnet4_select_test(&self, query: &Value) -> Result<Value, ControllerError> {
        self.api.execute_command("subnet4-select-test", vec!["dhcp4"], query.clone()).await
    }

    /// Execute kea-lfc-start command - Start lease file cleanup
    pub async fn kea_lfc_start(&self) -> Result<Value, ControllerError> {
        info!("Starting lease file cleanup");
        self.api.execute_command("kea-lfc-start", vec!["dhcp4"], json!({})).await
    }
}
