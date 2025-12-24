//! RouterOS REST API Client
//!
//! Client for interacting with MikroTik RouterOS/SwitchOS REST API.
//!
//! **Status:** Phase 2+ (deferred from Phase 1)

pub mod client;
pub mod models;
pub mod error;

pub use client::*;
pub use error::*;

