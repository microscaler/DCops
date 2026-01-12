//! Config Comparator - Compares Kea configurations

use crate::error::ControllerError;
use serde_json::Value;

/// Compares Kea configurations
pub struct ConfigComparator;

impl ConfigComparator {
    /// Create a new Config Comparator
    pub fn new() -> Self {
        Self
    }

    /// Check if two Kea configurations differ
    ///
    /// # Note
    /// This is a simplified comparison using JSON string comparison.
    /// In production, this should do a deep comparison that ignores:
    /// - Order of subnets/pools/reservations
    /// - Whitespace differences
    /// - Timestamps or other non-semantic fields
    pub fn configs_differ(&self, current: &Value, desired: &Value) -> Result<bool, ControllerError> {
        // Simplified comparison - in production, should do deep comparison
        // For now, just compare JSON strings
        let current_str = serde_json::to_string(current)?;
        let desired_str = serde_json::to_string(desired)?;
        Ok(current_str != desired_str)
    }
}

impl Default for ConfigComparator {
    fn default() -> Self {
        Self::new()
    }
}

