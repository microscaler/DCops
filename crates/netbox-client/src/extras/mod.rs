//! Extras module for NetBox API client
//!
//! This module re-exports client methods for Extras-related resources.

pub mod role;
pub mod tag;

// Re-export functions
pub use role::{query_roles, get_role, create_role};
pub use tag::{query_tags, get_tag, create_tag};

