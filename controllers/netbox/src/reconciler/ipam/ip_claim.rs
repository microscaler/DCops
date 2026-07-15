//! IPClaim reconciler
//!
//! Manages the lifecycle of IPClaim CRDs - allocates IP addresses from
//! an IPPool's child prefix in NetBox and optionally assigns them to devices.

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::kube_api_trait::KubeApiTrait;
use crds::{IPClaim, IPClaimState};
use std::str::FromStr;
use tracing::{debug, error, info, warn};

impl Reconciler {
    /// Resolve the IPPool referenced by the IPClaim to get its child prefix ID in NetBox
    async fn resolve_pool_prefix_id(
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

    /// Resolve the parent prefix CRD from the IPPool spec to extract tenant reference
    async fn resolve_pool_tenant(
        &self,
        pool: &crds::IPPool,
        claim_namespace: &str,
    ) -> Result<crds::NetBoxResourceReference, ControllerError> {
        let parent_crd = self
            .netbox_prefix_api
            .get(&pool.spec.prefix.name)
            .await
            .map_err(|e| {
                ControllerError::InvalidConfig(format!(
                    "Parent NetBoxPrefix CR '{}' not found: {}",
                    pool.spec.prefix.name, e
                ))
            })?;

        Ok(parent_crd.spec.tenant.clone())
    }

    /// Update the IPClaim status with the allocated IP info
    async fn update_ip_claim_status(
        &self,
        name: &str,
        namespace: &str,
        netbox_id: u64,
        netbox_url: String,
        ip: Option<String>,
        state: IPClaimState,
        error: Option<String>,
    ) -> Result<(), ControllerError> {
        let ip_display = ip.clone();
        let status_patch =
            Self::create_ip_claim_status_patch(netbox_id, netbox_url, ip, state, error);
        let pp = kube::api::PatchParams::default();
        self.netbox_ip_claim_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch))
            .await
            .map_err(ControllerError::Kube)?;
        debug!(
            "Updated IPClaim {}/{} status: NetBox ID {}, IP: {:?}",
            namespace, name, netbox_id, ip_display
        );
        Ok(())
    }

    /// Set failed status on the IPClaim and return error
    async fn set_failed_claim_status(
        &self,
        name: &str,
        namespace: &str,
        error: String,
    ) -> Result<(), ControllerError> {
        let status_patch = Self::create_ip_claim_status_patch(
            0,
            String::new(),
            None,
            IPClaimState::Failed,
            Some(error.clone()),
        );
        let pp = kube::api::PatchParams::default();
        if let Err(e) = self
            .netbox_ip_claim_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch))
            .await
        {
            error!(
                "Failed to update IPClaim {}/{} error status: {}",
                namespace, name, e
            );
        }
        Err(ControllerError::InvalidConfig(error))
    }

    /// Reclaim (delete) the allocated IP from NetBox
    /// Called when the IPClaim CR is being deleted
    async fn reclaim_ip(
        &self,
        claim: &IPClaim,
        resource_name: &str,
        namespace: &str,
    ) -> Result<(), ControllerError> {
        let ip_id = match claim.status.as_ref() {
            Some(status) => status.netbox_id.ok_or_else(|| {
                ControllerError::InvalidConfig(format!(
                    "IPClaim {}/{} has no allocated IP to reclaim",
                    namespace, resource_name
                ))
            }),
            None => Err(ControllerError::InvalidConfig(format!(
                "IPClaim {}/{} has no status to reclaim from",
                namespace, resource_name
            ))),
        }?;

        info!("Reclaiming IP (ID: {}) from NetBox for IPClaim {}/{}", ip_id, namespace, resource_name);

        // Use the main NetBox client for cleanup (IP addresses are in the default tenant context)
        match self
            .token_resolver
            .create_client_for_shared_resource(namespace, "IPClaim", resource_name)
            .await
        {
            Ok(client) => {
                match client
                    .delete_ip_address(netbox_client::IpAddressId(ip_id))
                    .await
                {
                    Ok(_) => {
                        info!(
                            "Successfully reclaimed IP {} for IPClaim {}/{}",
                            ip_id, namespace, resource_name
                        );
                        Ok(())
                    }
                    Err(e) => {
                        warn!(
                            "Failed to reclaim IP {} for IPClaim {}/{}: {}",
                            ip_id, namespace, resource_name, e
                        );
                        // Not critical — the IP may have already been deleted
                        Ok(())
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Could not create NetBox client for IPClaim {}/{} reclamation: {}",
                    namespace, resource_name, e
                );
                Ok(())
            }
        }
    }

    pub async fn reconcile_ip_claim(&self, claim_crd: &IPClaim) -> Result<(), ControllerError> {
        use crate::reconcile_helpers::extract_name_and_namespace;

        let (name, namespace) = extract_name_and_namespace(claim_crd, "IPClaim")?;
        info!("Reconciling IPClaim {}/{}", namespace, name);

        // Check if the claim is being deleted (has a deletion timestamp)
        if claim_crd.metadata.deletion_timestamp.is_some() {
            info!(
                "IPClaim {}/{} is being deleted, reclaiming IP",
                namespace, name
            );
            self.reclaim_ip(claim_crd, &name, &namespace).await?;
            return Ok(());
        }

        // 1. Resolve the IPPool to get its child prefix ID in NetBox
        let pool_prefix_id = match self.resolve_pool_prefix_id(claim_crd, &name).await {
            Ok(id) => id,
            Err(e) => {
                error!(
                    "Failed to resolve pool for IPClaim {}/{}: {}",
                    namespace, name, e
                );
                let status_patch = Self::create_ip_claim_status_patch(
                    0,
                    String::new(),
                    None,
                    IPClaimState::Failed,
                    Some(format!("{}", e)),
                );
                let pp = kube::api::PatchParams::default();
                let _ = self
                    .netbox_ip_claim_api
                    .patch_status(
                        &name,
                        &pp,
                        &kube::api::Patch::Merge(status_patch),
                    )
                    .await;
                return Err(e);
            }
        };

        info!("Resolved pool prefix ID: {}", pool_prefix_id);

        // 2. Get the IPPool CRD to find the parent prefix reference
        let pool_crd = match self
            .netbox_ip_pool_api
            .get(&claim_crd.spec.pool.name)
            .await
        {
            Ok(pool) => pool,
            Err(e) => {
                let error_msg =
                    format!("IPPool CR '{}' not found: {}", claim_crd.spec.pool.name, e);
                error!("{}", error_msg);
                return self
                    .set_failed_claim_status(&name, &namespace, error_msg)
                    .await;
            }
        };

        // 3. Resolve the parent prefix to get tenant info
        let parent_tenant_ref =
            match self.resolve_pool_tenant(&pool_crd, &namespace).await {
                Ok(t) => t,
                Err(e) => {
                    return self
                        .set_failed_claim_status(
                            &name,
                            &namespace,
                            format!("Failed to resolve pool tenant: {}", e),
                        )
                        .await;
                }
            };

        let tenant_namespace = parent_tenant_ref
            .namespace
            .as_deref()
            .unwrap_or(&namespace);

        info!(
            "Resolved pool tenant: {}/{}",
            tenant_namespace, parent_tenant_ref.name
        );

        // 4. Get the tenant-specific NetBox client
        let netbox_client = match self
            .token_resolver
            .create_client_for_tenant(tenant_namespace, &parent_tenant_ref)
            .await
        {
            Ok(c) => c,
            Err(e) => {
                return self
                    .set_failed_claim_status(
                        &name,
                        &namespace,
                        format!("Failed to create NetBox client: {}", e),
                    )
                    .await;
            }
        };

        // 5. Check if already allocated — skip if status is Created with netbox_id
        if let Some(status) = &claim_crd.status {
            if status.state == IPClaimState::Created && status.netbox_id.is_some() {
                info!(
                    "IPClaim {}/{} IP already allocated (ID: {}, IP: {:?}), skipping",
                    namespace,
                    name,
                    status.netbox_id.unwrap(),
                    status.ip
                );
                return Ok(());
            }

            if status.state == IPClaimState::Failed {
                // Failed claims need manual intervention — log and skip
                debug!(
                    "IPClaim {}/{} is in Failed state, skipping reconciliation",
                    namespace, name
                );
                return Ok(());
            }
        }

        // 6. Build the allocate IP request
        let preferred_address = claim_crd.spec.preferred_ip.as_ref().and_then(|ip| {
            ipnet::IpNet::from_str(ip).map_err(|e| {
                error!(
                    "Invalid preferred IP '{}' in IPClaim {}/{}: {}",
                    ip, namespace, name, e
                );
            })
            .ok()
        });

        // Determine effective status. IPs inside IP ranges get forced to 'reserved' by NetBox.
        // The reconciler will handle this automatically via the IP address reconciler.
        // For IPClaim, we always request 'active' and let the allocator handle the actual status.
        let status = Some(netbox_client::IPAddressStatus::Active);

        let allocate_request = netbox_client::AllocateIPRequest {
            address: preferred_address,
            description: claim_crd.spec.description.clone(),
            comments: None,
            status,
            role: None,
            dns_name: None,
            tenant: None, // Inherited from parent prefix
            tags: None,
            assigned_object_type: None,
            assigned_object_id: None,
        };

        // 7. Allocate the IP from the pool's child prefix
        info!(
            "Allocating IP from pool prefix {} for IPClaim {}/{}",
            pool_prefix_id, namespace, name
        );

        match netbox_client
            .allocate_ip(
                netbox_client::PrefixId(pool_prefix_id),
                Some(allocate_request),
            )
            .await
        {
            Ok(ip) => {
                info!(
                    "Allocated IP {} in NetBox (ID: {}, URL: {})",
                    ip.address, ip.id, ip.url
                );

                // Emit success event
                use crate::events::reasons;
                self.record_event_normal(
                    reasons::CREATED,
                    &format!(
                        "Allocated IP {} from pool (NetBox ID: {})",
                        ip.address, ip.id
                    ),
                    claim_crd,
                )
                .await;

                // Update status with the allocated IP info
                self.update_ip_claim_status(
                    &name,
                    &namespace,
                    ip.id,
                    ip.url,
                    Some(ip.address.to_string()),
                    IPClaimState::Created,
                    None,
                )
                .await?;

                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to allocate IP from pool: {}", e);
                error!("{}", error_msg);

                // Emit failure event
                use crate::events::reasons;
                self.record_event_warning(reasons::RECONCILIATION_FAILED, &error_msg, claim_crd)
                    .await;

                self.set_failed_claim_status(&name, &namespace, error_msg)
                    .await
            }
        }
    }
}
