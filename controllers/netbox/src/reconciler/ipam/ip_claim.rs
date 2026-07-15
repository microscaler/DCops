//! IPClaim reconciler
//!
//! Manages the lifecycle of IPClaim CRDs - allocates IP addresses from
//! an IPPool's child prefix in NetBox and optionally assigns them to devices.

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::kube_api_trait::KubeApiTrait;
use tracing::{info, error, debug, warn};
use crds::{IPClaim, IPClaimStatus, IPClaimState};

impl Reconciler {
    /// Resolve the IPPool CRD referenced by the IPClaim to get its netbox_id
    async fn resolve_pool_id(
        &self,
        claim: &IPClaim,
        resource_name: &str,
    ) -> Result<u64, ControllerError> {
        use crate::reconcile_helpers::{validate_reference_kind, resolve_required_dependency_id};

        validate_reference_kind(&claim.spec.pool, "IPPool", "pool", resource_name)?;

        resolve_required_dependency_id(
            &*self.netbox_ip_pool_api,
            &claim.spec.pool.name,
            "IPPool",
            resource_name,
            |crd| crd.status.as_ref(),
        )
        .await
    }

    pub async fn reconcile_ip_claim(&self, claim_crd: &IPClaim) -> Result<(), ControllerError> {
        use crate::reconcile_helpers::extract_name_and_namespace;

        let (name, namespace) = extract_name_and_namespace(claim_crd, "IPClaim")?;
        info!("Reconciling IPClaim {}/{}", namespace, name);

        // Resolve the referenced IPPool to get its netbox_id
        let pool_id = match self.resolve_pool_id(claim_crd, name).await {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to resolve pool for IPClaim {}/{}: {}", namespace, name, e);
                let status_patch = Self::create_ip_claim_status_patch(
                    0,
                    String::new(),
                    None,
                    IPClaimState::Failed,
                    Some(format!("{}", e)),
                );
                let pp = kube::api::PatchParams::default();
                let _ = self.netbox_ip_claim_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch))
                    .await;
                return Err(e);
            }
        };

        info!("Resolved pool {} for IPClaim {}/{}", pool_id, namespace, name);

        // Check if already allocated
        match &claim_crd.status {
            Some(status) => {
                if status.state == IPClaimState::Failed {
                    if let Some(error) = &status.error {
                        if error.contains("Invalid token") || error.contains("403 Forbidden") {
                            debug!("IPClaim {}/{} already marked as failed with auth error, skipping", namespace, name);
                            return Ok(());
                        }
                    }
                }

                if status.state == IPClaimState::Created && status.netbox_id.is_some() {
                    info!("IPClaim {}/{} IP already allocated (ID: {})", namespace, name, status.netbox_id.unwrap());
                    return Ok(());
                }

                if status.state == IPClaimState::Failed && status.netbox_id.is_some() {
                    info!("IPClaim {}/{} has Failed status, checking for existing IP", namespace, name);
                    if status.netbox_id.is_some() {
                        return Ok(());
                    }
                }
            },
            None => {}
        }

        error!("IPClaim {}/{} IP not yet allocated", namespace, name);
        let status_patch = Self::create_ip_claim_status_patch(
            0,
            String::new(),
            None,
            IPClaimState::Pending,
            None,
        );
        let pp = kube::api::PatchParams::default();
        let _ = self.netbox_ip_claim_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch))
            .await;
        Err(ControllerError::InvalidConfig(format!(
            "IPClaim {}/{} IP allocation not yet implemented", namespace, name
        )))
    }
}
