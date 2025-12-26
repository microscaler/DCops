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

pub async fn create_role(client: &MockNetBoxClient, name: &str, slug: &str, description: Option<&str>) -> Result<Role, NetBoxError> {
        let id = client.next_id();
        let role = Role {
            id,
            url: format!("{}/api/extras/roles/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            description: description.map(|s| s.to_string()),
            weight: None,
            comments: None,
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

pub async fn create_tag(client: &MockNetBoxClient, name: &str, slug: &str, description: Option<&str>) -> Result<Tag, NetBoxError> {
        let id = client.next_id();
        let tag = Tag {
            id,
            url: format!("{}/api/extras/tags/{}/", client.base_url, id),
            display: name.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            color: String::new(), // Tag.color is String (required field)
            description: description.map(|s| s.to_string()),
            comments: None,
            created: chrono::Utc::now().to_rfc3339(),
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        client.tags.lock().unwrap().insert(id, tag.clone());
        Ok(tag)
}
}
