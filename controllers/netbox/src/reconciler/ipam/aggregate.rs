//! NetBoxAggregate reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use tracing::{info, error, debug, warn};
use crds::{NetBoxAggregate, ResourceState};
use netbox_client::{NetBoxClientTrait, RirId};

impl Reconciler {
    pub async fn reconcile_netbox_aggregate(&self, aggregate_crd: &NetBoxAggregate) -> Result<(), ControllerError> {
        let name = aggregate_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxAggregate missing name".to_string()))?;
        let namespace = aggregate_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxAggregate {}/{}", namespace, name);
        
        // Get client for shared resource (falls back to main tenant)
        let netbox_client = self.token_resolver
            .create_client_for_shared_resource(namespace, "NetBoxAggregate", name)
            .await
            .map_err(|e| ControllerError::TokenResolution(e))?;
        
        // Resolve RIR ID (optional - if RIR is specified but doesn't exist, skip it)
        let rir_id = if let Some(rir_name) = &aggregate_crd.spec.rir {
            match netbox_client.get_rir_by_name(rir_name).await {
                Ok(Some(rir)) => {
                    debug!("Found RIR '{}' with ID {} for aggregate {}", rir_name, rir.id, name);
                    Some(rir.id)
                }
                Ok(None) => {
                    warn!("RIR '{}' not found in NetBox for aggregate {}, creating aggregate without RIR", rir_name, name);
                    None
                }
                Err(e) => {
                    warn!("Failed to get RIR '{}' for aggregate {}: {}, creating aggregate without RIR", rir_name, name, e);
                    None
                }
            }
        } else {
            None // RIR is optional
        };
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = &netbox_client;
            validate_status_and_drift(
                aggregate_crd.status.as_ref(),
                "NetBoxAggregate",
                namespace,
                name,
                |netbox_id| async move {
                    netbox_client_ref.query_aggregates(&[("id", &netbox_id.to_string())], false)
                        .await
                        .map(|mut aggregates| aggregates.pop().ok_or_else(|| netbox_client::NetBoxError::NotFound(format!("Aggregate {} not found", netbox_id))))
                        .and_then(|res| res)
                },
            ).await?
        };
        
        let netbox_aggregate = match drift_result {
            DriftCheckResult::UseExisting(aggregate) => Some(aggregate),
            DriftCheckResult::StatusCleared { message } => {
                let status_patch = Self::create_resource_status_patch(
                    0, String::new(), ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_aggregate_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxAggregate status: {}", update_err);
                }
                None
            }
            DriftCheckResult::Recreate => None,
        };
        
        let netbox_aggregate = match netbox_aggregate {
            Some(aggregate) => {
                use crate::reconcile_helpers::status_needs_update;
                let needs_status_update = status_needs_update(
                    aggregate_crd.status.as_ref(),
                    aggregate.id,
                    &aggregate.url,
                    "Created",
                    None,
                );
                
                if needs_status_update {
                    let status_patch = Self::create_resource_status_patch(
                        aggregate.id,
                        aggregate.url.clone(),
                        ResourceState::Created,
                        None,
                    );
                    let pp = kube::api::PatchParams::default();
                    match self.netbox_aggregate_api
                        .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxAggregate {}/{} status: NetBox ID {}", namespace, name, aggregate.id);
                            return Ok(());
                        }
                        Err(e) => {
                            error!("Failed to update NetBoxAggregate status: {}", e);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxAggregate {}/{} already has correct status (ID: {}), skipping update", namespace, name, aggregate.id);
                    return Ok(());
                }
            }
            None => {
                // Try to find existing aggregate by prefix
                let existing_aggregate = match netbox_client.query_aggregates(&[("prefix", &aggregate_crd.spec.prefix)], false).await {
                    Ok(mut aggregates) => {
                        aggregates.pop().map(|a| {
                            info!("Aggregate {} already exists in NetBox (ID: {}), acknowledging existence (idempotency)", a.prefix, a.id);
                            a
                        })
                    }
                    Err(e) => {
                        warn!("Failed to query aggregate: {}, will try to create", e);
                        None
                    }
                };
                
                if let Some(existing) = existing_aggregate {
                    existing
                } else {
                    info!("Creating NetBoxAggregate {} in NetBox", aggregate_crd.spec.prefix);
                    match netbox_client.create_aggregate(
                        &aggregate_crd.spec.prefix,
                        rir_id.map(RirId),
                        aggregate_crd.spec.date_allocated.as_deref(),
                        aggregate_crd.spec.description.clone(),
                        aggregate_crd.spec.comments.clone(),
                    ).await {
                        Ok(created_aggregate) => {
                            debug!("Successfully created NetBoxAggregate {} with ID {}", aggregate_crd.spec.prefix, created_aggregate.id);
                            created_aggregate
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create NetBoxAggregate in NetBox: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
            }
        };
        
        // Update CRD status
        let patch = Self::create_resource_status_patch(
            netbox_aggregate.id,
            netbox_aggregate.url.clone(),
            ResourceState::Created,
            None,
        );
        self.netbox_aggregate_api
            .patch_status(name, &kube::api::PatchParams::default(), &kube::api::Patch::Merge(patch))
            .await
            .map_err(|e| ControllerError::Kube(e.into()))?;
        
        info!("Successfully reconciled NetBoxAggregate {}/{}", namespace, name);
        Ok(())
    }
}
