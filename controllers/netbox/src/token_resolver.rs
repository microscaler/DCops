//! Token Resolver Service
//!
//! Single point of dependency injection for NetBox API tokens.
//! Resolves tenant-specific tokens from Kubernetes Secrets referenced by Tenant CRDs.

use crds::{NetBoxResourceReference, NetBoxTenant, NetBoxDevice, NetBoxSite};
use kube::{Api, Client};
use netbox_client::{NetBoxClient, NetBoxError};
use tracing::{debug, error, warn, info};

/// Error types for token resolution
#[derive(Debug, thiserror::Error)]
pub enum TokenResolutionError {
    #[error("Tenant CRD not found: {0}")]
    TenantNotFound(String),
    
    #[error("Secret not found: {0}")]
    SecretNotFound(String),
    
    #[error("Token key '{0}' not found in Secret")]
    TokenKeyNotFound(String),
    
    #[error("Failed to fetch Tenant CRD: {0}")]
    TenantFetchError(String),
    
    #[error("Failed to fetch Secret: {0}")]
    SecretFetchError(String),
    
    #[error("Failed to decode token from Secret: {0}")]
    TokenDecodeError(String),
    
    #[error("Failed to create NetBoxClient: {0}")]
    ClientCreation(NetBoxError),
    
    #[error("No referencing resource found for shared resource '{0}' in namespace '{1}' to determine tenant")]
    NoReferencingResourceFound(String, String),
}


/// Token Resolver Service
///
/// This is the SINGLE POINT of dependency injection for NetBox API tokens.
/// All token resolution flows through this service.
pub struct TokenResolver {
    kube_client: Client,
    pub(crate) netbox_url: String,
}

impl TokenResolver {
    /// Create a new TokenResolver
    pub fn new(kube_client: Client, netbox_url: String) -> Self {
        Self {
            kube_client,
            netbox_url,
        }
    }
    
    /// Resolve token for a tenant reference
    ///
    /// This is the SINGLE POINT of token resolution in the codebase.
    /// All token resolution flows through this method.
    ///
    /// # Arguments
    /// * `namespace` - Namespace where the resource (and potentially the Tenant CRD) exists
    /// * `tenant_ref` - Reference to the NetBoxTenant CRD
    ///
    /// # Returns
    /// The NetBox API token as a String
    pub async fn resolve_token(
        &self,
        namespace: &str,
        tenant_ref: &NetBoxResourceReference,
    ) -> Result<String, TokenResolutionError> {
        debug!(
            "Resolving token for tenant: {} in namespace: {}",
            tenant_ref.name, namespace
        );
        
        // Step 1: Determine Tenant CRD namespace
        let tenant_namespace = tenant_ref.namespace.as_deref().unwrap_or(namespace);
        
        // Step 2: Fetch Tenant CRD
        let tenant_api: Api<NetBoxTenant> = Api::namespaced(self.kube_client.clone(), tenant_namespace);
        let tenant = tenant_api
            .get(&tenant_ref.name)
            .await
            .map_err(|e| {
                error!("Failed to fetch Tenant CRD {}: {}", tenant_ref.name, e);
                TokenResolutionError::TenantFetchError(format!("{}: {}", tenant_ref.name, e))
            })?;
        
        // Step 3: Extract secret reference from Tenant CRD
        let secret_ref = &tenant.spec.token_secret;
        debug!(
            "Tenant {} references Secret: {}",
            tenant_ref.name, secret_ref.name
        );
        
        // Step 4: Determine Secret namespace
        let secret_namespace = secret_ref.namespace.as_deref().unwrap_or(tenant_namespace);
        
        // Step 5: Fetch Secret
        let secret_api: Api<k8s_openapi::api::core::v1::Secret> =
            Api::namespaced(self.kube_client.clone(), secret_namespace);
        let secret = secret_api
            .get(&secret_ref.name)
            .await
            .map_err(|e| {
                error!("Failed to fetch Secret {}: {}", secret_ref.name, e);
                TokenResolutionError::SecretFetchError(format!("{}: {}", secret_ref.name, e))
            })?;
        
        // Step 6: Extract token from Secret
        let token_key = secret_ref.key();
        let token_data = secret
            .data
            .as_ref()
            .and_then(|data| data.get(token_key))
            .ok_or_else(|| {
                error!("Token key '{}' not found in Secret {}", token_key, secret_ref.name);
                TokenResolutionError::TokenKeyNotFound(token_key.to_string())
            })?;
        
        // Step 7: Decode token (base64 encoded in Kubernetes Secrets)
        let token = String::from_utf8(token_data.0.clone())
            .map_err(|e| {
                error!("Failed to decode token from Secret {}: {}", secret_ref.name, e);
                TokenResolutionError::TokenDecodeError(format!("{}: {}", secret_ref.name, e))
            })?;
        
        // Trim whitespace (common issue with secrets)
        let token = token.trim().to_string();
        
        if token.is_empty() {
            return Err(TokenResolutionError::TokenDecodeError(
                format!("Token in Secret {} is empty", secret_ref.name)
            ));
        }
        
        debug!("Successfully resolved token for tenant: {}", tenant_ref.name);
        Ok(token)
    }
    
