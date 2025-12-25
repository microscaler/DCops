//! DCops CRD Definitions
//!
//! Kubernetes Custom Resource Definitions for DCops controllers.
//! Organized by NetBox API sections:
//! - dcim/ - Data Center Infrastructure Management
//! - ipam/ - IP Address Management
//! - tenancy/ - Tenancy
//! - extras/ - Extras (tags, config contexts, etc.)

// Boot resources
pub mod boot_profile;
pub mod boot_intent;

// IPAM resources
pub mod ip_pool;
pub mod ip_claim;

// Common reference types
pub mod references;

// DCIM (Data Center Infrastructure Management)
pub mod dcim;

// IPAM (IP Address Management)
pub mod ipam;

// Tenancy
pub mod tenancy;

// Extras
pub mod extras;

// Re-exports
pub use boot_profile::*;
pub use boot_intent::*;
pub use ip_pool::*;
pub use ip_claim::*;
pub use references::*;

// Re-export all DCIM resources
pub use dcim::*;

// Re-export all IPAM resources
pub use ipam::*;

// Re-export all tenancy resources
pub use tenancy::*;

// Re-export all extras resources
pub use extras::*;
