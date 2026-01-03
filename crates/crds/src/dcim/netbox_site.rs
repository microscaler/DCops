//! NetBoxSite Custom Resource Definition
//!
//! Defines a Kubernetes CRD for managing NetBox sites.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::references::NetBoxResourceReference;

/// NetBoxSiteSpec defines the desired state of a NetBox site
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "dcops.microscaler.io",
    version = "v1alpha1",
    kind = "NetBoxSite",
    namespaced,
    status = "NetBoxSiteStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxSiteSpec {
    /// Site name
    pub name: String,
    
    /// Site slug (optional, auto-generated from name if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    
    /// Description of the site
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Physical address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_address: Option<String>,
    
    /// Shipping address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipping_address: Option<String>,
    
    /// Latitude
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    
    /// Longitude
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    
    /// Tenant reference (references NetBoxTenant CRD, required)
    /// Tenant is required in NetBox for proper resource organization and access control
    pub tenant: NetBoxResourceReference,
    
    /// Region reference (references NetBoxRegion CRD, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<NetBoxResourceReference>,
    
    /// Site group reference (references NetBoxSiteGroup CRD, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_group: Option<NetBoxResourceReference>,
    
    /// Status (active, planned, retired, staging)
    #[serde(default = "default_site_status")]
    pub status: SiteStatus,
    
    /// Facility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facility: Option<String>,
    
    /// Time zone
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    
    /// Comments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
    
    /// Tag references (references NetBoxTag CRDs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<NetBoxResourceReference>>,
    
    /// Enable drift detection (default: true)
    /// 
    /// When enabled, the reconciler will detect and correct any changes made to the resource
    /// in the NetBox UI that differ from the CRD spec. Git is the source of truth.
    /// Set to false to disable drift detection (not recommended for GitOps workflows).
    #[serde(skip_serializing_if = "Option::is_none", default = "default_drift_detection")]
    pub drift_detection: Option<bool>,
}

fn default_drift_detection() -> Option<bool> {
    Some(true)
}

fn default_site_status() -> SiteStatus {
    SiteStatus::Active
}

/// Site status in NetBox
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum SiteStatus {
    Active,
    Planned,
    Retired,
    Staging,
}

/// NetBoxSiteStatus defines the observed state of a NetBox site
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxSiteStatus {
    /// NetBox site ID (set after creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_id: Option<u64>,
    
    /// NetBox site URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub netbox_url: Option<String>,
    
    /// Current state of the site
    pub state: crate::tenancy::netbox_tenant::ResourceState,
    
    /// Error message if reconciliation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Last reconciliation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reconciled: Option<chrono::DateTime<chrono::Utc>>,
}