    /// Create a NetBoxClient with resolved token for a tenant
    ///
    /// This is the SINGLE POINT of NetBoxClient creation with tenant tokens.
    /// All tenant-specific client creation flows through this method.
    ///
    /// # Arguments
    /// * `namespace` - Namespace where the resource exists
    /// * `tenant_ref` - Reference to the NetBoxTenant CRD
    ///
    /// # Returns
    /// A NetBoxClient instance configured with the tenant's token
    pub async fn create_client_for_tenant(
        &self,
        namespace: &str,
        tenant_ref: &NetBoxResourceReference,
    ) -> Result<NetBoxClient, TokenResolutionError> {
        let token = self.resolve_token(namespace, tenant_ref).await?;
        
        NetBoxClient::new(self.netbox_url.clone(), token)
            .map_err(|e| {
                error!("Failed to create NetBoxClient: {}", e);
                TokenResolutionError::ClientCreation(e)
            })
    }
    
    /// Get the main tenant reference (datacenter-tenant)
    ///
    /// This is used as a fallback for shared resources when no referencing resource is found.
    fn get_main_tenant_reference(&self) -> NetBoxResourceReference {
        NetBoxResourceReference {
            api_group: "dcops.microscaler.io".to_string(),
            kind: "NetBoxTenant".to_string(),
            name: "datacenter-tenant".to_string(),
            namespace: None,
        }
    }

    /// Resolve tenant for a shared resource by finding a referencing resource
    ///
    /// This implements Strategy 1 from SHARED_RESOURCE_TENANT_RESOLUTION.md:
    /// Find a resource that references the shared resource and use that resource's tenant.
    /// Falls back to the main tenant (datacenter-tenant) if no referencing resource is found.
    ///
    /// # Arguments
    /// * `namespace` - Namespace where the shared resource exists
    /// * `resource_kind` - Kind of the shared resource (e.g., "NetBoxManufacturer")
    /// * `resource_name` - Name of the shared resource CRD
    ///
    /// # Returns
    /// A `NetBoxResourceReference` pointing to the tenant to use
    pub async fn resolve_tenant_for_shared_resource(
        &self,
        namespace: &str,
        resource_kind: &str,
        resource_name: &str,
    ) -> Result<NetBoxResourceReference, TokenResolutionError> {
        debug!(
            "Resolving tenant for shared resource: {} {}/{}",
            resource_kind, namespace, resource_name
        );

        let result = match resource_kind {
            "NetBoxManufacturer" | "NetBoxDeviceType" | "NetBoxPlatform" | "NetBoxDeviceRole" => {
                // Find a Device that uses this resource
                self.find_tenant_from_referencing_devices(namespace, resource_kind, resource_name).await
            }
            "NetBoxRegion" | "NetBoxSiteGroup" => {
                // Find a Site that uses this resource
                self.find_tenant_from_referencing_sites(namespace, resource_kind, resource_name).await
            }
            _ => {
                // Unknown shared resource kind - fall back to main tenant
                warn!("Unknown shared resource kind: {}. Falling back to main tenant (datacenter-tenant).", resource_kind);
                Ok(self.get_main_tenant_reference())
            }
        };

        // If no referencing resource found, fall back to main tenant
        match result {
            Ok(tenant_ref) => Ok(tenant_ref),
            Err(TokenResolutionError::NoReferencingResourceFound(_, _)) => {
                warn!(
                    "No referencing resource found for {} {}/{}. Falling back to main tenant (datacenter-tenant).",
                    resource_kind, namespace, resource_name
                );
                Ok(self.get_main_tenant_reference())
            }
            Err(e) => Err(e),
        }
    }
    
