//! ISC Kea Control Agent API client module
//!
//! This module provides a client for interacting with ISC Kea's Control Agent REST API.
//! The Control Agent is the interface for configuring and managing Kea DHCP servers.

mod client;
mod api;
mod commands;

pub use client::KeaClient;

