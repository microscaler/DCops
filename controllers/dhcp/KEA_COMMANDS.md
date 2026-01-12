# Kea Control Agent Commands Reference

This document lists all Kea Control Agent commands implemented in our REST client.

## Command Categories

### Configuration Management
- ✅ `config-get` - Retrieve current configuration
- ✅ `config-set` - Apply new configuration
- ✅ `config-test` - Validate configuration without applying
- ✅ `config-reload` - Reload configuration from file
- ✅ `config-write` - Write current configuration to file
- ✅ `config-hash-get` - Get configuration hash
- ✅ `config-backend-pull` - Pull configuration from backend

### Server Control
- ✅ `shutdown` - Gracefully shutdown server
- ✅ `status-get` - Get server status
- ✅ `version-get` - Get server version
- ✅ `build-report` - Get build information
- ✅ `server-tag-get` - Get server tag

### DHCP Service Control
- ✅ `dhcp-enable` - Enable DHCP service
- ✅ `dhcp-disable` - Disable DHCP service

### Lease Management (requires `lease_cmds` hook library)
- ✅ `lease4-get` - Get IPv4 lease information
- ✅ `lease4-get-all` - Get all IPv4 leases
- ✅ `lease4-add` - Add IPv4 lease
- ✅ `lease4-del` - Delete IPv4 lease
- ✅ `lease4-wipe` - Wipe all IPv4 leases
- ✅ `lease4-update` - Update IPv4 lease

### Subnet Management (requires `subnet_cmds` hook library)
- ✅ `subnet4-add` - Add IPv4 subnet
- ✅ `subnet4-del` - Delete IPv4 subnet
- ✅ `subnet4-delta-add` - Add subnet delta (partial update)
- ✅ `subnet4-delta-del` - Delete subnet delta (partial removal)

### Reservation Management (requires `host_cmds` hook library)
- ✅ `reservation-add` - Add host reservation
- ✅ `reservation-del` - Delete host reservation
- ✅ `reservation-get` - Get host reservation
- ✅ `reservation-list` - List all host reservations

### Statistics
- ✅ `statistic-get` - Get specific statistic
- ✅ `statistic-get-all` - Get all statistics
- ✅ `statistic-global-get-all` - Get all global statistics
- ✅ `statistic-reset` - Reset specific statistic
- ✅ `statistic-reset-all` - Reset all statistics
- ✅ `statistic-remove` - Remove specific statistic
- ✅ `statistic-remove-all` - Remove all statistics
- ✅ `statistic-sample-age-set` - Set statistic sample age
- ✅ `statistic-sample-count-set` - Set statistic sample count

### Utility Commands
- ✅ `leases-reclaim` - Reclaim expired leases
- ✅ `subnet4-select-test` - Test subnet selection
- ✅ `kea-lfc-start` - Start lease file cleanup

## Usage Examples

### Configuration Management
```rust
let kea_client = KeaClient::new("http://localhost:8000".to_string());

// Get current configuration
let config = kea_client.commands().config_get().await?;

// Test configuration
kea_client.commands().config_test(&new_config).await?;

// Apply configuration
kea_client.commands().config_set(&new_config).await?;
```

### Lease Management
```rust
// Get lease information
let lease = kea_client.commands().lease4_get("192.168.1.100").await?;

// Add lease
let new_lease = json!({
    "ip-address": "192.168.1.100",
    "hw-address": "aa:bb:cc:dd:ee:ff",
    "subnet-id": 1
});
kea_client.commands().lease4_add(&new_lease).await?;
```

### Subnet Management
```rust
// Add subnet
let subnet = json!({
    "subnet": "192.168.1.0/24",
    "id": 1,
    "pools": [{"pool": "192.168.1.100-192.168.1.200"}]
});
kea_client.commands().subnet4_add(&subnet).await?;
```

### Statistics
```rust
// Get all statistics
let stats = kea_client.commands().statistic_get_all().await?;

// Get specific statistic
let leases = kea_client.commands().statistic_get("cumulative-assigned-addresses").await?;
```

## Hook Library Requirements

Some commands require specific hook libraries to be loaded:

- **Lease Commands**: `lease_cmds` hook library
- **Subnet Commands**: `subnet_cmds` hook library  
- **Reservation Commands**: `host_cmds` hook library

These hook libraries must be configured in the Kea configuration file for the commands to be available.

## Notes

- All commands are sent to the `dhcp4` service by default
- Commands that modify state (config-set, lease4-add, etc.) log at `info!` level
- Error handling is consistent across all commands via `ControllerError`
- The API client automatically checks Kea response codes and returns appropriate errors

