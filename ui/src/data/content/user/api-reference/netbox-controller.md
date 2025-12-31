# NetBox Controller

The NetBox Controller manages IP addresses and infrastructure inventory.

## Responsibilities

- IP address allocation
- Site management
- Device inventory
- Network topology

## Reconciliation

The controller continuously reconciles:
- Git state → NetBox state
- Detects and corrects drift
- Maintains consistency

## Status

Check controller status:

```bash
kubectl get netboxcontroller
```

