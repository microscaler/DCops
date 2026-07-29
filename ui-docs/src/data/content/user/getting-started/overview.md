# Overview

DCops is a Kubernetes-native, GitOps-driven infrastructure management system for datacenters.

## Key Features

- **IP Address Management** - Automated IP allocation from NetBox
- **Infrastructure Inventory** - Manage sites, devices, and network topology
- **PXE Boot Control** - Declarative boot profile management
- **GitOps Workflow** - All infrastructure defined in Git

## Architecture

DCops consists of several Kubernetes controllers:

- **NetBox Controller** - Manages IP addresses, sites, and inventory
- **PXE Intent Controller** - Controls PXE boot behavior
- **RouterOS Controller** - Manages RouterOS devices

## Next Steps

- [Installation Guide](./installation.md)
- [Quick Start](./quick-start.md)

