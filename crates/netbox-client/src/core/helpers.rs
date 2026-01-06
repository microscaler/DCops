//! Helper functions for NetBox API request body construction
//!
//! These helpers provide DRY (Don't Repeat Yourself) utilities for building
//! JSON request bodies for NetBox API calls. They handle common patterns like
//! nested object references, optional fields, and slug generation.

use crate::error::NetBoxError;
use crate::tenancy;
use super::NetBoxClientCore;
use tracing::{debug, warn};

/// Add a nested object reference to a request body.
/// 
/// For NetBox 4.0, nested objects must be sent as `{"id": X}` to reference existing objects.
/// If `id` is `None`, the field is not added to the body (PATCH semantics - only send changed fields).
pub fn add_nested_reference(body: &mut serde_json::Value, field_name: &str, id: Option<u64>) {
    if let Some(id_value) = id {
        body[field_name] = serde_json::json!({"id": id_value});
        debug!("Adding nested reference: {}={}", field_name, id_value);
    }
}

/// Add a required nested object reference to a request body.
/// 
/// For required nested fields (e.g., `rir` in aggregates, `device` in interfaces).
pub fn add_required_nested_reference(body: &mut serde_json::Value, field_name: &str, id: u64) {
    body[field_name] = serde_json::json!({"id": id});
    debug!("Adding required nested reference: {}={}", field_name, id);
}

/// Add a nullable nested object reference to a request body.
/// 
/// For fields that must be explicitly set to `null` if not provided (e.g., `group` in tenant).
pub fn add_nullable_nested_reference(body: &mut serde_json::Value, field_name: &str, id: Option<u64>) {
    if let Some(id_value) = id {
        body[field_name] = serde_json::json!({"id": id_value});
    } else {
        body[field_name] = serde_json::Value::Null;
    }
    debug!("Adding nullable nested reference: {}={:?}", field_name, id);
}

/// Generate a slug from a name if not provided.
/// 
/// Converts name to lowercase and replaces spaces with hyphens.
pub fn generate_slug(name: &str, provided_slug: Option<&str>) -> String {
    if let Some(slug_str) = provided_slug {
        slug_str.to_string()
    } else {
        name.to_lowercase().replace(' ', "-")
    }
}

/// Add an optional string field to a request body.
/// 
/// If `value` is `None`, the field is not added (PATCH semantics).
pub fn add_optional_string_field(body: &mut serde_json::Value, field_name: &str, value: Option<&str>) {
    if let Some(val) = value {
        body[field_name] = serde_json::Value::String(val.to_string());
    }
}

/// Add an optional owned string field to a request body.
/// 
/// If `value` is `None`, the field is not added (PATCH semantics).
pub fn add_optional_string_field_owned(body: &mut serde_json::Value, field_name: &str, value: Option<String>) {
    if let Some(val) = value {
        body[field_name] = serde_json::Value::String(val);
    }
}

/// Add an optional number field to a request body.
/// 
/// If `value` is `None`, the field is not added (PATCH semantics).
pub fn add_optional_number_field<T: Into<serde_json::Number>>(body: &mut serde_json::Value, field_name: &str, value: Option<T>) {
    if let Some(val) = value {
        body[field_name] = serde_json::Value::Number(val.into());
    }
}

/// Add an optional boolean field to a request body.
/// 
/// If `value` is `None`, the field is not added (PATCH semantics).
pub fn add_optional_bool_field(body: &mut serde_json::Value, field_name: &str, value: Option<bool>) {
    if let Some(val) = value {
        body[field_name] = serde_json::Value::Bool(val);
    }
}

/// Add an optional enum/serializable field to a request body.
/// 
/// If `value` is `None`, the field is not added (PATCH semantics).
pub fn add_optional_enum_field<T: serde::Serialize>(body: &mut serde_json::Value, field_name: &str, value: Option<T>) -> Result<(), NetBoxError> {
    if let Some(val) = value {
        body[field_name] = serde_json::to_value(val)
            .map_err(|e| NetBoxError::Serialization(e))?;
    }
    Ok(())
}

/// Add optional tags field to request body.
/// 
/// Tags can be provided as:
/// - Vec<String> - tag IDs as strings (e.g., ["1", "2"])
/// - Vec<serde_json::Value> - tag IDs as numbers or dictionaries (e.g., [1, 2] or [{"name": "tag1"}])
/// 
/// Special handling:
/// - If `tags` is `Some(vec![])` (empty vector), sets `"tags": []` to clear all tags
/// - If `tags` is `None`, the field is not added (PATCH semantics - don't update tags)
pub fn add_optional_tags_field<T>(body: &mut serde_json::Value, tags: Option<T>) -> Result<(), NetBoxError>
where
    T: serde::Serialize,
{
    if let Some(tags_vec) = tags {
        let tags_value = serde_json::to_value(tags_vec)
            .map_err(|e| NetBoxError::Serialization(e))?;
        
        // Check if tags_vec is an empty array/vector
        // If it's an empty array, we need to explicitly set it to clear tags
        if tags_value.is_array() && tags_value.as_array().map(|a| a.is_empty()).unwrap_or(false) {
            body["tags"] = serde_json::json!([]);
            debug!("Adding empty tags array to request body to clear all tags: []");
        } else {
            body["tags"] = tags_value.clone();
            debug!("Adding tags field to request body: {:?}", tags_value);
        }
    } else {
        debug!("No tags provided, skipping tags field");
    }
    Ok(())
}

/// Add full tenant object to request body for CREATE operations.
/// 
/// NetBox 4.0 requires the full tenant object (id, name, slug) for CREATE operations,
/// not just {"id": X}. This function fetches the tenant and adds the full object.
/// 
/// For PATCH operations, use `add_nested_reference` instead.
pub async fn add_tenant_for_create(
    body: &mut serde_json::Value,
    core: &NetBoxClientCore,
    tenant_id: Option<u64>,  // Keep as u64 for now - will be updated when tenancy module uses TenantId
) {
    if let Some(tid) = tenant_id {
        // Fetch the full tenant object to get name and slug
        match tenancy::get_tenant(core, tid).await {
            Ok(tenant) => {
                body["tenant"] = serde_json::json!({
                    "id": tenant.id,
                    "name": tenant.name,
                    "slug": tenant.slug,
                });
                debug!("Adding full tenant object for CREATE: id={}, name={}", tenant.id, tenant.name);
            }
            Err(e) => {
                // Fall back to just ID if we can't fetch the tenant
                warn!("Failed to fetch tenant {} for CREATE, using ID only: {}", tid, e);
                body["tenant"] = serde_json::json!({"id": tid});
            }
        }
    }
}

