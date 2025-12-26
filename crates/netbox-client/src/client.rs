//! NetBox API client
//!
//! Implements the NetBox REST API client for IPAM operations.
//! Based on NetBox API structure: /api/ipam/prefixes/ and /api/ipam/ip-addresses/

use crate::error::NetBoxError;
use crate::models::*;
use crate::netbox_trait::NetBoxClientTrait;
use reqwest::Client;
use std::time::Duration;
use tracing::debug;

/// NetBox API client
pub struct NetBoxClient {
    client: Client,
    base_url: String,
    token: String,
}

impl NetBoxClient {
    /// Create a new NetBox client
    ///
    /// # Arguments
    /// * `base_url` - NetBox base URL (e.g., "http://netbox:80")
    /// * `token` - API token for authentication
    pub fn new(base_url: String, token: String) -> Result<Self, NetBoxError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| NetBoxError::Http(e))?;
        
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        })
    }
    
    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
    
    /// Validate the API token by making a simple authenticated request.
    ///
    /// This method tests connectivity and token validity before proceeding with operations.
    /// It makes a lightweight request to the NetBox status endpoint.
    ///
    /// # Returns
    /// * `Ok(())` - Token is valid and NetBox is reachable
    /// * `Err(NetBoxError)` - Token is invalid or NetBox is unreachable
    pub async fn validate_token(&self) -> Result<(), NetBoxError> {
        // Use the status endpoint as it's lightweight and requires authentication
        let url = format!("{}/api/status/", self.base_url);
        debug!("Validating NetBox token and connectivity");
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        
        if status == 401 || status == 403 {
            return Err(NetBoxError::Api(format!(
                "Invalid token: {} - {}",
                status,
                body
            )));
        }
        
        if !status.is_success() {
            return Err(NetBoxError::Api(format!(
                "Failed to validate token: {} - {}",
                status, body
            )));
        }
        
        debug!("Token validated successfully");
        Ok(())
    }
    
    /// Fetch all pages of a paginated response
    async fn fetch_all_pages<T: for<'de> serde::Deserialize<'de>>(
        &self,
        mut url: String,
    ) -> Result<Vec<T>, NetBoxError> {
        let mut all_results = Vec::new();
        
        loop {
            debug!("Fetching page: {}", url);
            
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to fetch page: {} - {}",
                    status, body
                )));
            }
            
            // Try to deserialize, but capture the response body for better error messages
            let response_text = response.text().await?;
            let page: PaginatedResponse<T> = serde_json::from_str(&response_text).map_err(|e| {
                NetBoxError::Api(format!(
                    "error decoding response body: {} - Response (first 500 chars): {}",
                    e,
                    response_text.chars().take(500).collect::<String>()
                ))
            })?;
            all_results.extend(page.results);
            
            // Check if there's a next page
            match page.next {
                Some(next_url) => {
                    // Extract the path from the full URL
                    url = if next_url.starts_with("http") {
                        next_url
                    } else {
                        format!("{}{}", self.base_url, next_url)
                    };
                }
                None => break,
            }
        }
        
        Ok(all_results)
    }
    
    /// Get a prefix by ID
    ///
    /// # Arguments
    /// * `id` - Prefix ID
    ///
    /// # Returns
    /// * `Ok(Prefix)` - The prefix object
    /// * `Err(NetBoxError)` - If the request fails
    pub async fn get_prefix(&self, id: u64) -> Result<Prefix, NetBoxError> {
        let url = format!("{}/api/ipam/prefixes/{}/", self.base_url, id);
        debug!("Fetching prefix {} from NetBox", id);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if response.status() == 404 {
            return Err(NetBoxError::NotFound(format!("Prefix {} not found", id)));
        }
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to get prefix {}: {} - {}",
                id, status, body
            )));
        }
        
        let prefix: Prefix = response.json().await?;
        Ok(prefix)
    }
    
    /// Get available IP addresses from a prefix
    ///
    /// # Arguments
    /// * `prefix_id` - Prefix ID
    /// * `limit` - Optional limit on number of IPs to return
    ///
    /// # Returns
    /// * `Ok(Vec<AvailableIP>)` - List of available IP addresses
    /// * `Err(NetBoxError)` - If the request fails
    pub async fn get_available_ips(&self, prefix_id: u64, limit: Option<u32>) -> Result<Vec<AvailableIP>, NetBoxError> {
        let mut url = format!("{}/api/ipam/prefixes/{}/available-ips/", self.base_url, prefix_id);
        if let Some(limit) = limit {
            url = format!("{}?limit={}", url, limit);
        }
        
        debug!("Fetching available IPs from prefix {}", prefix_id);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to get available IPs from prefix {}: {} - {}",
                prefix_id, status, body
            )));
        }
        
        let ips: Vec<AvailableIP> = response.json().await
            .map_err(|e| NetBoxError::Http(e))?;
        Ok(ips)
    }
    
    /// Allocate an IP address from a prefix
    ///
    /// This method:
    /// 1. Gets available IPs from the prefix
    /// 2. Takes the first available IP
    /// 3. Creates an IPAddress object in NetBox
    /// 4. Returns the allocated IP
    ///
    /// # Arguments
    /// * `prefix_id` - Prefix ID to allocate from
    /// * `request` - Optional allocation request (description, status, etc.)
    ///
    /// # Returns
    /// * `Ok(IPAddress)` - The allocated IP address
    /// * `Err(NetBoxError)` - If allocation fails
    pub async fn allocate_ip(&self, prefix_id: u64, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        // Get available IPs
        let available_ips = self.get_available_ips(prefix_id, Some(1)).await?;
        
        if available_ips.is_empty() {
            return Err(NetBoxError::Api(format!(
                "No available IPs in prefix {}",
                prefix_id
            )));
        }
        
        let available_ip = &available_ips[0];
        
        // Build request body
        let mut body = serde_json::json!({
            "address": available_ip.address.clone(),
        });
        
        if let Some(req) = request {
            if let Some(desc) = req.description {
                body["description"] = serde_json::Value::String(desc);
            }
            if let Some(status) = req.status {
                body["status"] = serde_json::to_value(status)
                    .map_err(|e| NetBoxError::Serialization(e))?;
            }
            if let Some(role) = req.role {
                body["role"] = serde_json::Value::String(role);
            }
            if let Some(dns_name) = req.dns_name {
                body["dns_name"] = serde_json::Value::String(dns_name);
            }
            if let Some(tags) = req.tags {
                body["tags"] = serde_json::to_value(tags)
                    .map_err(|e| NetBoxError::Serialization(e))?;
            }
        }
        
        // Create IP address via POST to available-ips endpoint
        let url = format!("{}/api/ipam/prefixes/{}/available-ips/", self.base_url, prefix_id);
        debug!("Allocating IP {} from prefix {}", available_ip.address, prefix_id);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to allocate IP from prefix {}: {} - {}",
                prefix_id, status, body
            )));
        }
        
        // NetBox returns an array of created IP addresses
        let created_ips: Vec<IPAddress> = response.json().await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if created_ips.is_empty() {
            return Err(NetBoxError::Api("No IP address was created".to_string()));
        }
        
        Ok(created_ips[0].clone())
    }
    
    /// Get an IP address by ID
    ///
    /// # Arguments
    /// * `id` - IP Address ID
    ///
    /// # Returns
    /// * `Ok(IPAddress)` - The IP address object
    /// * `Err(NetBoxError)` - If the request fails
    pub async fn get_ip_address(&self, id: u64) -> Result<IPAddress, NetBoxError> {
        let url = format!("{}/api/ipam/ip-addresses/{}/", self.base_url, id);
        debug!("Fetching IP address {} from NetBox", id);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if response.status() == 404 {
            return Err(NetBoxError::NotFound(format!("IP address {} not found", id)));
        }
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to get IP address {}: {} - {}",
                id, status, body
            )));
        }
        
        let ip: IPAddress = response.json().await
            .map_err(|e| NetBoxError::Http(e))?;
        Ok(ip)
    }
    
    /// Query IP addresses by filter
    ///
    /// # Arguments
    /// * `filters` - Query parameters (e.g., [("address", "192.168.1.1/24")])
    /// * `fetch_all` - If true, fetch all pages (default: false, returns first page only)
    ///
    /// # Returns
    /// * `Ok(Vec<IPAddress>)` - List of matching IP addresses
    /// * `Err(NetBoxError)` - If the request fails
    pub async fn query_ip_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<IPAddress>, NetBoxError> {
        let mut url = format!("{}/api/ipam/ip-addresses/", self.base_url);
        
        // Build query string
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying IP addresses with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query IP addresses: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<IPAddress> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get device by MAC address
    ///
    /// Queries interfaces for a matching MAC address and returns the device.
    ///
    /// # Arguments
    /// * `mac` - MAC address to search for
    ///
    /// # Returns
    /// * `Ok(Some(Device))` - The device if found
    /// * `Ok(None)` - If no device found
    /// * `Err(NetBoxError)` - If the request fails
    pub async fn get_device_by_mac(&self, mac: &str) -> Result<Option<Device>, NetBoxError> {
        debug!("Querying device by MAC address: {}", mac);
        
        let interfaces = self.query_interfaces(&[("mac_address", mac)], false).await?;
        
        if interfaces.is_empty() {
            return Ok(None);
        }
        
        // Get the device from the first matching interface
        let interface = &interfaces[0];
        let device_id = interface.device.id;
        
        // Fetch the device
        let device_url = format!("{}/api/dcim/devices/{}/", self.base_url, device_id);
        let device_response = self.client
            .get(&device_url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !device_response.status().is_success() {
            let status = device_response.status();
            let body = device_response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to get device {}: {} - {}",
                device_id, status, body
            )));
        }
        
        let device: Device = device_response.json().await
            .map_err(|e| NetBoxError::Http(e))?;
        Ok(Some(device))
    }
    
    /// Query prefixes by filters
    ///
    /// # Arguments
    /// * `filters` - Query parameters (e.g., [("prefix", "192.168.1.0/24"), ("status", "active")])
    /// * `fetch_all` - If true, fetch all pages (default: false, returns first page only)
    ///
    /// # Returns
    /// * `Ok(Vec<Prefix>)` - List of matching prefixes
    /// * `Err(NetBoxError)` - If the request fails
    pub async fn query_prefixes(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Prefix>, NetBoxError> {
        let mut url = format!("{}/api/ipam/prefixes/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying prefixes with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query prefixes: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<Prefix> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Create a new IP address
    ///
    /// # Arguments
    /// * `address` - IP address with CIDR (e.g., "192.168.1.1/24")
    /// * `request` - Optional allocation request (description, status, etc.)
    ///
    /// # Returns
    /// * `Ok(IPAddress)` - The created IP address
    /// * `Err(NetBoxError)` - If creation fails
    pub async fn create_ip_address(&self, address: &str, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        let mut body = serde_json::json!({
            "address": address,
        });
        
        if let Some(req) = request {
            if let Some(desc) = req.description {
                body["description"] = serde_json::Value::String(desc);
            }
            if let Some(status) = req.status {
                body["status"] = serde_json::to_value(status)
                    .map_err(|e| NetBoxError::Serialization(e))?;
            }
            if let Some(role) = req.role {
                body["role"] = serde_json::Value::String(role);
            }
            if let Some(dns_name) = req.dns_name {
                body["dns_name"] = serde_json::Value::String(dns_name);
            }
            if let Some(tags) = req.tags {
                body["tags"] = serde_json::to_value(tags)
                    .map_err(|e| NetBoxError::Serialization(e))?;
            }
        }
        
        let url = format!("{}/api/ipam/ip-addresses/", self.base_url);
        debug!("Creating IP address: {}", address);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create IP address: {} - {}",
                status, body
            )));
        }
        
        let ip: IPAddress = response.json().await
            .map_err(|e| NetBoxError::Http(e))?;
        Ok(ip)
    }
    
    /// Update an existing IP address
    ///
    /// # Arguments
    /// * `id` - IP Address ID
    /// * `request` - Update request with fields to change
    ///
    /// # Returns
    /// * `Ok(IPAddress)` - The updated IP address
    /// * `Err(NetBoxError)` - If update fails
    pub async fn update_ip_address(&self, id: u64, request: AllocateIPRequest) -> Result<IPAddress, NetBoxError> {
        let mut body = serde_json::json!({});
        
        if let Some(desc) = request.description {
            body["description"] = serde_json::Value::String(desc);
        }
        if let Some(status) = request.status {
            body["status"] = serde_json::to_value(status)
                .map_err(|e| NetBoxError::Serialization(e))?;
        }
        if let Some(role) = request.role {
            body["role"] = serde_json::Value::String(role);
        }
        if let Some(dns_name) = request.dns_name {
            body["dns_name"] = serde_json::Value::String(dns_name);
        }
        if let Some(tags) = request.tags {
            body["tags"] = serde_json::to_value(tags)
                .map_err(|e| NetBoxError::Serialization(e))?;
        }
        
        let url = format!("{}/api/ipam/ip-addresses/{}/", self.base_url, id);
        debug!("Updating IP address: {}", id);
        
        let response = self.client
            .patch(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to update IP address {}: {} - {}",
                id, status, body
            )));
        }
        
        let ip: IPAddress = response.json().await
            .map_err(|e| NetBoxError::Http(e))?;
        Ok(ip)
    }
    
    /// Delete an IP address
    ///
    /// # Arguments
    /// * `id` - IP Address ID
    ///
    /// # Returns
    /// * `Ok(())` - If deletion succeeds
    /// * `Err(NetBoxError)` - If deletion fails
    pub async fn delete_ip_address(&self, id: u64) -> Result<(), NetBoxError> {
        let url = format!("{}/api/ipam/ip-addresses/{}/", self.base_url, id);
        debug!("Deleting IP address: {}", id);
        
        let response = self.client
            .delete(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if response.status() == 404 {
            return Err(NetBoxError::NotFound(format!("IP address {} not found", id)));
        }
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to delete IP address {}: {} - {}",
                id, status, body
            )));
        }
        
        Ok(())
    }
    
    /// Query devices by filters
    ///
    /// # Arguments
    /// * `filters` - Query parameters (e.g., [("name", "router-01"), ("site", "datacenter-1")])
    /// * `fetch_all` - If true, fetch all pages (default: false, returns first page only)
    ///
    /// # Returns
    /// * `Ok(Vec<Device>)` - List of matching devices
    /// * `Err(NetBoxError)` - If the request fails
    pub async fn query_devices(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Device>, NetBoxError> {
        let mut url = format!("{}/api/dcim/devices/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying devices with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query devices: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<Device> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get a device by ID
    ///
    /// # Arguments
    /// * `id` - Device ID
    ///
    /// # Returns
    /// * `Ok(Device)` - The device object
    /// * `Err(NetBoxError)` - If the request fails
    pub async fn get_device(&self, id: u64) -> Result<Device, NetBoxError> {
        let url = format!("{}/api/dcim/devices/{}/", self.base_url, id);
        debug!("Fetching device {} from NetBox", id);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if response.status() == 404 {
            return Err(NetBoxError::NotFound(format!("Device {} not found", id)));
        }
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to get device {}: {} - {}",
                id, status, body
            )));
        }
        
        let device: Device = response.json().await
            .map_err(|e| NetBoxError::Http(e))?;
        Ok(device)
    }
    
    /// Query interfaces by filters
    ///
    /// # Arguments
    /// * `filters` - Query parameters (e.g., [("device_id", "1"), ("name", "eth0")])
    /// * `fetch_all` - If true, fetch all pages (default: false, returns first page only)
    ///
    /// # Returns
    /// * `Ok(Vec<Interface>)` - List of matching interfaces
    /// * `Err(NetBoxError)` - If the request fails
    pub async fn query_interfaces(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Interface>, NetBoxError> {
        let mut url = format!("{}/api/dcim/interfaces/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying interfaces with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query interfaces: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<Interface> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Query VLANs by filters
    ///
    /// # Arguments
    /// * `filters` - Query parameters (e.g., [("vid", "100"), ("site", "datacenter-1")])
    /// * `fetch_all` - If true, fetch all pages (default: false, returns first page only)
    ///
    /// # Returns
    /// * `Ok(Vec<Vlan>)` - List of matching VLANs
    /// * `Err(NetBoxError)` - If the request fails
    pub async fn query_vlans(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Vlan>, NetBoxError> {
        let mut url = format!("{}/api/ipam/vlans/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying VLANs with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query VLANs: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<Vlan> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get a VLAN by ID
    ///
    /// # Arguments
    /// * `id` - VLAN ID
    ///
    /// # Returns
    /// * `Ok(Vlan)` - The VLAN object
    /// * `Err(NetBoxError)` - If the request fails
    pub async fn get_vlan(&self, id: u64) -> Result<Vlan, NetBoxError> {
        let url = format!("{}/api/ipam/vlans/{}/", self.base_url, id);
        debug!("Fetching VLAN {} from NetBox", id);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if response.status() == 404 {
            return Err(NetBoxError::NotFound(format!("VLAN {} not found", id)));
        }
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to get VLAN {}: {} - {}",
                id, status, body
            )));
        }
        
        let vlan: Vlan = response.json().await
            .map_err(|e| NetBoxError::Http(e))?;
        Ok(vlan)
    }
    
    /// Create a new prefix in NetBox
    ///
    /// # Arguments
    /// * `prefix` - Prefix CIDR (e.g., "192.168.1.0/24")
    /// * `description` - Optional description
    /// * `site` - Optional site name
    /// * `vlan_id` - Optional VLAN ID
    /// * `status` - Prefix status (active, reserved, deprecated, container)
    /// * `role` - Optional role
    /// * `tags` - Optional tags
    ///
    /// # Returns
    /// * `Ok(Prefix)` - The created prefix
    /// * `Err(NetBoxError)` - If creation fails
    pub async fn create_prefix(
        &self,
        prefix: &str,
        description: Option<String>,
        site_id: Option<u64>,
        vlan_id: Option<u32>,
        status: Option<&str>,
        role_id: Option<u64>,
        tenant_id: Option<u64>,
        tags: Option<Vec<String>>,
    ) -> Result<Prefix, NetBoxError> {
        let url = format!("{}/api/ipam/prefixes/", self.base_url);
        debug!("Creating prefix {} in NetBox", prefix);
        
        let mut body = serde_json::json!({
            "prefix": prefix,
        });
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(sid) = site_id {
            body["site"] = serde_json::Value::Number(sid.into());
        }
        
        if let Some(vid) = vlan_id {
            body["vlan"] = serde_json::Value::Number(vid.into());
        }
        
        if let Some(status_str) = status {
            body["status"] = serde_json::Value::String(status_str.to_string());
        }
        
        if let Some(rid) = role_id {
            body["role"] = serde_json::Value::Number(rid.into());
        }
        
        if let Some(tid) = tenant_id {
            body["tenant"] = serde_json::Value::Number(tid.into());
        }
        
        if let Some(tags_vec) = tags {
            body["tags"] = serde_json::to_value(tags_vec)
                .map_err(|e| NetBoxError::Serialization(e))?;
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create prefix {}: {} - {}",
                prefix, status, body
            )));
        }
        
        let prefix_obj: Prefix = response.json().await
            .map_err(|e| NetBoxError::Http(e))?;
        Ok(prefix_obj)
    }
    
    /// Update an existing prefix in NetBox
    ///
    /// # Arguments
    /// * `id` - Prefix ID
    /// * `prefix` - Optional new prefix CIDR
    /// * `description` - Optional description
    /// * `status` - Optional status
    /// * `role` - Optional role
    /// * `tags` - Optional tags
    ///
    /// # Returns
    /// * `Ok(Prefix)` - The updated prefix
    /// * `Err(NetBoxError)` - If update fails
    pub async fn update_prefix(
        &self,
        id: u64,
        prefix: Option<&str>,
        description: Option<String>,
        status: Option<&str>,
        role: Option<String>,
        tenant_id: Option<u64>,
        site_id: Option<u64>,
        vlan_id: Option<u32>,
        tags: Option<Vec<String>>,
    ) -> Result<Prefix, NetBoxError> {
        let url = format!("{}/api/ipam/prefixes/{}/", self.base_url, id);
        debug!("Updating prefix {} in NetBox", id);
        
        let mut body = serde_json::json!({});
        
        if let Some(prefix_str) = prefix {
            body["prefix"] = serde_json::Value::String(prefix_str.to_string());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(status_str) = status {
            body["status"] = serde_json::Value::String(status_str.to_string());
        }
        
        if let Some(role_str) = role {
            body["role"] = serde_json::Value::String(role_str);
        }
        
        // Include tenant in update if provided (None means don't change tenant)
        if let Some(tid) = tenant_id {
            body["tenant"] = serde_json::Value::Number(tid.into());
        }
        
        // Include site in update if provided (None means don't change site)
        if let Some(sid) = site_id {
            body["site"] = serde_json::Value::Number(sid.into());
        }
        
        // Include vlan in update if provided (None means don't change vlan)
        if let Some(vid) = vlan_id {
            body["vlan"] = serde_json::Value::Number(vid.into());
        }
        
        if let Some(tags_vec) = tags {
            body["tags"] = serde_json::to_value(tags_vec)
                .map_err(|e| NetBoxError::Serialization(e))?;
        }
        
        let response = self.client
            .patch(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to update prefix {}: {} - {}",
                id, status, body
            )));
        }
        
        let prefix_obj: Prefix = response.json().await
            .map_err(|e| NetBoxError::Http(e))?;
        Ok(prefix_obj)
    }
    
    // ====================
    // Tenant API Methods
    // ====================
    
    /// Query tenants by filters
    pub async fn query_tenants(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Tenant>, NetBoxError> {
        let mut url = format!("{}/api/tenancy/tenants/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying tenants with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query tenants: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<Tenant> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get tenant by ID
    pub async fn get_tenant(&self, id: u64) -> Result<Tenant, NetBoxError> {
        let url = format!("{}/api/tenancy/tenants/{}/", self.base_url, id);
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to get tenant {}: {} - {}",
                id, status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    /// Create a new tenant
    pub async fn create_tenant(
        &self,
        name: &str,
        slug: Option<&str>,
        description: Option<String>,
        comments: Option<String>,
        group: Option<u64>, // Tenant group ID
    ) -> Result<Tenant, NetBoxError> {
        let url = format!("{}/api/tenancy/tenants/", self.base_url);
        debug!("Creating tenant {} in NetBox", name);
        
        let mut body = serde_json::json!({
            "name": name,
        });
        
        if let Some(slug_str) = slug {
            body["slug"] = serde_json::Value::String(slug_str.to_string());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        // Always include group field - set to null if not provided
        // NetBox API may require this field to be explicitly set
        if let Some(group_id) = group {
            body["group"] = serde_json::Value::Number(group_id.into());
        } else {
            body["group"] = serde_json::Value::Null;
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create tenant: {} - {}",
                status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    // ====================
    // Site API Methods
    // ====================
    
    /// Query sites by filters
    pub async fn query_sites(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Site>, NetBoxError> {
        let mut url = format!("{}/api/dcim/sites/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying sites with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query sites: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<Site> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get site by ID
    pub async fn get_site(&self, id: u64) -> Result<Site, NetBoxError> {
        let url = format!("{}/api/dcim/sites/{}/", self.base_url, id);
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to get site {}: {} - {}",
                id, status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    /// Create a new site
    pub async fn create_site(
        &self,
        name: &str,
        slug: Option<&str>,
        description: Option<String>,
        physical_address: Option<String>,
        shipping_address: Option<String>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        tenant_id: Option<u64>,
        region_id: Option<u64>,
        site_group_id: Option<u64>,
        status: Option<&str>, // "active", "planned", "retired", "staging"
        facility: Option<String>,
        time_zone: Option<String>,
        comments: Option<String>,
    ) -> Result<Site, NetBoxError> {
        let url = format!("{}/api/dcim/sites/", self.base_url);
        debug!("Creating site {} in NetBox", name);
        
        let mut body = serde_json::json!({
            "name": name,
        });
        
        if let Some(slug_str) = slug {
            body["slug"] = serde_json::Value::String(slug_str.to_string());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(addr) = physical_address {
            body["physical_address"] = serde_json::Value::String(addr);
        }
        
        if let Some(addr) = shipping_address {
            body["shipping_address"] = serde_json::Value::String(addr);
        }
        
        if let Some(lat) = latitude {
            body["latitude"] = serde_json::Value::Number(serde_json::Number::from_f64(lat).unwrap());
        }
        
        if let Some(lon) = longitude {
            body["longitude"] = serde_json::Value::Number(serde_json::Number::from_f64(lon).unwrap());
        }
        
        if let Some(tid) = tenant_id {
            body["tenant"] = serde_json::Value::Number(tid.into());
        }
        
        if let Some(rid) = region_id {
            body["region"] = serde_json::Value::Number(rid.into());
        }
        
        if let Some(sgid) = site_group_id {
            body["site_group"] = serde_json::Value::Number(sgid.into());
        }
        
        if let Some(status_str) = status {
            body["status"] = serde_json::Value::String(status_str.to_string());
        }
        
        if let Some(fac) = facility {
            body["facility"] = serde_json::Value::String(fac);
        }
        
        if let Some(tz) = time_zone {
            body["time_zone"] = serde_json::Value::String(tz);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create site: {} - {}",
                status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    /// Update a site
    /// 
    /// Note: For nested fields (tenant, region, site_group), only include them if they've changed.
    /// NetBox's nested serializers expect either an integer PK or a dictionary with attributes.
    /// When using PATCH, we send {"id": X} format for nested fields.
    pub async fn update_site(
        &self,
        id: u64,
        name: Option<&str>,
        slug: Option<&str>,
        description: Option<String>,
        physical_address: Option<String>,
        shipping_address: Option<String>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        tenant_id: Option<u64>,
        region_id: Option<u64>,
        site_group_id: Option<u64>,
        status: Option<&str>, // "active", "planned", "retired", "staging"
        facility: Option<String>,
        time_zone: Option<String>,
        comments: Option<String>,
    ) -> Result<Site, NetBoxError> {
        let url = format!("{}/api/dcim/sites/{}/", self.base_url, id);
        debug!("Updating site {} in NetBox", id);
        
        let mut body = serde_json::json!({});
        
        if let Some(name_str) = name {
            body["name"] = serde_json::Value::String(name_str.to_string());
        }
        
        if let Some(slug_str) = slug {
            body["slug"] = serde_json::Value::String(slug_str.to_string());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(addr) = physical_address {
            body["physical_address"] = serde_json::Value::String(addr);
        }
        
        if let Some(addr) = shipping_address {
            body["shipping_address"] = serde_json::Value::String(addr);
        }
        
        if let Some(lat) = latitude {
            body["latitude"] = serde_json::Value::Number(serde_json::Number::from_f64(lat).unwrap());
        }
        
        if let Some(lon) = longitude {
            body["longitude"] = serde_json::Value::Number(serde_json::Number::from_f64(lon).unwrap());
        }
        
        // For PATCH updates, NetBox's nested serializers accept:
        // 1. Integer PK (e.g., 1) - this is what we use for updates (same as create_site)
        // 2. Dictionary with attributes (e.g., {"id": 1}) - but NetBox 4.0 may require full object with name/group
        // We use integer format to match create_site behavior and avoid validation errors
        // NOTE: Only include tenant if it's provided (caller should only pass if changed)
        // If tenant_id is None, we don't include it in the body (leaves existing tenant unchanged)
        // IMPORTANT: Only include tenant in update if it's actually changed (caller responsibility)
        if let Some(tid) = tenant_id {
            // Send tenant ID as integer (same format as create_site)
            // This avoids NetBox 4.0 validation errors about requiring name/group fields
            body["tenant"] = serde_json::Value::Number(tid.into());
        }
        // If tenant_id is None, we don't include "tenant" in the body at all
        // This means "don't change the tenant" (PATCH semantics - only send changed fields)
        
        if let Some(rid) = region_id {
            body["region"] = serde_json::json!({"id": rid});
        }
        
        if let Some(sgid) = site_group_id {
            body["site_group"] = serde_json::json!({"id": sgid});
        }
        
        if let Some(status_str) = status {
            body["status"] = serde_json::Value::String(status_str.to_string());
        }
        
        if let Some(fac) = facility {
            body["facility"] = serde_json::Value::String(fac);
        }
        
        if let Some(tz) = time_zone {
            body["time_zone"] = serde_json::Value::String(tz);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .patch(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            // Check if response is HTML (404/error page) vs JSON error
            let body = if body_text.trim_start().starts_with("<!DOCTYPE") || body_text.trim_start().starts_with("<html") {
                // Extract error message from HTML if possible, otherwise use truncated HTML
                if body_text.len() > 500 {
                    format!("HTML error page (first 500 chars): {}", &body_text[..500])
                } else {
                    format!("HTML error page: {}", body_text)
                }
            } else {
                body_text
            };
            return Err(NetBoxError::Api(format!(
                "Failed to update site {}: {} - {}",
                id, status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    // ====================
    // Role API Methods
    // ====================
    
    /// Query roles by filters
    pub async fn query_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Role>, NetBoxError> {
        let mut url = format!("{}/api/ipam/roles/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying roles with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query roles: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<Role> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get role by ID
    pub async fn get_role(&self, id: u64) -> Result<Role, NetBoxError> {
        let url = format!("{}/api/ipam/roles/{}/", self.base_url, id);
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to get role {}: {} - {}",
                id, status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    /// Create a new role
    pub async fn create_role(
        &self,
        name: &str,
        slug: Option<&str>,
        description: Option<String>,
        weight: Option<u16>,
        comments: Option<String>,
    ) -> Result<Role, NetBoxError> {
        let url = format!("{}/api/ipam/roles/", self.base_url);
        debug!("Creating role {} in NetBox", name);
        
        let mut body = serde_json::json!({
            "name": name,
        });
        
        if let Some(slug_str) = slug {
            body["slug"] = serde_json::Value::String(slug_str.to_string());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(w) = weight {
            body["weight"] = serde_json::Value::Number(w.into());
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create role: {} - {}",
                status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    // ====================
    // Tag API Methods
    // ====================
    
    /// Query tags by filters
    pub async fn query_tags(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Tag>, NetBoxError> {
        let mut url = format!("{}/api/extras/tags/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying tags with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query tags: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<Tag> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get tag by ID
    pub async fn get_tag(&self, id: u64) -> Result<Tag, NetBoxError> {
        let url = format!("{}/api/extras/tags/{}/", self.base_url, id);
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to get tag {}: {} - {}",
                id, status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    /// Create a new tag
    pub async fn create_tag(
        &self,
        name: &str,
        slug: Option<&str>,
        color: Option<&str>, // Hex color code (e.g., "9e9e9e")
        description: Option<String>,
        comments: Option<String>,
    ) -> Result<Tag, NetBoxError> {
        let url = format!("{}/api/extras/tags/", self.base_url);
        debug!("Creating tag {} in NetBox", name);
        
        let mut body = serde_json::json!({
            "name": name,
        });
        
        if let Some(slug_str) = slug {
            body["slug"] = serde_json::Value::String(slug_str.to_string());
        }
        
        if let Some(color_str) = color {
            body["color"] = serde_json::Value::String(color_str.to_string());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create tag: {} - {}",
                status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    // ====================
    // Aggregate API Methods
    // ====================
    
    /// Query aggregates by filters
    pub async fn query_aggregates(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Aggregate>, NetBoxError> {
        let mut url = format!("{}/api/ipam/aggregates/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying aggregates with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query aggregates: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<Aggregate> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get aggregate by ID
    pub async fn get_aggregate(&self, id: u64) -> Result<Aggregate, NetBoxError> {
        let url = format!("{}/api/ipam/aggregates/{}/", self.base_url, id);
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to get aggregate {}: {} - {}",
                id, status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    /// Create a new aggregate
    pub async fn create_aggregate(
        &self,
        prefix: &str,
        rir_id: Option<u64>, // RIR ID (not name)
        date_allocated: Option<&str>, // ISO 8601 date
        description: Option<String>,
        comments: Option<String>,
    ) -> Result<Aggregate, NetBoxError> {
        let url = format!("{}/api/ipam/aggregates/", self.base_url);
        debug!("Creating aggregate {} in NetBox", prefix);
        
        let mut body = serde_json::json!({
            "prefix": prefix,
        });
        
        // RIR is required for aggregates - must be provided
        if let Some(rir) = rir_id {
            body["rir"] = serde_json::Value::Number(rir.into());
        } else {
            return Err(NetBoxError::Api(
                "RIR is required for aggregates but was not provided".to_string()
            ));
        }
        
        if let Some(date) = date_allocated {
            body["date_allocated"] = serde_json::Value::String(date.to_string());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create aggregate: {} - {}",
                status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    // ====================
    // RIR API Methods
    // ====================
    
    /// Query RIRs by filters
    pub async fn query_rirs(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Rir>, NetBoxError> {
        let mut url = format!("{}/api/ipam/rirs/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying RIRs with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query RIRs: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<Rir> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get RIR by name (slug or name)
    pub async fn get_rir_by_name(&self, name: &str) -> Result<Option<Rir>, NetBoxError> {
        // Try by name first
        let rirs = self.query_rirs(&[("name", name)], false).await?;
        if let Some(rir) = rirs.first() {
            return Ok(Some(rir.clone()));
        }
        
        // Try by slug
        let rirs = self.query_rirs(&[("slug", name)], false).await?;
        Ok(rirs.first().cloned())
    }
    
    // ====================
    // Tenant Group API Methods
    // ====================
    
    /// Query tenant groups by filters
    pub async fn query_tenant_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<TenantGroup>, NetBoxError> {
        let mut url = format!("{}/api/tenancy/tenant-groups/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying tenant groups with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query tenant groups: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<TenantGroup> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get tenant group by name (slug or name)
    pub async fn get_tenant_group_by_name(&self, name: &str) -> Result<Option<TenantGroup>, NetBoxError> {
        // Try by name first
        let groups = self.query_tenant_groups(&[("name", name)], false).await?;
        if let Some(group) = groups.first() {
            return Ok(Some(group.clone()));
        }
        
        // Try by slug
        let groups = self.query_tenant_groups(&[("slug", name)], false).await?;
        Ok(groups.first().cloned())
    }
    
    /// Create a new tenant group
    pub async fn create_tenant_group(
        &self,
        name: &str,
        slug: Option<&str>,
        description: Option<String>,
        comments: Option<String>,
        parent_id: Option<u64>,
    ) -> Result<TenantGroup, NetBoxError> {
        let url = format!("{}/api/tenancy/tenant-groups/", self.base_url);
        debug!("Creating tenant group {} in NetBox", name);
        
        // Slug is required for tenant groups
        let slug_value = if let Some(slug_str) = slug {
            slug_str.to_string()
        } else {
            // Auto-generate slug from name if not provided
            name.to_lowercase().replace(' ', "-")
        };
        
        let mut body = serde_json::json!({
            "name": name,
            "slug": slug_value,
        });
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        if let Some(parent) = parent_id {
            body["parent"] = serde_json::Value::Number(parent.into());
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create tenant group: {} - {}",
                status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    /// Create a new RIR
    pub async fn create_rir(
        &self,
        name: &str,
        slug: Option<&str>,
        description: Option<String>,
        is_private: Option<bool>,
    ) -> Result<Rir, NetBoxError> {
        let url = format!("{}/api/ipam/rirs/", self.base_url);
        debug!("Creating RIR {} in NetBox", name);
        
        // Slug is required for RIRs
        let slug_value = if let Some(slug_str) = slug {
            slug_str.to_string()
        } else {
            // Auto-generate slug from name if not provided
            name.to_lowercase().replace(' ', "-")
        };
        
        let mut body = serde_json::json!({
            "name": name,
            "slug": slug_value,
        });
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(private) = is_private {
            body["is_private"] = serde_json::Value::Bool(private);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create RIR: {} - {}",
                status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    // ============================================================================
    // DCIM API Methods - Device Roles
    // ============================================================================
    
    /// Query device roles by filters
    pub async fn query_device_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<DeviceRole>, NetBoxError> {
        let mut url = format!("{}/api/dcim/device-roles/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying device roles with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query device roles: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<DeviceRole> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get device role by name or slug
    pub async fn get_device_role_by_name(&self, name: &str) -> Result<Option<DeviceRole>, NetBoxError> {
        let roles = self.query_device_roles(&[("name", name)], false).await?;
        if let Some(role) = roles.first() {
            return Ok(Some(role.clone()));
        }
        
        let roles = self.query_device_roles(&[("slug", name)], false).await?;
        Ok(roles.first().cloned())
    }
    
    /// Create a new device role
    pub async fn create_device_role(
        &self,
        name: &str,
        slug: Option<&str>,
        color: Option<&str>,
        vm_role: Option<bool>,
        description: Option<String>,
        comments: Option<String>,
    ) -> Result<DeviceRole, NetBoxError> {
        let url = format!("{}/api/dcim/device-roles/", self.base_url);
        debug!("Creating device role {} in NetBox", name);
        
        let slug_value = if let Some(slug_str) = slug {
            slug_str.to_string()
        } else {
            name.to_lowercase().replace(' ', "-")
        };
        
        let mut body = serde_json::json!({
            "name": name,
            "slug": slug_value,
        });
        
        if let Some(color_str) = color {
            body["color"] = serde_json::Value::String(color_str.to_string());
        }
        
        if let Some(vm) = vm_role {
            body["vm_role"] = serde_json::Value::Bool(vm);
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create device role: {} - {}",
                status, body
            )));
        }
        
        // Capture response body for better error messages
        let response_text = response.text().await.map_err(|e| NetBoxError::Http(e))?;
        serde_json::from_str(&response_text).map_err(|e| {
            NetBoxError::Api(format!("error decoding response body: {} - Response: {}", e, response_text))
        })
    }
    
    // ============================================================================
    // DCIM API Methods - Manufacturers
    // ============================================================================
    
    /// Query manufacturers by filters
    pub async fn query_manufacturers(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Manufacturer>, NetBoxError> {
        let mut url = format!("{}/api/dcim/manufacturers/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying manufacturers with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query manufacturers: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<Manufacturer> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get manufacturer by name or slug
    pub async fn get_manufacturer_by_name(&self, name: &str) -> Result<Option<Manufacturer>, NetBoxError> {
        let manufacturers = self.query_manufacturers(&[("name", name)], false).await?;
        if let Some(mfg) = manufacturers.first() {
            return Ok(Some(mfg.clone()));
        }
        
        let manufacturers = self.query_manufacturers(&[("slug", name)], false).await?;
        Ok(manufacturers.first().cloned())
    }
    
    /// Create a new manufacturer
    pub async fn create_manufacturer(
        &self,
        name: &str,
        slug: Option<&str>,
        description: Option<String>,
    ) -> Result<Manufacturer, NetBoxError> {
        let url = format!("{}/api/dcim/manufacturers/", self.base_url);
        debug!("Creating manufacturer {} in NetBox", name);
        
        let slug_value = if let Some(slug_str) = slug {
            slug_str.to_string()
        } else {
            name.to_lowercase().replace(' ', "-")
        };
        
        let mut body = serde_json::json!({
            "name": name,
            "slug": slug_value,
        });
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create manufacturer: {} - {}",
                status, body
            )));
        }
        
        // Capture response body for better error messages
        let response_text = response.text().await.map_err(|e| NetBoxError::Http(e))?;
        serde_json::from_str(&response_text).map_err(|e| {
            NetBoxError::Api(format!("error decoding response body: {} - Response: {}", e, response_text))
        })
    }
    
    // ============================================================================
    // DCIM API Methods - Platforms
    // ============================================================================
    
    /// Query platforms by filters
    pub async fn query_platforms(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Platform>, NetBoxError> {
        let mut url = format!("{}/api/dcim/platforms/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying platforms with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query platforms: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<Platform> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get platform by name or slug
    pub async fn get_platform_by_name(&self, name: &str) -> Result<Option<Platform>, NetBoxError> {
        let platforms = self.query_platforms(&[("name", name)], false).await?;
        if let Some(platform) = platforms.first() {
            return Ok(Some(platform.clone()));
        }
        
        let platforms = self.query_platforms(&[("slug", name)], false).await?;
        Ok(platforms.first().cloned())
    }
    
    /// Create a new platform
    pub async fn create_platform(
        &self,
        name: &str,
        slug: Option<&str>,
        manufacturer_id: Option<u64>,
        napalm_driver: Option<&str>,
        napalm_args: Option<&str>,
        description: Option<String>,
        comments: Option<String>,
    ) -> Result<Platform, NetBoxError> {
        let url = format!("{}/api/dcim/platforms/", self.base_url);
        debug!("Creating platform {} in NetBox", name);
        
        let slug_value = if let Some(slug_str) = slug {
            slug_str.to_string()
        } else {
            name.to_lowercase().replace(' ', "-")
        };
        
        let mut body = serde_json::json!({
            "name": name,
            "slug": slug_value,
        });
        
        if let Some(mfg_id) = manufacturer_id {
            body["manufacturer"] = serde_json::Value::Number(mfg_id.into());
        }
        
        if let Some(driver) = napalm_driver {
            body["napalm_driver"] = serde_json::Value::String(driver.to_string());
        }
        
        if let Some(args) = napalm_args {
            body["napalm_args"] = serde_json::Value::String(args.to_string());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create platform: {} - {}",
                status, body
            )));
        }
        
        // Capture response body for better error messages
        let response_text = response.text().await.map_err(|e| NetBoxError::Http(e))?;
        serde_json::from_str(&response_text).map_err(|e| {
            NetBoxError::Api(format!("error decoding response body: {} - Response: {}", e, response_text))
        })
    }
    
    // ============================================================================
    // DCIM API Methods - Device Types
    // ============================================================================
    
    /// Query device types by filters
    pub async fn query_device_types(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<DeviceType>, NetBoxError> {
        let mut url = format!("{}/api/dcim/device-types/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying device types with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query device types: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<DeviceType> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get device type by manufacturer and model
    pub async fn get_device_type_by_model(&self, manufacturer_id: u64, model: &str) -> Result<Option<DeviceType>, NetBoxError> {
        let device_types = self.query_device_types(&[("manufacturer_id", &manufacturer_id.to_string()), ("model", model)], false).await?;
        Ok(device_types.first().cloned())
    }
    
    /// Create a new device type
    pub async fn create_device_type(
        &self,
        manufacturer_id: u64,
        model: &str,
        slug: Option<&str>,
        part_number: Option<&str>,
        u_height: Option<f64>,
        is_full_depth: Option<bool>,
        description: Option<String>,
        comments: Option<String>,
    ) -> Result<DeviceType, NetBoxError> {
        let url = format!("{}/api/dcim/device-types/", self.base_url);
        debug!("Creating device type {} in NetBox", model);
        
        let slug_value = if let Some(slug_str) = slug {
            slug_str.to_string()
        } else {
            model.to_lowercase().replace(' ', "-")
        };
        
        let mut body = serde_json::json!({
            "manufacturer": manufacturer_id,
            "model": model,
            "slug": slug_value,
        });
        
        if let Some(part) = part_number {
            body["part_number"] = serde_json::Value::String(part.to_string());
        }
        
        if let Some(height) = u_height {
            body["u_height"] = serde_json::Value::Number(serde_json::Number::from_f64(height).unwrap_or(serde_json::Number::from(1)));
        }
        
        if let Some(full_depth) = is_full_depth {
            body["is_full_depth"] = serde_json::Value::Bool(full_depth);
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create device type: {} - {}",
                status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    // ============================================================================
    // DCIM API Methods - Devices
    // ============================================================================
    
    /// Create a new device (note: query_devices and get_device already exist above)
    pub async fn create_device(
        &self,
        device_type_id: u64,
        device_role_id: u64,
        site_id: u64,
        name: Option<&str>,
        tenant_id: Option<u64>,
        platform_id: Option<u64>,
        location_id: Option<u64>,
        serial: Option<&str>,
        asset_tag: Option<&str>,
        status: Option<&str>,
        primary_ip4_id: Option<u64>,
        primary_ip6_id: Option<u64>,
        description: Option<String>,
        comments: Option<String>,
    ) -> Result<Device, NetBoxError> {
        let url = format!("{}/api/dcim/devices/", self.base_url);
        debug!("Creating device in NetBox");
        
        let mut body = serde_json::json!({
            "device_type": device_type_id,
            "role": device_role_id,
            "site": site_id,
        });
        
        if let Some(name_str) = name {
            body["name"] = serde_json::Value::String(name_str.to_string());
        }
        
        if let Some(tenant) = tenant_id {
            body["tenant"] = serde_json::Value::Number(tenant.into());
        }
        
        if let Some(platform) = platform_id {
            body["platform"] = serde_json::Value::Number(platform.into());
        }
        
        if let Some(location) = location_id {
            body["location"] = serde_json::Value::Number(location.into());
        }
        
        if let Some(serial_str) = serial {
            body["serial"] = serde_json::Value::String(serial_str.to_string());
        }
        
        if let Some(asset) = asset_tag {
            body["asset_tag"] = serde_json::Value::String(asset.to_string());
        }
        
        if let Some(status_str) = status {
            body["status"] = serde_json::Value::String(status_str.to_string());
        }
        
        if let Some(ip4) = primary_ip4_id {
            body["primary_ip4"] = serde_json::Value::Number(ip4.into());
        }
        
        if let Some(ip6) = primary_ip6_id {
            body["primary_ip6"] = serde_json::Value::Number(ip6.into());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create device: {} - {}",
                status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    /// Update a device
    pub async fn update_device(
        &self,
        id: u64,
        name: Option<&str>,
        tenant_id: Option<u64>,
        platform_id: Option<u64>,
        location_id: Option<u64>,
        serial: Option<&str>,
        asset_tag: Option<&str>,
        status: Option<&str>,
        primary_ip4_id: Option<u64>,
        primary_ip6_id: Option<u64>,
        description: Option<String>,
        comments: Option<String>,
    ) -> Result<Device, NetBoxError> {
        let url = format!("{}/api/dcim/devices/{}/", self.base_url, id);
        debug!("Updating device {} in NetBox", id);
        
        let mut body = serde_json::json!({});
        
        if let Some(name_str) = name {
            body["name"] = serde_json::Value::String(name_str.to_string());
        }
        
        if let Some(tenant) = tenant_id {
            body["tenant"] = serde_json::Value::Number(tenant.into());
        }
        
        if let Some(platform) = platform_id {
            body["platform"] = serde_json::Value::Number(platform.into());
        }
        
        if let Some(location) = location_id {
            body["location"] = serde_json::Value::Number(location.into());
        }
        
        if let Some(serial_str) = serial {
            body["serial"] = serde_json::Value::String(serial_str.to_string());
        }
        
        if let Some(asset) = asset_tag {
            body["asset_tag"] = serde_json::Value::String(asset.to_string());
        }
        
        if let Some(status_str) = status {
            body["status"] = serde_json::Value::String(status_str.to_string());
        }
        
        if let Some(ip4) = primary_ip4_id {
            body["primary_ip4"] = serde_json::Value::Number(ip4.into());
        }
        
        if let Some(ip6) = primary_ip6_id {
            body["primary_ip6"] = serde_json::Value::Number(ip6.into());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .patch(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to update device {}: {} - {}",
                id, status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    // ============================================================================
    // DCIM API Methods - Interfaces
    // ============================================================================
    
    /// Get interface by ID
    pub async fn get_interface(&self, id: u64) -> Result<Interface, NetBoxError> {
        let url = format!("{}/api/dcim/interfaces/{}/", self.base_url, id);
        debug!("Fetching interface {} from NetBox", id);
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if response.status() == 404 {
            return Err(NetBoxError::NotFound(format!("Interface {} not found", id)));
        }
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to get interface {}: {} - {}",
                id, status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    /// Create a new interface
    pub async fn create_interface(
        &self,
        device_id: u64,
        name: &str,
        interface_type: &str,
        enabled: Option<bool>,
        mac_address: Option<&str>,
        mtu: Option<u16>,
        description: Option<String>,
    ) -> Result<Interface, NetBoxError> {
        let url = format!("{}/api/dcim/interfaces/", self.base_url);
        debug!("Creating interface {} on device {} in NetBox", name, device_id);
        
        let mut body = serde_json::json!({
            "device": device_id,
            "name": name,
            "type": interface_type,
        });
        
        if let Some(enabled_val) = enabled {
            body["enabled"] = serde_json::Value::Bool(enabled_val);
        }
        
        if let Some(mac) = mac_address {
            body["mac_address"] = serde_json::Value::String(mac.to_string());
        }
        
        if let Some(mtu_val) = mtu {
            body["mtu"] = serde_json::Value::Number(mtu_val.into());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create interface: {} - {}",
                status, body
            )));
        }
        
        // Try to deserialize, but capture the response body for better error messages
        let response_text = response.text().await?;
        let interface: Interface = serde_json::from_str(&response_text).map_err(|e| {
            NetBoxError::Api(format!(
                "error decoding response body: {} - Response (first 500 chars): {}",
                e,
                response_text.chars().take(500).collect::<String>()
            ))
        })?;
        Ok(interface)
    }
    
    /// Update an interface
    pub async fn update_interface(
        &self,
        id: u64,
        name: Option<&str>,
        interface_type: Option<&str>,
        enabled: Option<bool>,
        mac_address: Option<&str>,
        mtu: Option<u16>,
        description: Option<String>,
    ) -> Result<Interface, NetBoxError> {
        let url = format!("{}/api/dcim/interfaces/{}/", self.base_url, id);
        debug!("Updating interface {} in NetBox", id);
        
        let mut body = serde_json::json!({});
        
        if let Some(name_str) = name {
            body["name"] = serde_json::Value::String(name_str.to_string());
        }
        
        if let Some(if_type) = interface_type {
            body["type"] = serde_json::Value::String(if_type.to_string());
        }
        
        if let Some(enabled_val) = enabled {
            body["enabled"] = serde_json::Value::Bool(enabled_val);
        }
        
        if let Some(mac) = mac_address {
            body["mac_address"] = serde_json::Value::String(mac.to_string());
        }
        
        if let Some(mtu_val) = mtu {
            body["mtu"] = serde_json::Value::Number(mtu_val.into());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        let response = self.client
            .patch(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to update interface {}: {} - {}",
                id, status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    // ============================================================================
    // DCIM API Methods - MAC Addresses
    // ============================================================================
    
    /// Query MAC addresses by filters
    pub async fn query_mac_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<MACAddress>, NetBoxError> {
        let mut url = format!("{}/api/dcim/mac-addresses/", self.base_url);
        
        if !filters.is_empty() {
            let query: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            url = format!("{}?{}", url, query.join("&"));
        }
        
        debug!("Querying MAC addresses with filters: {:?}", filters);
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                // Check if response is HTML (404 page) vs JSON error
                if body.trim_start().starts_with("<!DOCTYPE") || body.trim_start().starts_with("<html") {
                    return Err(NetBoxError::NotFound(format!(
                        "MAC addresses endpoint not found (404): {}",
                        status
                    )));
                }
                return Err(NetBoxError::Api(format!(
                    "Failed to query MAC addresses: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<MACAddress> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get MAC address by address
    pub async fn get_mac_address_by_address(&self, mac: &str) -> Result<Option<MACAddress>, NetBoxError> {
        let mac_addresses = self.query_mac_addresses(&[("mac_address", mac)], false).await?;
        Ok(mac_addresses.first().cloned())
    }
    
    /// Create a new MAC address
    pub async fn create_mac_address(
        &self,
        mac_address: &str,
        assigned_object_type: &str, // e.g., "dcim.interface"
        assigned_object_id: u64,
        description: Option<String>,
        comments: Option<String>,
    ) -> Result<MACAddress, NetBoxError> {
        let url = format!("{}/api/dcim/mac-addresses/", self.base_url);
        debug!("Creating MAC address {} in NetBox", mac_address);
        
        let mut body = serde_json::json!({
            "mac_address": mac_address,
            "assigned_object_type": assigned_object_type,
            "assigned_object_id": assigned_object_id,
        });
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            // Check if response is HTML (404 page) vs JSON error
            if body.trim_start().starts_with("<!DOCTYPE") || body.trim_start().starts_with("<html") {
                return Err(NetBoxError::NotFound(format!(
                    "MAC addresses endpoint not found (404): {}",
                    status
                )));
            }
            return Err(NetBoxError::Api(format!(
                "Failed to create MAC address: {} - {}",
                status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    // ============================================================================
    // IPAM API Methods - VLANs
    // ============================================================================
    
    /// Create a new VLAN
    pub async fn create_vlan(
        &self,
        vid: u16,
        name: &str,
        site_id: Option<u64>,
        group_id: Option<u64>,
        tenant_id: Option<u64>,
        role_id: Option<u64>,
        status: Option<&str>,
        description: Option<String>,
        comments: Option<String>,
    ) -> Result<Vlan, NetBoxError> {
        let url = format!("{}/api/ipam/vlans/", self.base_url);
        debug!("Creating VLAN {} ({}) in NetBox", vid, name);
        
        let mut body = serde_json::json!({
            "vid": vid,
            "name": name,
        });
        
        if let Some(site) = site_id {
            body["site"] = serde_json::Value::Number(site.into());
        }
        
        if let Some(group) = group_id {
            body["group"] = serde_json::Value::Number(group.into());
        }
        
        if let Some(tenant) = tenant_id {
            body["tenant"] = serde_json::Value::Number(tenant.into());
        }
        
        if let Some(role) = role_id {
            body["role"] = serde_json::Value::Number(role.into());
        }
        
        if let Some(status_str) = status {
            body["status"] = serde_json::Value::String(status_str.to_string());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create VLAN: {} - {}",
                status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    /// Update a VLAN
    pub async fn update_vlan(
        &self,
        id: u64,
        vid: Option<u16>,
        name: Option<&str>,
        site_id: Option<u64>,
        group_id: Option<u64>,
        tenant_id: Option<u64>,
        role_id: Option<u64>,
        status: Option<&str>,
        description: Option<String>,
        comments: Option<String>,
    ) -> Result<Vlan, NetBoxError> {
        let url = format!("{}/api/ipam/vlans/{}/", self.base_url, id);
        debug!("Updating VLAN {} in NetBox", id);
        
        let mut body = serde_json::json!({});
        
        if let Some(vid_val) = vid {
            body["vid"] = serde_json::Value::Number(vid_val.into());
        }
        
        if let Some(name_str) = name {
            body["name"] = serde_json::Value::String(name_str.to_string());
        }
        
        if let Some(site) = site_id {
            body["site"] = serde_json::Value::Number(site.into());
        }
        
        if let Some(group) = group_id {
            body["group"] = serde_json::Value::Number(group.into());
        }
        
        if let Some(tenant) = tenant_id {
            body["tenant"] = serde_json::Value::Number(tenant.into());
        }
        
        if let Some(role) = role_id {
            body["role"] = serde_json::Value::Number(role.into());
        }
        
        if let Some(status_str) = status {
            body["status"] = serde_json::Value::String(status_str.to_string());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .patch(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to update VLAN {}: {} - {}",
                id, status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    // ============================================================================
    // DCIM API Methods - Regions
    // ============================================================================
    
    /// Query regions
    pub async fn query_regions(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Region>, NetBoxError> {
        let mut url = format!("{}/api/dcim/regions/", self.base_url);
        
        if !filters.is_empty() {
            let query_params: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect();
            url.push('?');
            url.push_str(&query_params.join("&"));
        }
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query regions: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<Region> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get region by name
    pub async fn get_region_by_name(&self, name: &str) -> Result<Option<Region>, NetBoxError> {
        let regions = self.query_regions(&[("name", name)], false).await?;
        Ok(regions.first().cloned())
    }
    
    /// Get region by ID
    pub async fn get_region(&self, id: u64) -> Result<Region, NetBoxError> {
        let url = format!("{}/api/dcim/regions/{}/", self.base_url, id);
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to get region {}: {} - {}",
                id, status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    /// Create a new region
    pub async fn create_region(
        &self,
        name: &str,
        slug: Option<&str>,
        parent_id: Option<u64>,
        description: Option<String>,
        comments: Option<String>,
    ) -> Result<Region, NetBoxError> {
        let url = format!("{}/api/dcim/regions/", self.base_url);
        debug!("Creating region {} in NetBox", name);
        
        let slug_value = if let Some(slug_str) = slug {
            slug_str.to_string()
        } else {
            name.to_lowercase().replace(' ', "-")
        };
        
        let mut body = serde_json::json!({
            "name": name,
            "slug": slug_value,
        });
        
        if let Some(parent) = parent_id {
            body["parent"] = serde_json::Value::Number(parent.into());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create region: {} - {}",
                status, body
            )));
        }
        
        // Capture response body for better error messages
        let response_text = response.text().await.map_err(|e| NetBoxError::Http(e))?;
        serde_json::from_str(&response_text).map_err(|e| {
            NetBoxError::Api(format!("error decoding response body: {} - Response: {}", e, response_text))
        })
    }
    
    // ============================================================================
    // DCIM API Methods - Site Groups
    // ============================================================================
    
    /// Query site groups
    pub async fn query_site_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<SiteGroup>, NetBoxError> {
        let mut url = format!("{}/api/dcim/site-groups/", self.base_url);
        
        if !filters.is_empty() {
            let query_params: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect();
            url.push('?');
            url.push_str(&query_params.join("&"));
        }
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query site groups: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<SiteGroup> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get site group by name
    pub async fn get_site_group_by_name(&self, name: &str) -> Result<Option<SiteGroup>, NetBoxError> {
        let site_groups = self.query_site_groups(&[("name", name)], false).await?;
        Ok(site_groups.first().cloned())
    }
    
    /// Get site group by ID
    pub async fn get_site_group(&self, id: u64) -> Result<SiteGroup, NetBoxError> {
        let url = format!("{}/api/dcim/site-groups/{}/", self.base_url, id);
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to get site group {}: {} - {}",
                id, status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    /// Create a new site group
    pub async fn create_site_group(
        &self,
        name: &str,
        slug: Option<&str>,
        parent_id: Option<u64>,
        description: Option<String>,
        comments: Option<String>,
    ) -> Result<SiteGroup, NetBoxError> {
        let url = format!("{}/api/dcim/site-groups/", self.base_url);
        debug!("Creating site group {} in NetBox", name);
        
        let slug_value = if let Some(slug_str) = slug {
            slug_str.to_string()
        } else {
            name.to_lowercase().replace(' ', "-")
        };
        
        let mut body = serde_json::json!({
            "name": name,
            "slug": slug_value,
        });
        
        if let Some(parent) = parent_id {
            body["parent"] = serde_json::Value::Number(parent.into());
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create site group: {} - {}",
                status, body
            )));
        }
        
        // Capture response body for better error messages
        let response_text = response.text().await.map_err(|e| NetBoxError::Http(e))?;
        serde_json::from_str(&response_text).map_err(|e| {
            NetBoxError::Api(format!("error decoding response body: {} - Response: {}", e, response_text))
        })
    }
    
    // ============================================================================
    // DCIM API Methods - Locations
    // ============================================================================
    
    /// Query locations
    pub async fn query_locations(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Location>, NetBoxError> {
        let mut url = format!("{}/api/dcim/locations/", self.base_url);
        
        if !filters.is_empty() {
            let query_params: Vec<String> = filters.iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect();
            url.push('?');
            url.push_str(&query_params.join("&"));
        }
        
        if fetch_all {
            self.fetch_all_pages(url).await
        } else {
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Token {}", self.token))
                .header("Accept", "application/json")
                .send()
                .await?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(NetBoxError::Api(format!(
                    "Failed to query locations: {} - {}",
                    status, body
                )));
            }
            
            let result: PaginatedResponse<Location> = response.json().await?;
            Ok(result.results)
        }
    }
    
    /// Get location by name and site
    pub async fn get_location_by_name(&self, site_id: u64, name: &str) -> Result<Option<Location>, NetBoxError> {
        let locations = self.query_locations(&[("site_id", &site_id.to_string()), ("name", name)], false).await?;
        Ok(locations.first().cloned())
    }
    
    /// Get location by ID
    pub async fn get_location(&self, id: u64) -> Result<Location, NetBoxError> {
        let url = format!("{}/api/dcim/locations/{}/", self.base_url, id);
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to get location {}: {} - {}",
                id, status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
    
    /// Create a new location
    pub async fn create_location(
        &self,
        site_id: u64,
        name: &str,
        slug: Option<&str>,
        parent_id: Option<u64>,
        description: Option<String>,
        comments: Option<String>,
    ) -> Result<Location, NetBoxError> {
        let url = format!("{}/api/dcim/locations/", self.base_url);
        debug!("Creating location {} in NetBox", name);
        
        let slug_value = if let Some(slug_str) = slug {
            slug_str.to_string()
        } else {
            name.to_lowercase().replace(' ', "-")
        };
        
        let mut body = serde_json::json!({
            "site": site_id,
            "name": name,
            "slug": slug_value,
        });
        
        // NetBox requires parent field - send null if not provided (top-level location)
        if let Some(parent) = parent_id {
            body["parent"] = serde_json::Value::Number(parent.into());
        } else {
            body["parent"] = serde_json::Value::Null; // Top-level location
        }
        
        if let Some(desc) = description {
            body["description"] = serde_json::Value::String(desc);
        }
        
        if let Some(comments_str) = comments {
            body["comments"] = serde_json::Value::String(comments_str);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NetBoxError::Http(e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NetBoxError::Api(format!(
                "Failed to create location: {} - {}",
                status, body
            )));
        }
        
        response.json().await.map_err(|e| NetBoxError::Http(e))
    }
}

// Implement NetBoxClientTrait for NetBoxClient
// This delegates all trait methods to the existing implementations
#[async_trait::async_trait]
impl NetBoxClientTrait for NetBoxClient {
    fn base_url(&self) -> &str {
        self.base_url()
    }

    async fn validate_token(&self) -> Result<(), NetBoxError> {
        self.validate_token().await
    }

    // IPAM Operations
    async fn get_prefix(&self, id: u64) -> Result<Prefix, NetBoxError> {
        self.get_prefix(id).await
    }

    async fn get_available_ips(&self, prefix_id: u64, limit: Option<u32>) -> Result<Vec<AvailableIP>, NetBoxError> {
        self.get_available_ips(prefix_id, limit).await
    }

    async fn allocate_ip(&self, prefix_id: u64, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        self.allocate_ip(prefix_id, request).await
    }

    async fn get_ip_address(&self, id: u64) -> Result<IPAddress, NetBoxError> {
        self.get_ip_address(id).await
    }

    async fn query_ip_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<IPAddress>, NetBoxError> {
        self.query_ip_addresses(filters, fetch_all).await
    }

    async fn query_prefixes(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Prefix>, NetBoxError> {
        self.query_prefixes(filters, fetch_all).await
    }

    async fn create_ip_address(&self, address: &str, request: Option<AllocateIPRequest>) -> Result<IPAddress, NetBoxError> {
        self.create_ip_address(address, request).await
    }

    async fn update_ip_address(&self, id: u64, request: AllocateIPRequest) -> Result<IPAddress, NetBoxError> {
        self.update_ip_address(id, request).await
    }

    async fn delete_ip_address(&self, id: u64) -> Result<(), NetBoxError> {
        self.delete_ip_address(id).await
    }

    async fn create_prefix(&self, prefix: &str, site_id: Option<u64>, tenant_id: Option<u64>, vlan_id: Option<u32>, role_id: Option<u64>, status: Option<&str>, description: Option<&str>, tags: Option<Vec<serde_json::Value>>) -> Result<Prefix, NetBoxError> {
        // Map trait parameters to actual method signature: prefix, description, site_id, vlan_id, status, role_id, tenant_id, tags
        let tags_vec: Option<Vec<String>> = tags.map(|v| {
            v.into_iter()
                .filter_map(|val| val.as_str().map(|s| s.to_string()))
                .collect()
        });
        self.create_prefix(prefix, description.map(|s| s.to_string()), site_id, vlan_id, status, role_id, tenant_id, tags_vec).await
    }

    async fn update_prefix(&self, id: u64, site_id: Option<u64>, tenant_id: Option<u64>, vlan_id: Option<u32>, role_id: Option<u64>, status: Option<&str>, description: Option<&str>, tags: Option<Vec<serde_json::Value>>) -> Result<Prefix, NetBoxError> {
        // Map trait parameters to actual method signature: id, prefix, description, status, role, tenant_id, site_id, vlan_id, tags
        let tags_vec: Option<Vec<String>> = tags.map(|v| {
            v.into_iter()
                .filter_map(|val| val.as_str().map(|s| s.to_string()))
                .collect()
        });
        let role_str = role_id.map(|r| r.to_string());
        self.update_prefix(id, None, description.map(|s| s.to_string()), status, role_str, tenant_id, site_id, vlan_id, tags_vec).await
    }

    async fn query_aggregates(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Aggregate>, NetBoxError> {
        self.query_aggregates(filters, fetch_all).await
    }

    async fn get_aggregate(&self, id: u64) -> Result<Aggregate, NetBoxError> {
        self.get_aggregate(id).await
    }

    async fn create_aggregate(&self, prefix: &str, rir_id: u64, description: Option<&str>) -> Result<Aggregate, NetBoxError> {
        self.create_aggregate(prefix, Some(rir_id), None, description.map(|s| s.to_string()), None).await
    }

    async fn query_rirs(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Rir>, NetBoxError> {
        self.query_rirs(filters, fetch_all).await
    }

    async fn get_rir_by_name(&self, name: &str) -> Result<Option<Rir>, NetBoxError> {
        self.get_rir_by_name(name).await
    }

    async fn create_rir(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Rir, NetBoxError> {
        self.create_rir(name, Some(slug), description.map(|s| s.to_string()), None).await
    }

    async fn create_vlan(&self, site_id: u64, vid: u32, name: &str, status: Option<&str>, description: Option<&str>) -> Result<Vlan, NetBoxError> {
        self.create_vlan(vid as u16, name, Some(site_id), None, None, None, status, description.map(|s| s.to_string()), None).await
    }

    async fn update_vlan(&self, id: u64, site_id: Option<u64>, vid: Option<u32>, name: Option<&str>, status: Option<&str>, description: Option<&str>) -> Result<Vlan, NetBoxError> {
        self.update_vlan(id, vid.map(|v| v as u16), name, site_id, None, None, None, status, description.map(|s| s.to_string()), None).await
    }

    async fn query_vlans(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Vlan>, NetBoxError> {
        self.query_vlans(filters, fetch_all).await
    }

    async fn get_vlan(&self, id: u64) -> Result<Vlan, NetBoxError> {
        self.get_vlan(id).await
    }

    // DCIM Operations
    async fn query_devices(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Device>, NetBoxError> {
        self.query_devices(filters, fetch_all).await
    }

    async fn get_device(&self, id: u64) -> Result<Device, NetBoxError> {
        self.get_device(id).await
    }

    async fn get_device_by_mac(&self, mac: &str) -> Result<Option<Device>, NetBoxError> {
        self.get_device_by_mac(mac).await
    }

    async fn create_device(&self, name: &str, device_type_id: u64, device_role_id: u64, site_id: u64, location_id: Option<u64>, tenant_id: Option<u64>, platform_id: Option<u64>, serial: Option<&str>, asset_tag: Option<&str>, status: &str, primary_ip4_id: Option<u64>, primary_ip6_id: Option<u64>, description: Option<&str>, comments: Option<&str>) -> Result<Device, NetBoxError> {
        self.create_device(device_type_id, device_role_id, site_id, Some(name), tenant_id, platform_id, location_id, serial, asset_tag, Some(status), primary_ip4_id, primary_ip6_id, description.map(|s| s.to_string()), comments.map(|s| s.to_string())).await
    }

    async fn update_device(&self, id: u64, name: Option<&str>, _device_type_id: Option<u64>, _device_role_id: Option<u64>, _site_id: Option<u64>, location_id: Option<u64>, tenant_id: Option<u64>, platform_id: Option<u64>, serial: Option<&str>, asset_tag: Option<&str>, status: Option<&str>, primary_ip4_id: Option<u64>, primary_ip6_id: Option<u64>, description: Option<&str>, comments: Option<&str>) -> Result<Device, NetBoxError> {
        self.update_device(id, name, tenant_id, platform_id, location_id, serial, asset_tag, status, primary_ip4_id, primary_ip6_id, description.map(|s| s.to_string()), comments.map(|s| s.to_string())).await
    }

    async fn query_interfaces(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Interface>, NetBoxError> {
        self.query_interfaces(filters, fetch_all).await
    }

    async fn get_interface(&self, id: u64) -> Result<Interface, NetBoxError> {
        self.get_interface(id).await
    }

    async fn create_interface(&self, device_id: u64, name: &str, interface_type: &str, enabled: bool, description: Option<&str>) -> Result<Interface, NetBoxError> {
        self.create_interface(device_id, name, interface_type, Some(enabled), None, None, description.map(|s| s.to_string())).await
    }

    async fn update_interface(&self, id: u64, name: Option<&str>, interface_type: Option<&str>, enabled: Option<bool>, mac_address: Option<&str>, description: Option<&str>) -> Result<Interface, NetBoxError> {
        self.update_interface(id, name, interface_type, enabled, mac_address, None, description.map(|s| s.to_string())).await
    }

    async fn query_mac_addresses(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<MACAddress>, NetBoxError> {
        self.query_mac_addresses(filters, fetch_all).await
    }

    async fn get_mac_address_by_address(&self, mac: &str) -> Result<Option<MACAddress>, NetBoxError> {
        self.get_mac_address_by_address(mac).await
    }

    async fn create_mac_address(&self, interface_id: u64, address: &str, description: Option<&str>) -> Result<MACAddress, NetBoxError> {
        self.create_mac_address(address, "dcim.interface", interface_id, description.map(|s| s.to_string()), None).await
    }

    async fn query_sites(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Site>, NetBoxError> {
        self.query_sites(filters, fetch_all).await
    }

    async fn get_site(&self, id: u64) -> Result<Site, NetBoxError> {
        self.get_site(id).await
    }

    async fn create_site(&self, name: &str, slug: Option<&str>, status: &str, region_id: Option<u64>, site_group_id: Option<u64>, tenant_id: Option<u64>, facility: Option<&str>, time_zone: Option<&str>, description: Option<&str>, comments: Option<&str>) -> Result<Site, NetBoxError> {
        self.create_site(name, slug, description.map(|s| s.to_string()), None, None, None, None, tenant_id, region_id, site_group_id, Some(status), facility.map(|s| s.to_string()), time_zone.map(|s| s.to_string()), comments.map(|s| s.to_string())).await
    }

    async fn update_site(&self, id: u64, name: Option<&str>, slug: Option<&str>, status: Option<&str>, region_id: Option<u64>, site_group_id: Option<u64>, tenant_id: Option<u64>, facility: Option<&str>, time_zone: Option<&str>, description: Option<&str>, comments: Option<&str>) -> Result<Site, NetBoxError> {
        self.update_site(id, name, slug, description.map(|s| s.to_string()), None, None, None, None, tenant_id, region_id, site_group_id, status, facility.map(|s| s.to_string()), time_zone.map(|s| s.to_string()), comments.map(|s| s.to_string())).await
    }

    async fn query_regions(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Region>, NetBoxError> {
        self.query_regions(filters, fetch_all).await
    }

    async fn get_region(&self, id: u64) -> Result<Region, NetBoxError> {
        self.get_region(id).await
    }

    async fn get_region_by_name(&self, name: &str) -> Result<Option<Region>, NetBoxError> {
        self.get_region_by_name(name).await
    }

    async fn create_region(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Region, NetBoxError> {
        self.create_region(name, Some(slug), None, description.map(|s| s.to_string()), None).await
    }

    async fn query_site_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<SiteGroup>, NetBoxError> {
        self.query_site_groups(filters, fetch_all).await
    }

    async fn get_site_group(&self, id: u64) -> Result<SiteGroup, NetBoxError> {
        self.get_site_group(id).await
    }

    async fn get_site_group_by_name(&self, name: &str) -> Result<Option<SiteGroup>, NetBoxError> {
        self.get_site_group_by_name(name).await
    }

    async fn create_site_group(&self, name: &str, slug: &str, description: Option<&str>) -> Result<SiteGroup, NetBoxError> {
        self.create_site_group(name, Some(slug), None, description.map(|s| s.to_string()), None).await
    }

    async fn query_locations(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Location>, NetBoxError> {
        self.query_locations(filters, fetch_all).await
    }

    async fn get_location(&self, id: u64) -> Result<Location, NetBoxError> {
        self.get_location(id).await
    }

    async fn get_location_by_name(&self, site_id: u64, name: &str) -> Result<Option<Location>, NetBoxError> {
        self.get_location_by_name(site_id, name).await
    }

    async fn create_location(&self, site_id: u64, name: &str, slug: Option<&str>, parent_id: Option<u64>, description: Option<String>, comments: Option<String>) -> Result<Location, NetBoxError> {
        self.create_location(site_id, name, slug, parent_id, description, comments).await
    }

    async fn query_device_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<DeviceRole>, NetBoxError> {
        self.query_device_roles(filters, fetch_all).await
    }

    async fn get_device_role_by_name(&self, name: &str) -> Result<Option<DeviceRole>, NetBoxError> {
        self.get_device_role_by_name(name).await
    }

    async fn create_device_role(&self, name: &str, slug: &str, description: Option<&str>) -> Result<DeviceRole, NetBoxError> {
        self.create_device_role(name, Some(slug), None, None, description.map(|s| s.to_string()), None).await
    }

    async fn query_manufacturers(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Manufacturer>, NetBoxError> {
        self.query_manufacturers(filters, fetch_all).await
    }

    async fn get_manufacturer_by_name(&self, name: &str) -> Result<Option<Manufacturer>, NetBoxError> {
        self.get_manufacturer_by_name(name).await
    }

    async fn create_manufacturer(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Manufacturer, NetBoxError> {
        self.create_manufacturer(name, Some(slug), description.map(|s| s.to_string())).await
    }

    async fn query_platforms(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Platform>, NetBoxError> {
        self.query_platforms(filters, fetch_all).await
    }

    async fn get_platform_by_name(&self, name: &str) -> Result<Option<Platform>, NetBoxError> {
        self.get_platform_by_name(name).await
    }

    async fn create_platform(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Platform, NetBoxError> {
        self.create_platform(name, Some(slug), None, None, None, description.map(|s| s.to_string()), None).await
    }

    async fn query_device_types(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<DeviceType>, NetBoxError> {
        self.query_device_types(filters, fetch_all).await
    }

    async fn get_device_type_by_model(&self, manufacturer_id: u64, model: &str) -> Result<Option<DeviceType>, NetBoxError> {
        self.get_device_type_by_model(manufacturer_id, model).await
    }

    async fn create_device_type(&self, manufacturer_id: u64, model: &str, slug: Option<&str>, description: Option<&str>) -> Result<DeviceType, NetBoxError> {
        self.create_device_type(manufacturer_id, model, slug, None, None, None, description.map(|s| s.to_string()), None).await
    }

    // Tenancy Operations
    async fn query_tenants(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Tenant>, NetBoxError> {
        self.query_tenants(filters, fetch_all).await
    }

    async fn get_tenant(&self, id: u64) -> Result<Tenant, NetBoxError> {
        self.get_tenant(id).await
    }

    async fn create_tenant(&self, name: &str, slug: &str, tenant_group_id: Option<u64>, description: Option<&str>, comments: Option<&str>) -> Result<Tenant, NetBoxError> {
        self.create_tenant(name, Some(slug), description.map(|s| s.to_string()), comments.map(|s| s.to_string()), tenant_group_id).await
    }

    async fn query_tenant_groups(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<TenantGroup>, NetBoxError> {
        self.query_tenant_groups(filters, fetch_all).await
    }

    async fn get_tenant_group_by_name(&self, name: &str) -> Result<Option<TenantGroup>, NetBoxError> {
        self.get_tenant_group_by_name(name).await
    }

    async fn create_tenant_group(&self, name: &str, slug: &str, description: Option<&str>) -> Result<TenantGroup, NetBoxError> {
        self.create_tenant_group(name, Some(slug), description.map(|s| s.to_string()), None, None).await
    }

    // Extras Operations
    async fn query_roles(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Role>, NetBoxError> {
        self.query_roles(filters, fetch_all).await
    }

    async fn get_role(&self, id: u64) -> Result<Role, NetBoxError> {
        self.get_role(id).await
    }

    async fn create_role(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Role, NetBoxError> {
        self.create_role(name, Some(slug), description.map(|s| s.to_string()), None, None).await
    }

    async fn query_tags(&self, filters: &[(&str, &str)], fetch_all: bool) -> Result<Vec<Tag>, NetBoxError> {
        self.query_tags(filters, fetch_all).await
    }

    async fn get_tag(&self, id: u64) -> Result<Tag, NetBoxError> {
        self.get_tag(id).await
    }

    async fn create_tag(&self, name: &str, slug: &str, description: Option<&str>) -> Result<Tag, NetBoxError> {
        self.create_tag(name, Some(slug), None, description.map(|s| s.to_string()), None).await
    }
}
