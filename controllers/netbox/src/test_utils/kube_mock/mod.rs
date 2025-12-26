//! Kubernetes API mocking utilities for unit tests
//!
//! This module provides modular utilities for creating mock Kubernetes API clients
//! that can be used in unit tests without requiring a real Kubernetes cluster.
//!
//! ## Architecture
//!
//! The mocking system is organized into modular components:
//! - `service.rs`: Mock HTTP service using tower-test
//! - `store.rs`: In-memory resource store
//! - `client.rs`: Mock kube::Client wrapper
//! - `helpers.rs`: Utility functions for common test scenarios

pub mod service;
pub mod store;
pub mod client;
pub mod helpers;

pub use service::MockKubeService;
pub use store::MockKubeStore;
pub use client::create_mock_kube_client;
pub use helpers::*;

