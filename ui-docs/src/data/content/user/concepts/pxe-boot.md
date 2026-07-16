# PXE Boot Control

DCops provides controlled PXE boot management to prevent infinite boot loops and enable safe cluster rebuilds.

> **⚠️ Status: Coming Soon**
> 
> PXE boot control is planned for future implementation. The CRDs (`BootProfile` and `BootIntent`) are defined but the controller is not yet implemented.

## Overview

PXE boot control allows you to:
- Define boot profiles declaratively
- Control when devices boot from PXE
- Prevent infinite boot loops
- Enable safe cluster rebuilds

## Planned Features

### BootProfile

Defines a PXE boot profile configuration:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: BootProfile
metadata:
  name: talos-install
spec:
  name: "Talos Linux Installation"
  description: "PXE boot profile for installing Talos Linux"
  # Profile configuration will be defined here
```

### BootIntent

Declares the desired boot behavior for a device:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: BootIntent
metadata:
  name: cluster-rebuild
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

## Planned Boot Profiles

Boot profiles will define what happens during PXE boot:

- **Install** - Install a new OS
- **Boot** - Boot from local disk
- **Rescue** - Boot into rescue mode
- **Custom** - Custom boot configuration

## Planned Safety Features

- **Prevent Infinite Loops** - Automatically disable PXE after successful boot
- **Controlled Rebuilds** - Enable PXE only when explicitly requested
- **Status Tracking** - Track boot state and completion
- **Automatic Disable** - Disable PXE after installation completes

## Current Status

The PXE Intent Controller is currently a stub implementation. The CRDs are defined but not yet functional.

**What Works:**
- CRDs can be created
- CRDs are validated

**What Doesn't Work:**
- No reconciliation logic
- No PXE server integration
- No boot control

## Future Implementation

When implemented, the PXE controller will:
1. Watch `BootIntent` and `BootProfile` CRDs
2. Configure PXE server based on intents
3. Monitor boot completion
4. Automatically disable PXE after successful boot
5. Emit events for boot state changes

## Related Resources

- [PXE Configuration Guide](../guides/pxe-configuration.md) - Configuration details (when implemented)
- [Infrastructure Inventory](./infrastructure-inventory.md) - Device management
- [CRD Reference](../api-reference/crd-reference.md) - Complete CRD documentation
