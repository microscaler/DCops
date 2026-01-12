# DHCP Controller Configuration

## Environment Variables

### `KEA_CONTROL_AGENT_URL`
The URL of the ISC Kea Control Agent REST API.

**Default**: `http://localhost:8000`

**Configuration Options**:
- **Same namespace**: `http://kea-dhcp.dcops-system:8000`
- **Different namespace**: `http://kea-dhcp.kea:8000` (if Kea is in `kea` namespace)
- **Port-forward (local dev)**: `http://localhost:8000` (requires `kubectl port-forward`)

**Note**: The controller will log warnings if Kea is unavailable but will continue running and retry on CRD changes.

### `WATCH_NAMESPACE`
The Kubernetes namespace to watch for NetBox CRDs.

**Default**: `default`

**Options**:
- Set to a specific namespace to watch only that namespace
- Leave unset or empty to watch all namespaces (requires ClusterRole)

## Kea Deployment

The DHCP controller requires an ISC Kea DHCP server with Control Agent enabled.

### Quick Start (for testing)

If Kea is not yet deployed, you can:

1. **Use port-forward for local testing**:
   ```bash
   # If Kea is deployed elsewhere
   kubectl port-forward -n kea svc/kea-dhcp 8000:8000
   ```
   Then set `KEA_CONTROL_AGENT_URL=http://localhost:8000` in the deployment.

2. **Deploy Kea separately**:
   - Deploy ISC Kea with Control Agent enabled
   - Update `KEA_CONTROL_AGENT_URL` to point to the Kea service
   - The controller will automatically connect when Kea becomes available

### Controller Behavior

The controller handles Kea unavailability gracefully:
- **Startup**: Logs a warning if Kea is unavailable, but continues running
- **Reconciliation**: Logs warnings on sync failures, but doesn't fail reconciliation
- **Retry**: Automatically retries when CRDs change or on periodic requeue

This allows the controller to start before Kea is deployed and automatically sync when Kea becomes available.

