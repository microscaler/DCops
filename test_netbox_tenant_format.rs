// Quick test script to probe NetBox API for correct tenant format
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let netbox_url = env::var("NETBOX_URL").unwrap_or_else(|_| "http://netbox.netbox:80".to_string());
    let netbox_token = env::var("NETBOX_TOKEN").expect("NETBOX_TOKEN must be set");
    
    let client = reqwest::Client::new();
    
    // First, get the current site to see its structure
    println!("=== Getting current site ===");
    let response = client
        .get(&format!("{}/api/dcim/sites/1/", netbox_url))
        .header("Authorization", format!("Token {}", netbox_token))
        .header("Accept", "application/json")
        .send()
        .await?;
    
    let site: serde_json::Value = response.json().await?;
    println!("Current tenant: {}", serde_json::to_string_pretty(&site["tenant"])?);
    println!("Current region: {}", serde_json::to_string_pretty(&site["region"])?);
    println!("Current site_group: {}", serde_json::to_string_pretty(&site["site_group"])?);
    
    let tenant_id = site["tenant"]["id"].as_u64().unwrap();
    println!("\nTenant ID: {}", tenant_id);
    
    // Test 1: Try {"id": tid} format
    println!("\n=== Test 1: Sending {{\"id\": {}}} ===", tenant_id);
    let body1 = serde_json::json!({
        "tenant": {"id": tenant_id}
    });
    let response1 = client
        .patch(&format!("{}/api/dcim/sites/1/", netbox_url))
        .header("Authorization", format!("Token {}", netbox_token))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&body1)
        .send()
        .await?;
    println!("Status: {}", response1.status());
    if !response1.status().is_success() {
        let error_text = response1.text().await?;
        println!("Error: {}", error_text);
    } else {
        println!("✅ SUCCESS with {{\"id\": {}}}", tenant_id);
        return Ok(());
    }
    
    // Test 2: Try just integer
    println!("\n=== Test 2: Sending integer {} ===", tenant_id);
    let body2 = serde_json::json!({
        "tenant": tenant_id
    });
    let response2 = client
        .patch(&format!("{}/api/dcim/sites/1/", netbox_url))
        .header("Authorization", format!("Token {}", netbox_token))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&body2)
        .send()
        .await?;
    println!("Status: {}", response2.status());
    if !response2.status().is_success() {
        let error_text = response2.text().await?;
        println!("Error: {}", error_text);
    } else {
        println!("✅ SUCCESS with integer {}", tenant_id);
        return Ok(());
    }
    
    // Test 3: Get tenant and try full object
    println!("\n=== Test 3: Getting tenant details ===");
    let tenant_response = client
        .get(&format!("{}/api/tenancy/tenants/{}/", netbox_url, tenant_id))
        .header("Authorization", format!("Token {}", netbox_token))
        .header("Accept", "application/json")
        .send()
        .await?;
    let tenant: serde_json::Value = tenant_response.json().await?;
    println!("Tenant details: {}", serde_json::to_string_pretty(&tenant)?);
    
    // Test 4: Try full object with id, name, group
    println!("\n=== Test 4: Sending full object with id, name, group ===");
    let mut tenant_obj = serde_json::json!({
        "id": tenant_id,
        "name": tenant["name"]
    });
    if let Some(group) = tenant.get("group") {
        tenant_obj["group"] = serde_json::json!({
            "id": group["id"],
            "name": group["name"]
        });
    } else {
        tenant_obj["group"] = serde_json::Value::Null;
    }
    let body4 = serde_json::json!({
        "tenant": tenant_obj
    });
    println!("Body: {}", serde_json::to_string_pretty(&body4)?);
    let response4 = client
        .patch(&format!("{}/api/dcim/sites/1/", netbox_url))
        .header("Authorization", format!("Token {}", netbox_token))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&body4)
        .send()
        .await?;
    println!("Status: {}", response4.status());
    if !response4.status().is_success() {
        let error_text = response4.text().await?;
        println!("Error: {}", error_text);
    } else {
        println!("✅ SUCCESS with full object");
        return Ok(());
    }
    
    println!("\n❌ All tests failed");
    Ok(())
}

