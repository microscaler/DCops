//! Shared types and constants for DHCP Controller

/// Default requeue duration for reconciliation
pub const DEFAULT_REQUEUE_DURATION_SECS: u64 = 10;

/// Kea Control Agent default port
pub const KEA_CONTROL_AGENT_DEFAULT_PORT: u16 = 8000;

/// Kea Control Agent default URL
pub const KEA_CONTROL_AGENT_DEFAULT_URL: &str = "http://localhost:8000";

/// Kea API timeout in seconds
pub const KEA_API_TIMEOUT_SECS: u64 = 30;

