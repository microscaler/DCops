//! Integration tests for NetBox client
//!
//! These tests require a running NetBox instance.
//! Set NETBOX_URL and NETBOX_TOKEN environment variables to run.

use netbox_client::{NetBoxClient, AllocateIPRequest, IPAddressStatus};

#[tokio::test]
#[ignore] // Requires running NetBox instance
async fn test_client_creation() {
    let url = std::env::var("NETBOX_URL")
        .unwrap_or_else(|_| "http://localhost:8001".to_string());
    let token = std::env::var("NETBOX_TOKEN")
        .expect("NETBOX_TOKEN environment variable must be set");
    
    let client = NetBoxClient::new(url, token).expect("Failed to create client");
    
    // Test basic API connectivity
    let prefixes = client.query_prefixes(&[], false).await;
    assert!(prefixes.is_ok(), "Failed to query prefixes");
}

#[tokio::test]
#[ignore]
async fn test_query_prefixes() {
    let url = std::env::var("NETBOX_URL")
        .unwrap_or_else(|_| "http://localhost:8001".to_string());
    let token = std::env::var("NETBOX_TOKEN")
        .expect("NETBOX_TOKEN environment variable must be set");
    
    let client = NetBoxClient::new(url, token).expect("Failed to create client");
    
    // Query all prefixes
    let prefixes = client.query_prefixes(&[], false).await
        .expect("Failed to query prefixes");
    
    println!("Found {} prefixes", prefixes.len());
}

#[tokio::test]
#[ignore]
async fn test_query_ip_addresses() {
    let url = std::env::var("NETBOX_URL")
        .unwrap_or_else(|_| "http://localhost:8001".to_string());
    let token = std::env::var("NETBOX_TOKEN")
        .expect("NETBOX_TOKEN environment variable must be set");
    
    let client = NetBoxClient::new(url, token).expect("Failed to create client");
    
    // Query all IP addresses
    let ips = client.query_ip_addresses(&[], false).await
        .expect("Failed to query IP addresses");
    
    println!("Found {} IP addresses", ips.len());
}

#[tokio::test]
#[ignore]
async fn test_create_and_delete_ip() {
    let url = std::env::var("NETBOX_URL")
        .unwrap_or_else(|_| "http://localhost:8001".to_string());
    let token = std::env::var("NETBOX_TOKEN")
        .expect("NETBOX_TOKEN environment variable must be set");
    
    let client = NetBoxClient::new(url, token).expect("Failed to create client");
    
    // Create an IP address
    let request = AllocateIPRequest {
        address: Some("192.168.100.1/24".to_string()),
        description: Some("Test IP address".to_string()),
        status: Some(IPAddressStatus::Active),
        role: None,
        dns_name: None,
        tags: None,
    };
    
    let ip = client.create_ip_address("192.168.100.1/24", Some(request)).await;
    
    if let Ok(ip) = ip {
        println!("Created IP address: {}", ip.address);
        
        // Clean up: delete the IP address
        let _ = client.delete_ip_address(ip.id).await;
    }
}

#[tokio::test]
#[ignore]
async fn test_query_vlans() {
    let url = std::env::var("NETBOX_URL")
        .unwrap_or_else(|_| "http://localhost:8001".to_string());
    let token = std::env::var("NETBOX_TOKEN")
        .expect("NETBOX_TOKEN environment variable must be set");
    
    let client = NetBoxClient::new(url, token).expect("Failed to create client");
    
    // Query all VLANs
    let vlans = client.query_vlans(&[], false).await
        .expect("Failed to query VLANs");
    
    println!("Found {} VLANs", vlans.len());
}