    /// Find tenant from Devices that reference this shared resource
    async fn find_tenant_from_referencing_devices(
        &self,
        namespace: &str,
        resource_kind: &str,
        resource_name: &str,
    ) -> Result<NetBoxResourceReference, TokenResolutionError> {
        let device_api: Api<NetBoxDevice> = Api::namespaced(self.kube_client.clone(), namespace);
        
        // List all devices in the namespace
        let devices = device_api.list(&Default::default()).await
            .map_err(|e| {
                error!("Failed to list devices in namespace {}: {}", namespace, e);
                TokenResolutionError::TenantFetchError(format!("Failed to list devices: {}", e))
            })?;
        
        // Find a device that references this shared resource
        for device in devices.items {
            let device_spec = &device.spec;
            
            // Check if device references this resource
            let matches = match resource_kind {
                "NetBoxManufacturer" => {
                    // Manufacturer is referenced via DeviceType
                    // We'd need to check device_type -> manufacturer, but that requires another lookup
                    // For now, just use the device's tenant
                    false // Will be handled by device_type check below
                }
                "NetBoxDeviceType" => {
                    device_spec.device_type.name == resource_name
                }
                "NetBoxPlatform" => {
                    device_spec.platform.as_ref()
                        .map(|p| p.name == resource_name)
                        .unwrap_or(false)
                }
                "NetBoxDeviceRole" => {
                    device_spec.device_role.name == resource_name
                }
                _ => false,
            };
            
            if matches {
                info!(
                    "Found Device {}/{} that references {} {}, using device's tenant",
                    namespace, device.metadata.name.as_deref().unwrap_or("<unnamed>"),
                    resource_kind, resource_name
                );
                return Ok(device_spec.tenant.clone());
            }
        }
        
        // For Manufacturer, try to find via DeviceType
        if resource_kind == "NetBoxManufacturer" {
            // This is more complex - we'd need to:
            // 1. List all DeviceTypes
            // 2. Find one that references this Manufacturer
            // 3. Find a Device that uses that DeviceType
            // For now, return error to trigger fallback to main tenant
        } else {
            // Return error to trigger fallback to main tenant
        }
        
        Err(TokenResolutionError::NoReferencingResourceFound(
            resource_name.to_string(),
            namespace.to_string(),
        ))
    }
    
    /// Find tenant from Sites that reference this shared resource
    async fn find_tenant_from_referencing_sites(
        &self,
        namespace: &str,
        resource_kind: &str,
        resource_name: &str,
    ) -> Result<NetBoxResourceReference, TokenResolutionError> {
        let site_api: Api<NetBoxSite> = Api::namespaced(self.kube_client.clone(), namespace);
        
        // List all sites in the namespace
        let sites = site_api.list(&Default::default()).await
            .map_err(|e| {
                error!("Failed to list sites in namespace {}: {}", namespace, e);
                TokenResolutionError::TenantFetchError(format!("Failed to list sites: {}", e))
            })?;
        
        // Find a site that references this shared resource
        for site in sites.items {
            let site_spec = &site.spec;
            
            // Check if site references this resource
            let matches = match resource_kind {
                "NetBoxRegion" => {
                    site_spec.region.as_ref()
                        .map(|r| r.name == resource_name)
                        .unwrap_or(false)
                }
                "NetBoxSiteGroup" => {
                    site_spec.site_group.as_ref()
                        .map(|sg| sg.name == resource_name)
                        .unwrap_or(false)
                }
                _ => false,
            };
            
            if matches {
                info!(
                    "Found Site {}/{} that references {} {}, using site's tenant",
                    namespace, site.metadata.name.as_deref().unwrap_or("<unnamed>"),
                    resource_kind, resource_name
                );
                return Ok(site_spec.tenant.clone());
            }
        }
        
        // Return error to trigger fallback to main tenant
        Err(TokenResolutionError::NoReferencingResourceFound(
            resource_name.to_string(),
            namespace.to_string(),
        ))
    }
    
    /// Create a NetBoxClient for a shared resource
    ///
    /// This resolves the tenant for a shared resource and creates a client with that tenant's token.
    ///
    /// # Arguments
    /// * `namespace` - Namespace where the shared resource exists
    /// * `resource_kind` - Kind of the shared resource (e.g., "NetBoxManufacturer")
    /// * `resource_name` - Name of the shared resource CRD
    ///
    /// # Returns
    /// A NetBoxClient instance configured with the resolved tenant's token
    pub async fn create_client_for_shared_resource(
        &self,
        namespace: &str,
        resource_kind: &str,
        resource_name: &str,
    ) -> Result<NetBoxClient, TokenResolutionError> {
        let tenant_ref = self.resolve_tenant_for_shared_resource(namespace, resource_kind, resource_name).await?;
        self.create_client_for_tenant(namespace, &tenant_ref).await
    }
}

