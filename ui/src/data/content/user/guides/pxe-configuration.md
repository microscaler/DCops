# PXE Configuration

Configure PXE boot behavior with DCops.

> **⚠️ Status: Coming Soon**
> 
> PXE boot control is planned for future implementation. This guide will be updated when the feature is available.

## Overview

When implemented, PXE configuration will allow you to:
- Define boot profiles declaratively
- Control PXE boot behavior per device
- Prevent infinite boot loops
- Enable safe cluster rebuilds

## Planned Features

### Boot Profiles

Define reusable boot profiles:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: BootProfile
metadata:
  name: talos-install
spec:
  name: "Talos Linux Installation"
  # Profile configuration will be defined here
```

### Boot Intents

Control boot behavior for specific devices:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: BootIntent
metadata:
  name: install-cluster
spec:
  deviceRef:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxDevice"
    name: "server-01"
  profile:
    apiGroup: "dcops.microscaler.io"
    kind: "BootProfile"
    name: "talos-install"
  enabled: true
```

## Current Status

The PXE Intent Controller is not yet implemented. The CRDs are defined but reconciliation logic is not functional.

## Related Resources

- [PXE Boot Control](../concepts/pxe-boot.md) - Conceptual overview
- [Infrastructure Inventory](../concepts/infrastructure-inventory.md) - Device management
- [CRD Reference](../api-reference/crd-reference.md) - Complete CRD documentation
