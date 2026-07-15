//! IPPool reconciler
//!
//! Creates child prefixes in NetBox under a parent prefix and manages
//! allocation strategies (sequential/random) for CIDR selection.

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::kube_api_trait::KubeApiTrait;
use crate::reconcile_helpers::{
    extract_name_and_namespace, resolve_optional_dependency_id,
    resolve_required_dependency_id, validate_reference_kind,
};
use crds::{IPPool, IPPoolState, IPPoolStatus};
use ipnet::IpNet;
use netbox_client::{NetBoxClientTrait, PrefixId};
use std::str::FromStr;
use tracing::{debug, error, info, warn};

impl Reconciler {
    /// Resolve a NetBoxPrefix reference to get the NetBox prefix ID
    async fn resolve_parent_prefix_id(
        &self,
        prefix_ref: &crds::NetBoxResourceReference,
        pool_name: &str,
    ) -> Result<u64, ControllerError> {
        validate_reference_kind(prefix_ref, "NetBoxPrefix", "prefix", pool_name)?;
        resolve_required_dependency_id(
            &*self.netbox_prefix_api,
            &prefix_ref.name,
            "NetBoxPrefix",
            pool_name,
            |crd| crd.status.as_ref(),
        )
        .await
    }

    /// Resolve a NetBoxRole reference to get the NetBox role ID (optional)
    async fn resolve_role_id(
        &self,
        role_ref: Option<&crds::NetBoxResourceReference>,
        pool_name: &str,
    ) -> Option<u64> {
        resolve_optional_dependency_id(
            &*self.netbox_role_api,
            role_ref,
            "NetBoxRole",
            "role",
            pool_name,
            |crd| crd.status.as_ref(),
        )
        .await
    }

    /// Resolve the parent prefix CRD and extract the tenant reference from it.
    async fn resolve_parent_tenant(
        &self,
        prefix_ref: &crds::NetBoxResourceReference,
        pool_name: &str,
        pool_namespace: &str,
    ) -> Result<crds::NetBoxResourceReference, ControllerError> {
        let tenant_ref = prefix_ref.namespace.as_deref().unwrap_or(pool_namespace);
        let parent_crd = self
            .netbox_prefix_api
            .get(&prefix_ref.name)
            .await
            .map_err(|e| {
                ControllerError::InvalidConfig(format!(
                    "Parent NetBoxPrefix CR '{}' not found: {}",
                    prefix_ref.name, e
                ))
            })?;

        // The parent prefix CRD has a .spec.tenant field that tells us which tenant to use
        Ok(parent_crd.spec.tenant.clone())
    }

    /// Compute the child prefix CIDR based on allocation strategy.
    /// Uses the parent prefix length + offset to compute the child CIDR.
    fn compute_child_prefix_cidr(
        parent_prefix: &IpNet,
        child_prefix_len: u8,
        strategy: &crds::AllocationStrategy,
    ) -> Result<IpNet, ControllerError> {
        match strategy {
            crds::AllocationStrategy::Sequential => {
                // Sequential: just use the child prefix (first child in range)
                let child = parent_prefix
                    .supernet()
                    .ok_or_else(|| {
                        ControllerError::InvalidIPFormat(format!(
                            "Cannot compute child prefix /{} from {:?}",
                            child_prefix_len, parent_prefix
                        ))
                    })?;
                Ok(child)
            }
            crds::AllocationStrategy::Random => {
                // Random: for now, also use the first child prefix.
                // A full random strategy would require iterating sub-prefixes
                // and tracking which ones are allocated.
                let child = parent_prefix
                    .supernet()
                    .ok_or_else(|| {
                        ControllerError::InvalidIPFormat(format!(
                            "Cannot compute child prefix /{} from {:?}",
                            child_prefix_len, parent_prefix
                        ))
                    })?;
                Ok(child)
            }
        }
    }

    /// Find the best child prefix length for the pool.
    /// Uses parent prefix length + 8 (e.g. /24 -> /32, /16 -> /24).
    fn compute_child_prefix_length(parent_prefix: &IpNet) -> u8 {
        let max = if parent_prefix.addr().is_ipv4() {
            32u8
        } else {
            128u8
        };
        let child_len = parent_prefix.prefix_len() + 8;
        if child_len < parent_prefix.prefix_len() + 2 {
            parent_prefix.prefix_len() + 2
        } else if child_len > max {
            max - 1
        } else {
            child_len
        }
    }

