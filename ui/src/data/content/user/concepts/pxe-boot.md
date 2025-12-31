# PXE Boot Control

DCops provides controlled PXE boot management to prevent infinite boot loops.

## PXE Intent

A PXE Intent declares the desired boot behavior:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: PXEIntent
metadata:
  name: cluster-rebuild
spec:
  deviceRef:
    name: server-01
  profile: "talos-install"
  enabled: true
```

## Boot Profiles

Boot profiles define what should happen during PXE boot:

- **Install** - Install a new OS
- **Boot** - Boot from local disk
- **Rescue** - Boot into rescue mode

## Safety Features

- Prevents accidental reinstallations
- Controlled cluster rebuilds
- Automatic disable after completion

