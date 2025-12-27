// Test script to probe NetBox API for correct tenant format
use netbox_client::NetBoxClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let netbox_url = env::var("NETBOX_URL").unwrap_or_else(|_| "http://netbox.netbox:80".to_string());
    let netbox_token = env::var("NETBOX_TOKEN").expect("NETBOX_TOKEN must be set");
    
    let client = NetBoxClient::new(netbox_url.clone(), netbox_token.clone())?;
    
    // Get current site
    println!("=== Getting current site ===");
    let site = client.get_site(1).await?;
    println!("Site: id={}, name={}", site.id, site.name);
    println!("Current tenant: {:?}", site.tenant);
    
    let tenant_id = site.tenant.as_ref().map(|t| t.id);
    
    if let Some(tid) = tenant_id {
        println!("\nSite already has tenant ID: {}", tid);
    } else {
        println!("\nSite has no tenant - testing setting tenant ID 1");
        
        // Get tenant details
        let tenant = client.get_tenant(1).await?;
        println!("Tenant: id={}, name={}, group={:?}", tenant.id, tenant.name, tenant.group);
        
        // Test with reqwest directly to see what works
        use reqwest::Client;
        let http_client = Client::new();
        
        // Test 1: Just {"id": 1}
        println!("\n=== Test 1: {{\"id\": 1}} ===");
        let body1 = serde_json::json!({"tenant": {"id": 1}});
        let resp1 = http_client
            .patch(&format!("{}/api/dcim/sites/1/", netbox_url))
            .header("Authorization", format!("Token {}", netbox_token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body1)
            .send()
            .await?;
        println!("Status: {}", resp1.status());
        if !resp1.status().is_success() {
            println!("Error: {}", resp1.text().await?);
        } else {
            println!("✅ SUCCESS!");
            return Ok(());
        }
        
        // Test 2: Full object with id, name, slug, group
        println!("\n=== Test 2: Full object with id, name, slug, group ===");
        let mut tenant_obj = serde_json::json!({
            "id": tenant.id,
            "name": tenant.name,
            "slug": tenant.slug,
        });
        if let Some(group) = tenant.group {
            tenant_obj["group"] = serde_json::json!({
                "id": group.id,
                "name": group.name,
            });
        } else {
            tenant_obj["group"] = serde_json::Value::Null;
        }
        let body2 = serde_json::json!({"tenant": tenant_obj});
        println!("Body: {}", serde_json::to_string_pretty(&body2)?);
        let resp2 = http_client
            .patch(&format!("{}/api/dcim/sites/1/", netbox_url))
            .header("Authorization", format!("Token {}", netbox_token))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body2)
            .send()
            .await?;
        println!("Status: {}", resp2.status());
        if !resp2.status().is_success() {
            println!("Error: {}", resp2.text().await?);
        } else {
            println!("✅ SUCCESS!");
            return Ok(());
        }
    }
    
    Ok(())
}