    /// Update the IPPool status with the child prefix info.
    async fn update_ip_pool_status(
        &self,
        name: &str,
        namespace: &str,
        netbox_id: u64,
        netbox_url: String,
        state: IPPoolState,
        error: Option<String>,
    ) -> Result<(), ControllerError> {
        let status_patch = Self::create_ip_pool_status_patch(netbox_id, netbox_url, state, error);
        let pp = kube::api::PatchParams::default();
        self.netbox_ip_pool_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch))
            .await
            .map_err(ControllerError::Kube)?;
        debug!("Updated IPPool {}/{} status: NetBox ID {}", namespace, name, netbox_id);
        Ok(())
    }

    /// Set failed status on the IPPool.
    async fn set_failed_status(
        &self,
        name: &str,
        namespace: &str,
        error: String,
    ) -> Result<(), ControllerError> {
        let status_patch = Self::create_ip_pool_status_patch(
            0,
            String::new(),
            IPPoolState::Failed,
            Some(error.clone()),
        );
        let pp = kube::api::PatchParams::default();
        if let Err(e) = self
            .netbox_ip_pool_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch))
            .await
        {
            error!(
                "Failed to update IPPool {}/{} error status: {}",
                namespace, name, e
            );
        }
        Err(ControllerError::InvalidConfig(error))
    }

    pub async fn reconcile_ip_pool(&self, ip_pool_crd: &IPPool) -> Result<(), ControllerError> {
        let (name, namespace) = extract_name_and_namespace(ip_pool_crd, "IPPool")?;

        info!("Reconciling IPPool {}/{}", namespace, name);

        // 1. Resolve the parent prefix reference to get the NetBox prefix ID
        let parent_netbox_id =
            self.resolve_parent_prefix_id(&ip_pool_crd.spec.prefix, name).await?;
        info!("Resolved parent prefix ID: {}", parent_netbox_id);

        // 2. Resolve the parent prefix CRD to find the tenant reference
        let parent_tenant_ref = self
            .resolve_parent_tenant(&ip_pool_crd.spec.prefix, name, namespace)
            .await?;
        info!(
            "Resolved parent prefix tenant: {}/{}",
            parent_tenant_ref.namespace.as_deref().unwrap_or(namespace),
            parent_tenant_ref.name
        );

        // 3. Get the tenant-specific NetBox client
        let netbox_client = self
            .token_resolver
            .create_client_for_tenant(
                parent_tenant_ref.namespace.as_deref().unwrap_or(namespace),
                &parent_tenant_ref,
            )
            .await?;

        // 4. Get the parent prefix from NetBox to determine child CIDR
        let parent_prefix = match netbox_client
            .get_prefix(PrefixId(parent_netbox_id))
            .await
        {
            Ok(prefix) => {
                info!(
                    "Retrieved parent prefix {} in NetBox (ID: {})",
                    prefix.prefix, parent_netbox_id
                );
                prefix
            }
            Err(e) => {
                warn!(
                    "Parent prefix {} not found in NetBox: {}",
                    parent_netbox_id, e
                );
                return self
                    .set_failed_status(name, namespace, format!("Parent prefix not found in NetBox: {}", e))
                    .await;
            }
        };

        // 5. Compute child prefix CIDR
        let child_prefix_len = Self::compute_child_prefix_length(&parent_prefix.prefix);
        let child_prefix = Self::compute_child_prefix_cidr(
            &parent_prefix.prefix,
            child_prefix_len,
            &ip_pool_crd.spec.allocation_strategy,
        )?;
        info!(
            "Will create child prefix {} (from parent {}, strategy: {:?})",
            child_prefix,
            parent_prefix.prefix,
            ip_pool_crd.spec.allocation_strategy
        );

        // 6. Resolve optional role reference
        let role_id = self
            .resolve_role_id(ip_pool_crd.spec.role.as_ref(), name)
            .await;
        if let Some(rid) = role_id {
            info!("Resolved role ID for child prefix: {}", rid);
        }

        // 7. Check if child prefix already exists in status
        let existing_child_id = if let Some(status) = &ip_pool_crd.status {
            status.netbox_id.or(Some(0))
        } else {
            Some(0)
        }
        .unwrap_or(0);

        if existing_child_id > 0 {
            debug!(
                "IPPool {}/{} already has child prefix ID {}, skipping creation",
                namespace, name, existing_child_id
            );
            return Ok(());
        }

        // 8. Create child prefix in NetBox
        info!(
            "Creating child prefix {} under parent prefix {}",
            child_prefix, parent_prefix.prefix
        );
        match netbox_client
            .create_prefix(
                &child_prefix,
                ip_pool_crd.spec.description.clone(),
                None, // site_id - child prefixes typically don't have site
                None, // vlan_id
                Some("active"),
                role_id.map(netbox_client::RoleId),
                None, // tenant_id - inherits from parent prefix
                None, // tags
            )
            .await
        {
            Ok(created) => {
                info!(
                    "Created child prefix {} in NetBox (ID: {}, URL: {})",
                    child_prefix, created.id, created.url
                );

                // Emit success event
                use crate::events::reasons;
                self.record_event_normal(
                    reasons::CREATED,
                    &format!(
                        "Created child prefix {} in NetBox (ID: {})",
                        child_prefix, created.id
                    ),
                    ip_pool_crd,
                )
                .await;

                // Update status
                self.update_ip_pool_status(
                    name,
                    namespace,
                    created.id,
                    created.url,
                    IPPoolState::Created,
                    None,
                )
                .await?;

                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to create child prefix in NetBox: {}", e);
                error!("{}", error_msg);

                // Emit failure event
                use crate::events::reasons;
                self.record_event_warning(reasons::RECONCILIATION_FAILED, &error_msg, ip_pool_crd)
                    .await;

                self.set_failed_status(name, namespace, error_msg).await
            }
        }
    }
}
