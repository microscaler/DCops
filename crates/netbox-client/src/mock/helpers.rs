//! Helper functions for creating nested NetBox model types

use crate::models::*;

/// Helper functions for creating nested types in mock implementations
pub struct Helpers {
    base_url: String,
}

impl Helpers {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    /// Helper to create NestedTenant
    pub fn create_nested_tenant(&self, id: u64, name: Option<String>) -> NestedTenant {
        let name_str = name.unwrap_or_else(|| format!("Tenant {}", id));
        NestedTenant {
            id,
            url: format!("{}/api/tenancy/tenants/{}/", self.base_url, id),
            display: name_str.clone(),
            name: name_str.clone(),
            slug: name_str.to_lowercase().replace(' ', "-"),
        }
    }
    
    /// Helper to create NestedSite
    pub fn create_nested_site(&self, id: u64, name: Option<String>) -> NestedSite {
        let name_str = name.unwrap_or_else(|| format!("Site {}", id));
        NestedSite {
            id,
            url: format!("{}/api/dcim/sites/{}/", self.base_url, id),
            display: name_str.clone(),
            name: name_str.clone(),
            slug: name_str.to_lowercase().replace(' ', "-"),
        }
    }
    
    /// Helper to create NestedRegion
    pub fn create_nested_region(&self, id: u64, name: Option<String>) -> NestedRegion {
        let name_str = name.unwrap_or_else(|| format!("Region {}", id));
        NestedRegion {
            id,
            url: format!("{}/api/dcim/regions/{}/", self.base_url, id),
            display: name_str.clone(),
            name: name_str.clone(),
            slug: name_str.to_lowercase().replace(' ', "-"),
        }
    }
    
    /// Helper to create NestedSiteGroup
    pub fn create_nested_site_group(&self, id: u64, name: Option<String>) -> NestedSiteGroup {
        let name_str = name.unwrap_or_else(|| format!("Site Group {}", id));
        NestedSiteGroup {
            id,
            url: format!("{}/api/dcim/site-groups/{}/", self.base_url, id),
            display: name_str.clone(),
            name: name_str.clone(),
            slug: name_str.to_lowercase().replace(' ', "-"),
        }
    }
    
    /// Helper to create NestedVlan
    pub fn create_nested_vlan(&self, id: u64, vid: u16, name: Option<String>) -> NestedVlan {
        let name_str = name.unwrap_or_else(|| format!("VLAN {}", vid));
        NestedVlan {
            id,
            url: format!("{}/api/ipam/vlans/{}/", self.base_url, id),
            display: name_str.clone(),
            vid,
            name: name_str,
        }
    }
    
    /// Helper to create NestedRole
    pub fn create_nested_role(&self, id: u64, name: Option<String>) -> NestedRole {
        let name_str = name.unwrap_or_else(|| format!("Role {}", id));
        NestedRole {
            id,
            url: format!("{}/api/ipam/roles/{}/", self.base_url, id),
            display: name_str.clone(),
            name: name_str.clone(),
            slug: name_str.to_lowercase().replace(' ', "-"),
        }
    }

    /// Helper to create NestedLocation
    pub fn create_nested_location(&self, id: u64, name: Option<String>) -> NestedLocation {
        let name_str = name.unwrap_or_else(|| format!("Location {}", id));
        NestedLocation {
            id,
            url: format!("{}/api/dcim/locations/{}/", self.base_url, id),
            display: name_str.clone(),
            name: name_str.clone(),
            slug: name_str.to_lowercase().replace(' ', "-"),
        }
    }

    /// Helper to create NestedTag from serde_json::Value
    pub fn create_nested_tag(&self, value: &serde_json::Value) -> Option<NestedTag> {
        value.as_str().map(|s| NestedTag {
            id: 0,
            url: format!("{}/api/extras/tags/0/", self.base_url),
            display: s.to_string(),
            name: s.to_string(),
            slug: s.to_lowercase().replace(' ', "-"),
        })
    }

    /// Helper to convert Vec<serde_json::Value> to Vec<NestedTag>
    pub fn convert_tags(&self, tags: Vec<serde_json::Value>) -> Vec<NestedTag> {
        tags.into_iter()
            .filter_map(|v| self.create_nested_tag(&v))
            .collect()
    }
}

