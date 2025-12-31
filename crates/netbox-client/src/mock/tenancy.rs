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

pub async fn create_tenant(client: &MockNetBoxClient, name: &str, slug: Option<&str>, description: Option<String>, comments: Option<String>, group: Option<u64>, tags: Option<Vec<String>>) -> Result<Tenant, NetBoxError> {
        let id = client.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let tenant = Tenant {
            id,
            url: format!("{}/api/tenancy/tenants/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug_value,
            description,
            comments,
            group: group.map(|id| NestedTenantGroup {
                id,
                url: format!("{}/api/tenancy/tenant-groups/{}/", client.base_url, id),
                display: format!("Tenant Group {}", id),
                name: format!("Tenant Group {}", id),
                slug: format!("tenant-group-{}", id),
            }),
            tags: tags.unwrap_or_default().into_iter().map(|t| NestedTag {
                id: 0,
                url: String::new(),
                display: t.clone(),
                name: t,
                slug: String::new(),
            }).collect(),
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.tenants.lock().unwrap().insert(id, tenant.clone());
        Ok(tenant)
    }

pub async fn update_tenant(client: &MockNetBoxClient, id: u64, name: Option<&str>, slug: Option<&str>, description: Option<String>, comments: Option<String>, group: Option<u64>, tags: Option<Vec<String>>) -> Result<Tenant, NetBoxError> {
        let mut tenants = client.tenants.lock().unwrap();
        let tenant = tenants.get_mut(&id)
            .ok_or_else(|| NetBoxError::NotFound(format!("Tenant {} not found", id)))?;
        
        if let Some(name_val) = name {
            tenant.name = name_val.to_string();
            tenant.display = name_val.to_string();
        }
        
        if let Some(slug_val) = slug {
            tenant.slug = slug_val.to_string();
        }
        
        if description.is_some() {
            tenant.description = description;
        }
        
        if comments.is_some() {
            tenant.comments = comments;
        }
        
        if let Some(group_id) = group {
            tenant.group = Some(NestedTenantGroup {
                id: group_id,
                url: format!("{}/api/tenancy/tenant-groups/{}/", client.base_url, group_id),
                display: format!("Tenant Group {}", group_id),
                name: format!("Tenant Group {}", group_id),
                slug: format!("tenant-group-{}", group_id),
            });
        }
        
        if let Some(tags_vec) = tags {
            tenant.tags = tags_vec.into_iter().map(|t| NestedTag {
                id: 0,
                url: String::new(),
                display: t.clone(),
                name: t,
                slug: String::new(),
            }).collect();
        }
        
        tenant.last_updated = chrono::Utc::now().to_rfc3339();
        Ok(tenant.clone())
    }

pub async fn query_tenant_groups(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<TenantGroup>, NetBoxError> {
        let tenant_groups = client.tenant_groups.lock().unwrap();
        Ok(tenant_groups.values().cloned().collect())
}

pub async fn get_tenant_group_by_name(client: &MockNetBoxClient, name: &str) -> Result<Option<TenantGroup>, NetBoxError> {
        Ok(client.tenant_groups.lock().unwrap().get(name).cloned())
}

pub async fn create_tenant_group(client: &MockNetBoxClient, name: &str, slug: Option<&str>, description: Option<String>, comments: Option<String>, parent_id: Option<u64>, tags: Option<Vec<String>>) -> Result<TenantGroup, NetBoxError> {
        let id = client.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let tenant_group = TenantGroup {
            id,
            url: format!("{}/api/tenancy/tenant-groups/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug_value,
            description,
            comments,
            parent: parent_id.map(|id| client.helpers().create_nested_tenant_group(id, None)),
            tenant_count: 0,
            _depth: None,
            tags: tags.unwrap_or_default().into_iter().map(|t| NestedTag {
                id: 0,
                url: String::new(),
                display: t.clone(),
                name: t,
                slug: String::new(),
            }).collect(),
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.tenant_groups.lock().unwrap().insert(name.to_string(), tenant_group.clone());
        Ok(tenant_group)
    }

pub async fn update_tenant_group(client: &MockNetBoxClient, id: u64, name: Option<&str>, slug: Option<&str>, description: Option<String>, comments: Option<String>, parent_id: Option<u64>, tags: Option<Vec<String>>) -> Result<TenantGroup, NetBoxError> {
        let tenant_groups = client.tenant_groups.lock().unwrap();
        let tenant_group = tenant_groups.values().find(|tg| tg.id == id)
            .ok_or_else(|| NetBoxError::NotFound(format!("Tenant group {} not found", id)))?;
        let mut updated = tenant_group.clone();
        
        if let Some(name_val) = name {
            updated.name = name_val.to_string();
            updated.display = name_val.to_string();
        }
        
        if let Some(slug_val) = slug {
            updated.slug = slug_val.to_string();
        }
        
        if description.is_some() {
            updated.description = description;
        }
        
        if comments.is_some() {
            updated.comments = comments;
        }
        
        if let Some(parent) = parent_id {
            updated.parent = Some(client.helpers().create_nested_tenant_group(parent, None));
        }
        
        if let Some(tags_vec) = tags {
            updated.tags = tags_vec.into_iter().map(|t| NestedTag {
                id: 0,
                url: String::new(),
                display: t.clone(),
                name: t,
                slug: String::new(),
            }).collect();
        }
        
        updated.last_updated = chrono::Utc::now().to_rfc3339();
        drop(tenant_groups);
        client.tenant_groups.lock().unwrap().insert(updated.name.clone(), updated.clone());
        Ok(updated)
}
