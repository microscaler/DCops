// Test script to probe NetBox API for correct tenant format in PATCH updates
use netbox_client::NetBoxClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let netbox_url = env::var("NETBOX_URL").unwrap_or_else(|_| "http://netbox.netbox:80".to_string());
    let netbox_token = env::var("NETBOX_TOKEN").expect("NETBOX_TOKEN must be set");
    
    let client = NetBoxClient::new(netbox_url, netbox_token)?;
    
    // Get current site
    println!("=== Getting current site ===");
    let site = client.get_site(1).await?;
    println!("Site ID: 1, Name: {}", site.name);
    println!("Current tenant: {:?}", site.tenant);
    println!("Current region: {:?}", site.region);
    println!("Current site_group: {:?}", site.site_group);
    
    let tenant_id = site.tenant.as_ref().map(|t| t.id);
    if let Some(tid) = tenant_id {
        println!("\nTenant ID: {}", tid);
        
        // Get tenant details
        println!("\n=== Getting tenant details ===");
        let tenant = client.get_tenant(tid).await?;
        println!("Tenant: id={}, name={}, group={:?}", tenant.id, tenant.name, tenant.group);
        
        // Test different formats by calling update_site with different tenant formats
        // We'll modify the client code temporarily or use reqwest directly
        
        println!("\n=== Testing different formats ===");
        println!("Note: We'll test by looking at what the actual NetBox API accepts");
        println!("The current code sends: {{\"id\": {}}}", tid);
        println!("But NetBox says it needs name and group");
        println!("However, when we send full object, it says 'tenant already exists'");
        println!("\nThis suggests NetBox 4.0 has a bug or inconsistency.");
        println!("Let's check what region/site_group format looks like when we GET the site...");
    }
    
    Ok(())
}

