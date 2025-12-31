# Infrastructure Inventory

DCops manages your infrastructure inventory through NetBox.

## Sites

Sites represent physical locations:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: Site
metadata:
  name: datacenter-1
spec:
  name: "Primary Datacenter"
  region: "us-east"
  netboxRef:
    siteId: 789
```

## Devices

Devices represent physical hardware:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: Device
metadata:
  name: server-01
spec:
  siteRef:
    name: datacenter-1
  deviceType: "Server"
  netboxRef:
    deviceId: 101
```

## Reconciliation

DCops continuously reconciles:
- Git state → NetBox state
- Detects manual changes in NetBox
- Corrects drift automatically

