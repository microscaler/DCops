//! NetBoxAggregate reconciler

use super::super::Reconciler;
use crate::error::ControllerError;
use crate::reconcile_helpers;
use kube::Api;
use tracing::{info, error, debug, warn};
use crds::{NetBoxAggregate, NetBoxAggregateStatus, ResourceState};

impl Reconciler {
    pub async fn reconcile_netbox_aggregate(&self, aggregate_crd: &NetBoxAggregate) -> Result<(), ControllerError> {
        // Helper function to update status with error
        async fn update_status_error(
            api: &Api<NetBoxAggregate>,
            name: &str,
            namespace: &str,
            error_msg: String,
            current_status: Option<&NetBoxAggregateStatus>,
        ) {
            if let Some(status) = current_status {
                if status.state == ResourceState::Failed && status.error.as_ref() == Some(&error_msg) {
                    debug!("NetBoxAggregate {}/{} already has this error in status, skipping update", namespace, name);
                    return;
                }
            }
            
            // Update status with error (use lowercase state to match CRD validation schema)
            let status_patch = Reconciler::create_resource_status_patch(
                0, // No netbox_id on error
                String::new(), // No URL on error
                ResourceState::Failed,
                Some(error_msg.clone()),
            );
            let pp = kube::api::PatchParams::default();
            if let Err(e) = api.patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch)).await {
                error!("Failed to update NetBoxAggregate {}/{} error status: {}", namespace, name, e);
            } else {
                info!("Updated NetBoxAggregate {}/{} status with error", namespace, name);
            }
        }
        
        let name = aggregate_crd.metadata.name.as_ref()
            .ok_or_else(|| ControllerError::InvalidConfig("NetBoxAggregate missing name".to_string()))?;
        let namespace = aggregate_crd.metadata.namespace.as_deref()
            .unwrap_or("default");
        
        info!("Reconciling NetBoxAggregate {}/{}", namespace, name);
        
        // Check if already created - use helper for drift detection
        // Note: Aggregates don't have update logic yet, so we use the simple check_existing helper
        let netbox_aggregate = if let Some(status) = &aggregate_crd.status {
            if status.state == ResourceState::Created && status.netbox_id.is_some() {
                if let Some(netbox_id) = status.netbox_id {
                    // Use simple helper function for drift detection (no update logic)
                    match reconcile_helpers::check_existing(
                        &self.netbox_client,
                        netbox_id,
                        &format!("NetBoxAggregate {}/{}", namespace, name),
                        self.netbox_client.get_aggregate(netbox_id),
                    ).await {
                        Ok(Some(resource)) => {
                            // Resource exists and is up-to-date
                            Some(resource)
                        }
                        Ok(None) => {
                            // Drift detected - resource was deleted, clear status and recreate
                            warn!("NetBoxAggregate {}/{} was deleted in NetBox (ID: {}), clearing status and will recreate", namespace, name, netbox_id);
                            let status_patch = Self::create_resource_status_patch(
                                0, // Clear netbox_id
                                String::new(), // Clear URL
                                ResourceState::Pending,
                                Some("Resource was deleted in NetBox, will recreate".to_string()),
                            );
                            let pp = kube::api::PatchParams::default();
                            if let Err(e) = self.netbox_aggregate_api
                                .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                                .await
                            {
                                warn!("Failed to clear NetBoxAggregate status after drift detection: {}", e);
                            }
                            // Fall through to creation
                            None
                        }
                        Err(e) => {
                            // Error during drift detection - return to retry
                            return Err(e);
                        }
                    }
                } else {
                    None // No netbox_id, need to create
                }
            } else {
                None // Not in Created state, need to create
            }
        } else {
            None // No status, need to create
        };
        
        // Handle existing aggregate (from helper) or create new
        let netbox_aggregate = match netbox_aggregate {
            Some(aggregate) => {
                // Resource exists and is up-to-date - only update status if it changed
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
                        .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
                        .await
                    {
                        Ok(_) => {
                            debug!("Updated NetBoxAggregate {}/{} status: NetBox ID {}", namespace, name, aggregate.id);
                            return Ok(());
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update NetBoxAggregate status: {}", e);
                            error!("{}", error_msg);
                            return Err(ControllerError::Kube(e.into()));
                        }
                    }
                } else {
                    debug!("NetBoxAggregate {}/{} already has correct status (ID: {}), skipping update", namespace, name, aggregate.id);
                    return Ok(());
                }
            }
            None => {
                // Need to create aggregate - try to find existing by prefix (idempotency fallback)
                // Resolve RIR ID - RIR is required for aggregates
                info!("Resolving RIR for aggregate {}/{}", namespace, name);
        let rir_id = if let Some(rir_name) = &aggregate_crd.spec.rir {
            info!("RIR specified in CRD: '{}'", rir_name);
            match self.netbox_client.get_rir_by_name(rir_name).await {
                Ok(Some(rir)) => {
                    info!("Resolved RIR '{}' to ID {}", rir_name, rir.id);
                    Some(rir.id)
                }
                Ok(None) => {
                    // RIR specified but doesn't exist - create it
                    info!("RIR '{}' not found, creating it in NetBox", rir_name);
                    // Generate slug from name (lowercase, replace spaces with hyphens)
                    let slug = rir_name.to_lowercase().replace(' ', "-");
                    match self.netbox_client.create_rir(
                        rir_name,
                        Some(&slug),
                        Some(format!("RIR created by DCops for aggregate {}", aggregate_crd.spec.prefix)),
                        None, // Default is_private
                    ).await {
                        Ok(rir) => {
                            info!("Created RIR '{}' (ID: {})", rir.name, rir.id);
                            Some(rir.id)
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create RIR '{}': {}", rir_name, e);
                            error!("{}", error_msg);
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to query RIR '{}': {}, will try to create it", rir_name, e);
                    // Try to create the RIR even if query failed
                    let slug = rir_name.to_lowercase().replace(' ', "-");
                    match self.netbox_client.create_rir(
                        rir_name,
                        Some(&slug),
                        Some(format!("RIR created by DCops for aggregate {}", aggregate_crd.spec.prefix)),
                        None,
                    ).await {
                        Ok(rir) => {
                            info!("Created RIR '{}' (ID: {})", rir.name, rir.id);
                            Some(rir.id)
                        }
                        Err(create_err) => {
                            let error_msg = format!("Failed to resolve or create RIR '{}': {}", rir_name, create_err);
                            error!("{}", error_msg);
                            update_status_error(&self.netbox_aggregate_api, name, namespace, error_msg.clone(), aggregate_crd.status.as_ref()).await;
                            return Err(ControllerError::NetBox(create_err));
                        }
                    }
                }
            }
        } else {
            info!("No RIR specified in CRD, looking for default RIR");
            // Try to find a default RIR (common ones: ARIN, RIPE, APNIC, etc.)
            // For private networks, try to find a private RIR or use the first available
            let default_rirs = ["ARIN", "RIPE", "APNIC", "LACNIC", "AFRINIC"];
            let mut found_rir = None;
            for rir_name in &default_rirs {
                if let Ok(Some(rir)) = self.netbox_client.get_rir_by_name(rir_name).await {
                    info!("Using default RIR '{}' (ID: {}) for aggregate", rir_name, rir.id);
                    found_rir = Some(rir.id);
                    break;
                }
            }
            
            if found_rir.is_none() {
                // Try to get any RIR
                match self.netbox_client.query_rirs(&[], false).await {
                    Ok(rirs) if !rirs.is_empty() => {
                        let rir = &rirs[0];
                        info!("Using first available RIR '{}' (ID: {}) for aggregate", rir.name, rir.id);
                        found_rir = Some(rir.id);
                    }
                    _ => {
                        // Create a default RIR for private networks
                        info!("No RIRs found, creating default RIR 'Private' for private network aggregates");
                        match self.netbox_client.create_rir(
                            "Private",
                            Some("private"), // Slug is required
                            Some("Private network RIR for internal use".to_string()),
                            Some(true), // is_private = true
                        ).await {
                            Ok(rir) => {
                                info!("Created default RIR '{}' (ID: {})", rir.name, rir.id);
                                found_rir = Some(rir.id);
                            }
                            Err(e) => {
                                let error_msg = format!("No RIR specified and failed to create default RIR: {}. RIR is required for aggregates.", e);
                                error!("{}", error_msg);
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                }
            }
            
                found_rir
                };
                
                // Try to find existing aggregate by prefix
                let existing_aggregate = match self.netbox_client.query_aggregates(
                    &[("prefix", &aggregate_crd.spec.prefix)],
                    false,
                ).await {
                    Ok(aggregates) => aggregates.first().cloned(),
                    Err(_) => None
                };
                
                let netbox_aggregate = if let Some(existing) = existing_aggregate {
                    info!("Aggregate {} already exists in NetBox (ID: {})", aggregate_crd.spec.prefix, existing.id);
                    existing
                } else {
                    match self.netbox_client.create_aggregate(
                        &aggregate_crd.spec.prefix,
                        rir_id,
                        aggregate_crd.spec.date_allocated.as_deref(),
                        aggregate_crd.spec.description.clone(),
                        aggregate_crd.spec.comments.clone(),
                    ).await {
                        Ok(created) => {
                            info!("Created aggregate {} in NetBox (ID: {})", created.prefix, created.id);
                            created
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create aggregate in NetBox: {}", e);
                            error!("{}", error_msg);
                            update_status_error(&self.netbox_aggregate_api, name, namespace, error_msg.clone(), aggregate_crd.status.as_ref()).await;
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                };
                
                netbox_aggregate
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_aggregate.id,
            netbox_aggregate.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_aggregate_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(&status_patch))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxAggregate {}/{} status: NetBox ID {}", namespace, name, netbox_aggregate.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxAggregate status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
