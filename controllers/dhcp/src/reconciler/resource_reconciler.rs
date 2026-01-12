//! Resource Reconciler - Reconciles individual NetBox CRDs

use crate::error::ControllerError;
use crate::kea::KeaClient;
use crate::reconciler::DhcpReconciler;
use crds::{NetBoxPrefix, NetBoxIPRange, NetBoxIPAddress};
use crds::ipam::IPRangeStatus;
use std::sync::Arc;
use tracing::debug;

/// Reconciles individual NetBox CRDs
pub struct ResourceReconciler {
    kea_client: Arc<KeaClient>,
}

impl ResourceReconciler {
    /// Create a new Resource Reconciler
    pub fn new(kea_client: Arc<KeaClient>) -> Self {
        Self { kea_client }
    }

    /// Reconcile a single NetBoxPrefix CRD
    pub async fn reconcile_prefix(
        &self,
        _prefix: &NetBoxPrefix,
        reconciler: &DhcpReconciler,
    ) -> Result<(), ControllerError> {
        // Trigger full sync when a prefix changes
        // TODO: Optimize to only update the affected subnet
        if let Err(e) = reconciler.full_sync().await {
            use tracing::warn;
            warn!("Failed to sync prefix to Kea (Kea may not be available): {}", e);
            // Don't fail reconciliation - we'll retry on next change
        }
        Ok(())
    }

    /// Reconcile a single NetBoxIPRange CRD
    pub async fn reconcile_ip_range(
        &self,
        ip_range: &NetBoxIPRange,
        reconciler: &DhcpReconciler,
    ) -> Result<(), ControllerError> {
        // Only sync if status is Active (TODO: Add proper DHCP filtering via annotations/tags)
        if ip_range.spec.status != IPRangeStatus::Active {
            debug!("Skipping IP range {} - status is not 'Active'", 
                ip_range.metadata.name.as_deref().unwrap_or("unknown"));
            return Ok(());
        }

        // Trigger full sync when an IP range changes
        // TODO: Optimize to only update the affected pool
        if let Err(e) = reconciler.full_sync().await {
            use tracing::warn;
            warn!("Failed to sync IP range to Kea (Kea may not be available): {}", e);
            // Don't fail reconciliation - we'll retry on next change
        }
        Ok(())
    }

    /// Reconcile a single NetBoxIPAddress CRD
    pub async fn reconcile_ip_address(
        &self,
        ip_address: &NetBoxIPAddress,
        reconciler: &DhcpReconciler,
    ) -> Result<(), ControllerError> {
        // Only sync if status is "dhcp"
        if ip_address.spec.status != crds::IPAddressStatus::Dhcp {
            debug!("Skipping IP address {} - status is not 'dhcp'", 
                ip_address.metadata.name.as_deref().unwrap_or("unknown"));
            return Ok(());
        }

        // Trigger full sync when an IP address changes
        // TODO: Optimize to only update the affected reservation
        if let Err(e) = reconciler.full_sync().await {
            use tracing::warn;
            warn!("Failed to sync IP address to Kea (Kea may not be available): {}", e);
            // Don't fail reconciliation - we'll retry on next change
        }
        Ok(())
    }
}

