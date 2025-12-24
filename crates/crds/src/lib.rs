//! DCops CRD Definitions
//!
//! Kubernetes Custom Resource Definitions for DCops controllers.

pub mod boot_profile;
pub mod boot_intent;
pub mod ip_pool;
pub mod ip_claim;

pub use boot_profile::*;
pub use boot_intent::*;
pub use ip_pool::*;
pub use ip_claim::*;

