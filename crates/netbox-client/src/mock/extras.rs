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

pub async fn create_role(client: &MockNetBoxClient, name: &str, slug: Option<&str>, description: Option<String>, weight: Option<u16>, comments: Option<String>) -> Result<Role, NetBoxError> {
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
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.roles.lock().unwrap().insert(id, role.clone());
        Ok(role)
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
