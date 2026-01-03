//! Tenancy reconcilers
//! 
//! Handles: NetBoxTenant

use crate::reconciler::Reconciler;
use crate::error::ControllerError;
use crate::kube_api_trait::KubeApiTrait;
use crds::{NetBoxTenant, ResourceState};
use netbox_client::{TenantId, TenantGroupId};
use tracing::{info, error, debug, warn};

impl Reconciler {
    /// Reconciles a NetBoxTenant resource.
    pub async fn reconcile_netbox_tenant(&self, tenant_crd: &NetBoxTenant) -> Result<(), ControllerError> {
        // Helper function to update status with error
        async fn update_status_error(
            api: &dyn KubeApiTrait<NetBoxTenant>,
            name: &str,
            namespace: &str,
            error_msg: String,
            current_status: Option<&crds::NetBoxTenantStatus>,
        ) {
            if let Some(status) = current_status {
                if status.state == ResourceState::Failed && status.error.as_ref() == Some(&error_msg) {
                    debug!("NetBoxTenant {}/{} already has this error in status, skipping update", namespace, name);
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
            if let Err(e) = api.patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone())).await {
                error!("Failed to update NetBoxTenant {}/{} error status: {}", namespace, name, e);
            } else {
                info!("Updated NetBoxTenant {}/{} status with error", namespace, name);
            }
        }
        
        // Extract name and namespace using helper
        use crate::reconcile_helpers::extract_name_and_namespace;
        let (name, namespace) = extract_name_and_namespace(tenant_crd, "NetBoxTenant")?;
        
        info!("Reconciling NetBoxTenant {}/{}", namespace, name);
        
        // SPECIAL CASE: Tenant reconciler needs to use the token from the tenant's own secret
        // We can't use TokenResolver.resolve_token() here because it would create a circular dependency
        // (it would try to fetch the NetBoxTenant CRD, which requires the token we're trying to resolve)
        // Instead, we resolve the token directly from the tenant's token_secret in the spec
        let secret_ref = &tenant_crd.spec.token_secret;
        let secret_namespace = secret_ref.namespace.as_deref().unwrap_or(namespace);
        
        // Fetch Secret directly
        // Use SecretFetcher if available (for testing), otherwise use kube_client
        let secret = match (if let Some(secret_fetcher) = &self.secret_fetcher {
            secret_fetcher.get_secret(secret_namespace, &secret_ref.name).await
        } else {
            use kube::Api;
            let kube_client = self.token_resolver.kube_client().clone();
            let secret_api: Api<k8s_openapi::api::core::v1::Secret> =
                Api::namespaced(kube_client, secret_namespace);
            secret_api.get(&secret_ref.name).await
        }) {
            Ok(s) => s,
            Err(e) => {
                let error_msg = format!("Failed to fetch Secret {} in namespace {}: {}", secret_ref.name, secret_namespace, e);
                error!("{}", error_msg);
                update_status_error(&*self.netbox_tenant_api, name, namespace, error_msg.clone(), tenant_crd.status.as_ref()).await;
                // Emit event for token resolution failure
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::TOKEN_RESOLUTION_FAILED,
                    &error_msg,
                    tenant_crd,
                ).await;
                return Err(ControllerError::TokenResolution(crate::token_resolver::TokenResolutionError::SecretFetchError(
                    format!("{}: {}", secret_ref.name, e)
                )));
            }
        };
        
