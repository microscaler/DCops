# Architecture

DCops is built with Rust and follows Kubernetes controller patterns.

## Components

- **NetBox Controller** - Manages IPAM and inventory
- **PXE Intent Controller** - Controls PXE boot
- **RouterOS Controller** - Manages RouterOS devices

## Design Principles

- **GitOps First** - Git is the source of truth
- **Idempotent Operations** - Safe to run repeatedly
- **Drift Detection** - Automatic correction
- **Multi-Tenant** - Support for shared infrastructure

## Technology Stack

- **Rust** - High-performance, memory-safe controllers
- **Kubernetes** - Controller runtime
- **NetBox** - IPAM and inventory backend

