//! RouterOS REST API client

use crate::error::RouterOSError;
use reqwest::Client;

/// RouterOS REST API client
pub struct RouterOSClient {
    client: Client,
    base_url: String,
    username: String,
    password: String,
}

impl RouterOSClient {
    /// Create a new RouterOS client
    pub fn new(base_url: String, username: String, password: String) -> Result<Self, RouterOSError> {
        let client = Client::builder()
            .build()?;
        
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            username,
            password,
        })
    }
    
    // TODO: Implement RouterOS API operations
    // - Get/create VLAN interfaces
    // - Configure bridge VLAN tables
    // - Configure DHCP relay
    // - Query current state
}

