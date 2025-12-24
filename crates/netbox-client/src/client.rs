//! NetBox API client
//!
//! Implements the NetBox REST API client for IPAM operations.
//! Based on NetBox API structure: /api/ipam/prefixes/ and /api/ipam/ip-addresses/

use crate::error::NetBoxError;
use crate::models::*;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// NetBox API client
pub struct NetBoxClient {
    client: Client,
    base_url: String,
    token: String,
    max_retries: u32,
    retry_delay: Duration,
}

impl Default for NetBoxClient {
    fn default() -> Self {
        Self {
            client: Client::builder().build().unwrap(),
            base_url: String::new(),
            token: String::new(),
            max_retries: 3,
            retry_delay: Duration::from_millis(500),
        }
    }
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
            max_retries: 3,
            retry_delay: Duration::from_millis(500),
        })
    }
    
    /// Create a new NetBox client with custom retry settings
    ///
    /// # Arguments
    /// * `base_url` - NetBox base URL (e.g., "http://netbox:80")
    /// * `token` - API token for authentication
    /// * `max_retries` - Maximum number of retry attempts
    /// * `retry_delay` - Initial delay between retries (exponential backoff)
    pub fn with_retry(
        base_url: String,
        token: String,
        max_retries: u32,
        retry_delay: Duration,
    ) -> Result<Self, NetBoxError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| NetBoxError::Http(e))?;
        
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            max_retries,
            retry_delay,
        })
    }
    
    /// Execute a request with retry logic
    async fn execute_with_retry<F, Fut, T>(&self, mut f: F) -> Result<T, NetBoxError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, NetBoxError>>,
    {
        let mut last_error = None;
        
        for attempt in 0..=self.max_retries {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    // Don't retry on client errors (4xx) except 429 (rate limit)
                    if let NetBoxError::Http(ref reqwest_err) = e {
                        if let Some(status) = reqwest_err.status() {
                            if status.is_client_error() && status != 429 {
                                return Err(e);
                            }
                        }
                    }
                    
                    last_error = Some(e);
                    
                    if attempt < self.max_retries {
                        let delay = self.retry_delay * 2_u32.pow(attempt);
                        warn!("Request failed, retrying in {:?} (attempt {}/{})", delay, attempt + 1, self.max_retries);
                        sleep(delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
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
            
            let page: PaginatedResponse<T> = response.json().await?;
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
}
