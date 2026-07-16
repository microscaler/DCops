# PXE Intent Controller

The PXE Intent Controller manages PXE boot behavior.

## Responsibilities

- PXE boot control
- Boot profile management
- Safety mechanisms

## PXE Intent

Controls when devices boot from PXE:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: PXEIntent
metadata:
  name: example
spec:
  deviceRef:
    name: server-01
  profile: "install"
  enabled: true
```

## Status

Check controller status:

```bash
kubectl get pxeintent
```

