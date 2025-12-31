//! Extras operations for MockNetBoxClient
//!
//! Handles roles and tags

use super::MockNetBoxClient;
use crate::error::NetBoxError;
use crate::models::*;

pub async fn query_roles(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Role>, NetBoxError> {
        let roles = client.roles.lock().unwrap();
        Ok(roles.values().cloned().collect())
}

pub async fn get_role(client: &MockNetBoxClient, id: u64) -> Result<Role, NetBoxError> {
        client.roles
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Role {} not found", id)))
}

pub async fn create_role(client: &MockNetBoxClient, name: &str, slug: Option<&str>, description: Option<String>, weight: Option<u16>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Role, NetBoxError> {
        let id = client.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let role = Role {
            id,
            url: format!("{}/api/extras/roles/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug_value,
            description,
            weight,
            comments,
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

        client.roles.lock().unwrap().insert(id, role.clone());
        Ok(role)
    }

pub async fn update_role(client: &MockNetBoxClient, id: u64, name: Option<&str>, slug: Option<&str>, description: Option<String>, weight: Option<u16>, comments: Option<String>, tags: Option<Vec<String>>) -> Result<Role, NetBoxError> {
        let mut roles = client.roles.lock().unwrap();
        let role = roles.get_mut(&id)
            .ok_or_else(|| NetBoxError::NotFound(format!("Role {} not found", id)))?;
        
        if let Some(name_val) = name {
            role.name = name_val.to_string();
            role.display = name_val.to_string();
        }
        
        if let Some(slug_val) = slug {
            role.slug = slug_val.to_string();
        }
        
        if description.is_some() {
            role.description = description;
        }
        
        if weight.is_some() {
            role.weight = weight;
        }
        
        if comments.is_some() {
            role.comments = comments;
        }
        
        if let Some(tags_vec) = tags {
            role.tags = tags_vec.into_iter().map(|t| NestedTag {
                id: 0,
                url: String::new(),
                display: t.clone(),
                name: t,
                slug: String::new(),
            }).collect();
        }
        
        role.last_updated = chrono::Utc::now().to_rfc3339();
        Ok(role.clone())
    }

pub async fn query_tags(client: &MockNetBoxClient, _filters: &[(&str, &str)], _fetch_all: bool) -> Result<Vec<Tag>, NetBoxError> {
        let tags = client.tags.lock().unwrap();
        Ok(tags.values().cloned().collect())
}

pub async fn get_tag(client: &MockNetBoxClient, id: u64) -> Result<Tag, NetBoxError> {
        client.tags
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| NetBoxError::NotFound(format!("Tag {} not found", id)))
}

pub async fn create_tag(client: &MockNetBoxClient, name: &str, slug: Option<&str>, color: Option<&str>, description: Option<String>, comments: Option<String>) -> Result<Tag, NetBoxError> {
        let id = client.next_id();
        let slug_value = slug.map(|s| s.to_string()).unwrap_or_else(|| name.to_lowercase().replace(' ', "-"));
        let tag = Tag {
            id,
            url: format!("{}/api/extras/tags/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug_value,
            color: color.map(|s| s.to_string()).unwrap_or_else(|| "9e9e9e".to_string()),
            description,
            comments,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.tags.lock().unwrap().insert(id, tag.clone());
        Ok(tag)
    }
