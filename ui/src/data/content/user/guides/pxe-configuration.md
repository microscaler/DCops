# PXE Configuration

Configure PXE boot behavior with DCops.

## PXE Intent

Control when devices boot from PXE:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: PXEIntent
metadata:
  name: install-cluster
spec:
  deviceRef:
    name: server-01
  profile: "talos-install"
  enabled: true
```

## Boot Profiles

Define boot profiles in your PXE server configuration.

## Safety

- Automatic disable after completion
- Prevents infinite boot loops
- Safe cluster rebuilds

