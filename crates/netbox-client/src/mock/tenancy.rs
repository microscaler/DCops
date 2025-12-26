//! Tenancy operations for MockNetBoxClient
//!
//! Handles tenants and tenant groups

use super::MockNetBoxClient;
use crate::error::NetBoxError;
use crate::models::*;

pub async fn query_tenants(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Tenant>, NetBoxError> {
        let tenants = client.tenants.lock().unwrap();
        Ok(tenants.values().cloned().collect())
}

pub async fn get_tenant(client: &MockNetBoxClient, id: u64) -> Result<Tenant, NetBoxError> {
        client.tenants
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Tenant {} not found", id)))
}

pub async fn create_tenant(client: &MockNetBoxClient, name: &str, slug: &str, tenant_group_id: Option<u64>, description: Option<&str>, comments: Option<&str>) -> Result<Tenant, NetBoxError> {
        let id = client.next_id();
        let tenant = Tenant {
            id,
            url: format!("{}/api/tenancy/tenants/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            description: description.map(|s| s.to_string()),
            comments: comments.map(|s| s.to_string()),
            group: tenant_group_id.map(|id| NestedTenantGroup {
                id,
                url: format!("{}/api/tenancy/tenant-groups/{}/", client.base_url, id),
                display: format!("Tenant Group {}", id),
                name: format!("Tenant Group {}", id),
                slug: format!("tenant-group-{}", id),
            }),
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.tenants.lock().unwrap().insert(id, tenant.clone());
        Ok(tenant)
}

pub async fn query_tenant_groups(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<TenantGroup>, NetBoxError> {
        let tenant_groups = client.tenant_groups.lock().unwrap();
        Ok(tenant_groups.values().cloned().collect())
}

pub async fn get_tenant_group_by_name(client: &MockNetBoxClient, name: &str) -> Result<Option<TenantGroup>, NetBoxError> {
        Ok(client.tenant_groups.lock().unwrap().get(name).cloned())
}

pub async fn create_tenant_group(client: &MockNetBoxClient, name: &str, slug: &str, description: Option<&str>) -> Result<TenantGroup, NetBoxError> {
        let id = client.next_id();
        let tenant_group = TenantGroup {
            id,
            url: format!("{}/api/tenancy/tenant-groups/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            description: description.map(|s| s.to_string()),
            comments: None,
            parent: None,
            tenant_count: 0,
            _depth: None,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.tenant_groups.lock().unwrap().insert(name.to_string(), tenant_group.clone());
        Ok(tenant_group)
}

    // Extras Operations