        // Extract token from Secret
        let token_key = secret_ref.key();
        let token_data = match secret
            .data
            .as_ref()
            .and_then(|data| data.get(token_key)) {
            Some(data) => data,
            None => {
                let error_msg = format!("Token key '{}' not found in Secret {}", token_key, secret_ref.name);
                error!("{}", error_msg);
                update_status_error(&*self.netbox_tenant_api, name, namespace, error_msg.clone(), tenant_crd.status.as_ref()).await;
                // Emit event for token resolution failure
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::TOKEN_RESOLUTION_FAILED,
                    &error_msg,
                    tenant_crd,
                ).await;
                return Err(ControllerError::TokenResolution(crate::token_resolver::TokenResolutionError::TokenKeyNotFound(
                    token_key.to_string()
                )));
            }
        };
        
        // Decode token (base64 encoded in Kubernetes Secrets)
        let token = match String::from_utf8(token_data.0.clone()) {
            Ok(t) => t,
            Err(e) => {
                let error_msg = format!("Failed to decode token from Secret {}: {}", secret_ref.name, e);
                error!("{}", error_msg);
                update_status_error(&*self.netbox_tenant_api, name, namespace, error_msg.clone(), tenant_crd.status.as_ref()).await;
                // Emit event for token resolution failure
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::TOKEN_RESOLUTION_FAILED,
                    &error_msg,
                    tenant_crd,
                ).await;
                return Err(ControllerError::TokenResolution(crate::token_resolver::TokenResolutionError::TokenDecodeError(
                    format!("{}: {}", secret_ref.name, e)
                )));
            }
        };
        
        // Trim whitespace (common issue with secrets)
        let token = token.trim().to_string();
        
        if token.is_empty() {
            let error_msg = format!("Token in Secret {} is empty", secret_ref.name);
            error!("{}", error_msg);
            update_status_error(&*self.netbox_tenant_api, name, namespace, error_msg.clone(), tenant_crd.status.as_ref()).await;
            // Emit event for token resolution failure
            use crate::events::reasons;
            self.record_event_warning(
                reasons::TOKEN_RESOLUTION_FAILED,
                &error_msg,
                tenant_crd,
            ).await;
            return Err(ControllerError::TokenResolution(
                crate::token_resolver::TokenResolutionError::TokenDecodeError(error_msg)
            ));
        }
        
        // Create client with the resolved token using TokenResolver
        // This allows MockTokenResolver to return a mock client in tests
        let netbox_client = match self.token_resolver.create_client_with_token(token) {
            Ok(client) => client,
            Err(e) => {
                let error_msg = format!("Failed to create NetBoxClient: {}", e);
                error!("{}", error_msg);
                update_status_error(&*self.netbox_tenant_api, name, namespace, error_msg.clone(), tenant_crd.status.as_ref()).await;
                // Emit event for token resolution failure
                use crate::events::reasons;
                self.record_event_warning(
                    reasons::TOKEN_RESOLUTION_FAILED,
                    &error_msg,
                    tenant_crd,
                ).await;
                return Err(ControllerError::TokenResolution(e));
            }
        };
        
        // Check if already created - use shared helper for drift detection and status validation
        use crate::reconcile_helpers::{validate_status_and_drift, DriftCheckResult};
        
        let drift_result = {
            let netbox_client_ref = netbox_client.as_ref();
            validate_status_and_drift(
                tenant_crd.status.as_ref(),
                "NetBoxTenant",
                namespace,
                name,
                |netbox_id: u64| async move {
                    netbox_client_ref.get_tenant(TenantId(netbox_id)).await
                },
            ).await?
        };
        
        let netbox_tenant = match drift_result {
            DriftCheckResult::UseExisting(tenant) => {
                // Resource exists and is up-to-date
                Some(tenant)
            }
            DriftCheckResult::StatusCleared { message } => {
                // Status was cleared - update it to Pending
                let status_patch = Self::create_resource_status_patch(
                    0, // Clear netbox_id
                    String::new(), // Clear URL
                    ResourceState::Pending,
                    Some(message),
                );
                let pp = kube::api::PatchParams::default();
                if let Err(update_err) = self.netbox_tenant_api
                    .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                    .await
                {
                    warn!("Failed to clear NetBoxTenant status: {}", update_err);
                }
                // Fall through to creation
                None
            }
            DriftCheckResult::Recreate => {
                // Need to create - fall through
                None
            }
        };
        
        // Handle existing tenant (from helper) or create new
        let netbox_tenant = match netbox_tenant {
            Some(tenant) => {
                // Resource exists - check for drift between spec and NetBox
                // Separate tag updates from other field updates for DRY code
                
                // Resolve tenant group ID from CRD spec for comparison
                let spec_group_id = if let Some(group_ref) = &tenant_crd.spec.group {
                    match netbox_client.as_ref().get_tenant_group_by_name(&group_ref.name).await {
                        Ok(Some(group)) => Some(group.id),
                        Ok(None) => {
                            warn!("Tenant group '{}' specified in CRD but not found in NetBox", group_ref.name);
                            None
                        }
                        Err(e) => {
                            warn!("Failed to resolve tenant group '{}' from CRD: {}", group_ref.name, e);
                            None
                        }
                    }
                } else {
                    None
                };
                
                // Get current group ID from NetBox tenant
                let netbox_group_id = tenant.group.as_ref().map(|g| g.id);
                
                // Use reusable helpers for DRY field comparison
                // See docs/implementationChecklists/MACRO_ANALYSIS.md for why we use helpers instead of macros
                use crate::reconcile_helpers::{
                    compare_string_field,
                    compare_slug_field,
                    compare_optional_string_field,
                    compare_optional_dependency_id,
                };
                
                let auto_generated_slug = tenant_crd.spec.name.to_lowercase().replace(' ', "-");
                let needs_update = 
                    compare_string_field(&tenant_crd.spec.name, &tenant.name)
                    || compare_slug_field(&tenant_crd.spec.slug, &tenant.slug, auto_generated_slug)
                    || compare_optional_string_field(&tenant_crd.spec.description, &tenant.description)
                    || compare_optional_string_field(&tenant_crd.spec.comments, &tenant.comments)
                    || compare_optional_dependency_id(spec_group_id, netbox_group_id);
                // Note: Tags are handled separately using update_tags_if_differ helper
                
                // Update other fields if they changed
                let mut tenant = if needs_update {
                    info!("Tenant {}/{} has drift, updating in NetBox", namespace, name);
                    // Update tenant in NetBox
                    let slug = tenant_crd.spec.slug.as_deref().map(|s| s.to_string())
                        .unwrap_or_else(|| tenant_crd.spec.name.to_lowercase().replace(' ', "-"));
                    
                    // Use already resolved group_id from above
                    let group_id = spec_group_id;
                    
                    match netbox_client.as_ref().update_tenant(
                        TenantId(tenant.id),
                        Some(&tenant_crd.spec.name),
                        Some(&slug),
                        tenant_crd.spec.description.clone(),
                        tenant_crd.spec.comments.clone(),
                        group_id.map(TenantGroupId),
                        None, // Tags handled separately
                    ).await {
                        Ok(updated) => {
                            info!("Updated tenant {} in NetBox (ID: {})", updated.name, updated.id);
                            // Emit event for successful update
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::UPDATED,
                                &format!("Updated tenant {} in NetBox (ID: {})", updated.name, updated.id),
                                tenant_crd,
                            ).await;
                            updated
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to update tenant in NetBox: {}", e);
                            error!("{}", error_msg);
                            update_status_error(&*self.netbox_tenant_api, name, namespace, error_msg.clone(), tenant_crd.status.as_ref()).await;
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                } else {
                    tenant
                };
                
                // Handle tag updates separately using DRY helper
                // Note: We use tags_differ directly instead of update_tags_if_differ because
                // netbox_client is Box<dyn NetBoxClientTrait> which can't be easily moved into a closure.
                // This is still DRY as we use the tags_differ helper.
                let tags_need_update = crate::reconcile_helpers::tags_differ(&tenant.tags, &tenant_crd.spec.tags);
                
                let tenant = if tags_need_update {
                    info!("Tenant {}/{} tags differ, updating in NetBox", namespace, name);
                    // Resolve tags
                    let resolved_tags_json = self.resolve_tag_references(
                        netbox_client.as_ref(),
                        &tenant_crd.spec.tags,
                        namespace,
                        name,
                    None,
                ).await;
                    let resolved_tags = crate::reconcile_helpers::convert_tags_to_strings(resolved_tags_json);
                    
                    // Prepare slug for update
                    let slug = tenant_crd.spec.slug.as_deref().map(|s| s.to_string())
                        .unwrap_or_else(|| tenant_crd.spec.name.to_lowercase().replace(' ', "-"));
                    
                    // Use already resolved group_id from above (no need to resolve again)
                    let group_id = spec_group_id;
                    
                    match netbox_client.as_ref().update_tenant(
                        TenantId(tenant.id),
                        Some(&tenant_crd.spec.name),
                        Some(&slug),
                        tenant_crd.spec.description.clone(),
                        tenant_crd.spec.comments.clone(),
                        group_id.map(TenantGroupId),
                        resolved_tags,
                    ).await {
                        Ok(updated) => {
                            info!("Updated tenant {} tags in NetBox (ID: {})", updated.name, updated.id);
                            updated
                        }
                        Err(e) => {
                            warn!("Failed to update tenant tags: {}", e);
                            // Continue with existing tenant - tag update failure is non-fatal
                            tenant
                        }
                    }
                } else {
                    debug!("Tenant {}/{} tags are up-to-date, skipping update", namespace, name);
                    tenant
                };
                
                // Check if tenant is up-to-date (after potential tag update)
                {
                    debug!("Tenant {}/{} is up-to-date (ID: {}), no changes needed", namespace, name, tenant.id);
                    // Update status if needed
                    use crate::reconcile_helpers::status_needs_update;
                    let needs_status_update = status_needs_update(
                        tenant_crd.status.as_ref(),
                        tenant.id,
                        &tenant.url,
                        "Created",
                        None,
                    );
                    
                    if needs_status_update {
                        let status_patch = Self::create_resource_status_patch(
                            tenant.id,
                            tenant.url.clone(),
                            ResourceState::Created,
                            None,
                        );
                        let pp = kube::api::PatchParams::default();
                        match self.netbox_tenant_api
                            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
                            .await
                        {
                            Ok(_) => {
                                debug!("Updated NetBoxTenant {}/{} status: NetBox ID {}", namespace, name, tenant.id);
                                return Ok(());
                            }
                            Err(e) => {
                                let error_msg = format!("Failed to update NetBoxTenant status: {}", e);
                                error!("{}", error_msg);
                                // Emit event for reconciliation failure
                                use crate::events::reasons;
                                self.record_event_warning(
                                    reasons::RECONCILIATION_FAILED,
                                    &error_msg,
                                    tenant_crd,
                                ).await;
                                return Err(ControllerError::Kube(e.into()));
                            }
                        }
                    } else {
                        debug!("NetBoxTenant {}/{} already has correct status (ID: {}), skipping update", namespace, name, tenant.id);
                        return Ok(());
                    }
                }
            }
            None => {
                // Need to create tenant - try to find existing by name (idempotency fallback)
                let existing_tenant = match netbox_client.as_ref().query_tenants(
                    &[("name", &tenant_crd.spec.name)],
                    false,
                ).await {
                    Ok(tenants) => tenants.first().cloned(),
                    Err(e) => {
                        warn!("Failed to query tenants: {}, will try to create", e);
                        None
                    }
                };
                
                // Resolve tenant group ID if group reference provided
                // If no group is specified and NetBox requires one, create a default group
                info!("Resolving tenant group for tenant {}/{}", namespace, name);
                // Validate tenant group kind using helper
                use crate::reconcile_helpers::validate_reference_kind;
                let group_id = if let Some(group_ref) = &tenant_crd.spec.group {
                    // Validate kind (NetBoxTenantGroup CRD not yet implemented, but we can validate)
                    if validate_reference_kind(group_ref, "NetBoxTenantGroup", "group", name).is_err() {
                        // Helper already logged the warning, just return None
                        None
                    } else {
                        info!("Tenant group specified in CRD: '{}'", group_ref.name);
                        match netbox_client.as_ref().get_tenant_group_by_name(&group_ref.name).await {
                            Ok(Some(group)) => {
                                info!("Resolved tenant group '{}' to ID {}", group_ref.name, group.id);
                                Some(group.id)
                            }
                            Ok(None) => {
                                warn!("Tenant group '{}' not found, will try to create default group", group_ref.name);
                                None
                            }
                            Err(e) => {
                                warn!("Failed to resolve tenant group '{}': {}, will try to create default group", group_ref.name, e);
                                None
                            }
                        }
                    }
                } else {
                    // No tenant group specified - this is valid (tenant groups are optional in NetBox)
                    // GitOps principle: Only create resources via CRDs, not by direct API calls
                    debug!("No tenant group specified in CRD for tenant {}/{}", namespace, name);
                    None
                };
                
                let netbox_tenant = if let Some(existing) = existing_tenant {
                    info!("Tenant {} already exists in NetBox (ID: {})", tenant_crd.spec.name, existing.id);
                    existing
                } else {
                    // Create tenant
                    debug!("Attempting to create tenant {} in NetBox", tenant_crd.spec.name);
                    let slug = tenant_crd.spec.slug.as_deref().map(|s| s.to_string())
                        .unwrap_or_else(|| tenant_crd.spec.name.to_lowercase().replace(' ', "-"));
                    match netbox_client.as_ref().create_tenant(
                        &tenant_crd.spec.name,
                        Some(&slug),
                        tenant_crd.spec.description.clone(),
                        tenant_crd.spec.comments.clone(),
                        group_id.map(TenantGroupId),
                        None, // tags - not yet implemented in reconciler
                    ).await {
                        Ok(created) => {
                            info!("Created tenant {} in NetBox (ID: {})", created.name, created.id);
                            // Emit event for successful creation
                            use crate::events::reasons;
                            self.record_event_normal(
                                reasons::CREATED,
                                &format!("Created tenant {} in NetBox (ID: {})", created.name, created.id),
                                tenant_crd,
                            ).await;
                            created
                        }
                        Err(e) => {
                            // Handle CREATE conflicts using shared helper (GitOps idempotency)
                            use crate::reconcile_helpers::is_conflict_error;
                            
                            if is_conflict_error(&e) {
                                warn!("Tenant {} creation failed with conflict, attempting to retrieve existing tenant (idempotency)", tenant_crd.spec.name);
                                
                                // Try multiple query strategies
                                let mut found_tenant = None;
                                let slug_fallback = tenant_crd
                                    .spec
                                    .slug
                                    .clone()
                                    .unwrap_or_else(|| tenant_crd.spec.name.to_lowercase().replace(' ', "-"));
                                
                                // Strategy 1: Query by name
                                match netbox_client.as_ref().query_tenants(
                                    &[("name", &tenant_crd.spec.name)],
                                    false,
                                ).await {
                                    Ok(tenants) => {
                                        if let Some(tenant) = tenants.first() {
                                            info!("Found existing tenant by name '{}' in NetBox (ID: {}) after conflict", tenant_crd.spec.name, tenant.id);
                                            found_tenant = Some(tenant.clone());
                                        }
                                    }
                                    Err(_) => {}
                                }
                                
                                // Strategy 2: Query by slug if not found
                                if found_tenant.is_none() {
                                    match netbox_client.as_ref().query_tenants(
                                        &[("slug", slug_fallback.as_str())],
                                        false,
                                    ).await {
                                        Ok(tenants) => {
                                            if let Some(tenant) = tenants.first() {
                                                info!("Found existing tenant by slug '{}' in NetBox (ID: {}) after conflict", slug_fallback, tenant.id);
                                                found_tenant = Some(tenant.clone());
                                            }
                                        }
                                        Err(_) => {}
                                    }
                                }
                                
                                // Strategy 3: Fallback - query all tenants and filter
                                if found_tenant.is_none() {
                                    match netbox_client.as_ref().query_tenants(&[], true).await {
                                        Ok(all_tenants) => {
                                            if let Some(tenant) = all_tenants.iter().find(|t| {
                                                t.name == tenant_crd.spec.name || t.slug == slug_fallback
                                            }) {
                                                info!("Found existing tenant in NetBox (ID: {}) via fallback query", tenant.id);
                                                found_tenant = Some(tenant.clone());
                                            }
                                        }
                                        Err(_) => {}
                                    }
                                }
                                
                                if let Some(found) = found_tenant {
                                    info!("Found existing tenant {} in NetBox (ID: {}) via conflict resolution (idempotency)", found.name, found.id);
                                    found
                                } else {
                                    let error_msg = format!("Tenant {} already exists in NetBox but could not retrieve it: {}", tenant_crd.spec.name, e);
                                    error!("{}", error_msg);
                                    update_status_error(&*self.netbox_tenant_api, name, namespace, error_msg.clone(), tenant_crd.status.as_ref()).await;
                                    return Err(ControllerError::NetBox(netbox_client::NetBoxError::Api(error_msg)));
                                }
                            } else {
                                // Not a conflict, return original error
                                let error_msg = format!("Failed to create tenant in NetBox: {}", e);
                                error!("{}", error_msg);
                                update_status_error(&*self.netbox_tenant_api, name, namespace, error_msg.clone(), tenant_crd.status.as_ref()).await;
                                return Err(ControllerError::NetBox(e));
                            }
                        }
                    }
                };
                
                netbox_tenant
            }
        };
        
        // Update status (use lowercase state to match CRD validation schema)
        let status_patch = Self::create_resource_status_patch(
            netbox_tenant.id,
            netbox_tenant.url.clone(),
            ResourceState::Created,
            None,
        );
        let pp = kube::api::PatchParams::default();
        match self.netbox_tenant_api
            .patch_status(name, &pp, &kube::api::Patch::Merge(status_patch.clone()))
            .await
        {
            Ok(_) => {
                info!("Updated NetBoxTenant {}/{} status: NetBox ID {}", namespace, name, netbox_tenant.id);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to update NetBoxTenant status: {}", e);
                error!("{}", error_msg);
                Err(ControllerError::Kube(e.into()))
            }
        }
    }
}
