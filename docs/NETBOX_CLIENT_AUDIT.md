# NetBox Client Audit - Create/Update/Delete Methods

**Date:** 2025-12-27  
**Purpose:** Comprehensive audit of all create, update, and delete methods to identify required helpers

## Summary

- **Total Methods:** 27
  - **Create:** 20 methods
  - **Update:** 5 methods
  - **Delete:** 1 method
  - **Missing Update Methods:** 1 (update_location - not implemented)

## Helper Functions Needed

### 1. `add_nested_reference` ✅ (IMPLEMENTED)
- **Purpose:** Serialize nested object references as `{"id": X}` for NetBox 4.0 API
- **Status:** ✅ Implemented and used in 9 methods
- **Needed By:** All methods that reference nested objects (tenant, site, region, etc.)

### 2. `generate_slug` (NOT IMPLEMENTED)
- **Purpose:** Auto-generate slug from name if not provided
- **Pattern:** `name.to_lowercase().replace(' ', "-")`
- **Status:** ❌ Not implemented - duplicated in 12 methods
- **Needed By:** All create methods with optional slug parameter

### 3. `add_optional_string_field` (NOT IMPLEMENTED)
- **Purpose:** Conditionally add optional string fields to JSON body
- **Pattern:** `if let Some(value) = field { body["field"] = serde_json::Value::String(value); }`
- **Status:** ❌ Not implemented - duplicated in all methods
- **Needed By:** All create/update methods

### 4. `add_optional_number_field` (NOT IMPLEMENTED)
- **Purpose:** Conditionally add optional number fields to JSON body
- **Status:** ❌ Not implemented - duplicated in multiple methods
- **Needed By:** Methods with optional numeric fields

### 5. `add_optional_bool_field` (NOT IMPLEMENTED)
- **Purpose:** Conditionally add optional boolean fields to JSON body
- **Status:** ❌ Not implemented - duplicated in multiple methods
- **Needed By:** Methods with optional boolean fields

### 6. `handle_nullable_nested_reference` (NOT IMPLEMENTED)
- **Purpose:** Handle nested references that can be null (e.g., `group` in tenant)
- **Pattern:** `if let Some(id) = id { body["field"] = json!({"id": id}) } else { body["field"] = json!(null) }`
- **Status:** ❌ Not implemented - special case handling in some methods
- **Needed By:** Methods where null is explicitly required (e.g., `create_tenant` with `group`)

## Detailed Method Audit

### IPAM Operations

| Method | Type | Nested References | Uses Helper | Slug Gen | Other Helpers Needed |
|--------|------|-------------------|-------------|----------|---------------------|
| `create_ip_address` | Create | None | ❌ | ❌ | `add_optional_string_field`, `add_optional_enum_field` |
| `update_ip_address` | Update | None | ❌ | ❌ | `add_optional_string_field`, `add_optional_enum_field` |
| `delete_ip_address` | Delete | None | ❌ | ❌ | None (simple DELETE) |
| `create_prefix` | Create | `site`, `vlan`, `role`, `tenant` | ✅ | ❌ | `add_optional_string_field` |
| `update_prefix` | Update | `site`, `vlan`, `tenant` | ✅ | ❌ | `add_optional_string_field` |
| `create_aggregate` | Create | `rir` (required, not using helper) | ❌ | ❌ | `add_optional_string_field`, `validate_required` |
| `create_rir` | Create | None | ❌ | ✅ | `add_optional_string_field` |
| `create_vlan` | Create | `site`, `group`, `tenant`, `role` | ✅ | ❌ | `add_optional_string_field` |
| `update_vlan` | Update | `site`, `group`, `tenant`, `role` | ✅ | ❌ | `add_optional_string_field` |

### Tenancy Operations

| Method | Type | Nested References | Uses Helper | Slug Gen | Other Helpers Needed |
|--------|------|-------------------|-------------|----------|---------------------|
| `create_tenant` | Create | `group` (nullable - special case) | ❌ | ✅ | `handle_nullable_nested_reference`, `add_optional_string_field` |
| `create_tenant_group` | Create | `parent` (direct number, not using helper) | ❌ | ✅ | `add_nested_reference` (for parent), `add_optional_string_field` |

### DCIM Operations - Sites

| Method | Type | Nested References | Uses Helper | Slug Gen | Other Helpers Needed |
|--------|------|-------------------|-------------|----------|---------------------|
| `create_site` | Create | `tenant`, `region`, `site_group` | ✅ | ✅ | `add_optional_string_field`, `add_optional_number_field` |
| `update_site` | Update | `tenant`, `region`, `site_group` | ✅ | ❌ | `add_optional_string_field`, `add_optional_number_field` |
| `create_region` | Create | `parent` (direct number, not using helper) | ❌ | ✅ | `add_nested_reference` (for parent), `add_optional_string_field` |
| `create_site_group` | Create | `parent` (direct number, not using helper) | ❌ | ✅ | `add_nested_reference` (for parent), `add_optional_string_field` |
| `create_location` | Create | `site`, `parent`, `tenant` | ✅ | ✅ | `add_optional_string_field` |
| `update_location` | Update | **NOT IMPLEMENTED** | ❌ | ❌ | **MISSING METHOD** |

### DCIM Operations - Devices

| Method | Type | Nested References | Uses Helper | Slug Gen | Other Helpers Needed |
|--------|------|-------------------|-------------|----------|---------------------|
| `create_device` | Create | `tenant`, `platform`, `location` | ✅ | ❌ | `add_optional_string_field` |
| `update_device` | Update | `tenant`, `platform`, `location`, `primary_ip4`, `primary_ip6` | ✅ | ❌ | `add_optional_string_field` |
| `create_interface` | Create | `device` (direct number, not using helper) | ❌ | ❌ | `add_optional_string_field`, `add_optional_number_field`, `add_optional_bool_field` |
| `update_interface` | Update | None | ❌ | ❌ | `add_optional_string_field`, `add_optional_number_field`, `add_optional_bool_field` |
| `create_mac_address` | Create | `assigned_object` (special format) | ❌ | ❌ | `add_optional_string_field` |

### DCIM Operations - Device Components

| Method | Type | Nested References | Uses Helper | Slug Gen | Other Helpers Needed |
|--------|------|-------------------|-------------|----------|---------------------|
| `create_device_role` | Create | None | ❌ | ✅ | `add_optional_string_field`, `add_optional_bool_field` |
| `create_manufacturer` | Create | None | ❌ | ✅ | `add_optional_string_field` |
| `create_platform` | Create | `manufacturer` (direct number, not using helper) | ❌ | ✅ | `add_nested_reference` (for manufacturer), `add_optional_string_field` |
| `create_device_type` | Create | `manufacturer` (direct number, not using helper) | ❌ | ✅ | `add_nested_reference` (for manufacturer), `add_optional_string_field`, `add_optional_number_field`, `add_optional_bool_field` |

### Extras Operations

| Method | Type | Nested References | Uses Helper | Slug Gen | Other Helpers Needed |
|--------|------|-------------------|-------------|----------|---------------------|
| `create_role` | Create | None | ❌ | ✅ | `add_optional_string_field`, `add_optional_number_field` |
| `create_tag` | Create | None | ❌ | ✅ | `add_optional_string_field` |

## Detailed Analysis by Helper Type

### 1. Nested Reference Helper (`add_nested_reference`)

#### ✅ Currently Using Helper (9 methods)
- `create_prefix` - site, vlan, role, tenant
- `update_prefix` - site, vlan, tenant
- `create_site` - tenant, region, site_group
- `update_site` - tenant, region, site_group
- `create_device` - tenant, platform, location
- `update_device` - tenant, platform, location, primary_ip4, primary_ip6
- `create_vlan` - site, group, tenant, role
- `update_vlan` - site, group, tenant, role
- `create_location` - site, parent, tenant

#### ❌ Should Use Helper But Don't (6 methods)
- `create_aggregate` - `rir` (currently: `body["rir"] = serde_json::Value::Number(rir.into())`)
- `create_tenant_group` - `parent` (currently: `body["parent"] = serde_json::Value::Number(parent.into())`)
- `create_region` - `parent` (currently: `body["parent"] = serde_json::Value::Number(parent.into())`)
- `create_site_group` - `parent` (currently: `body["parent"] = serde_json::Value::Number(parent.into())`)
- `create_platform` - `manufacturer` (currently: `body["manufacturer"] = serde_json::Value::Number(mfg_id.into())`)
- `create_device_type` - `manufacturer` (currently: `body["manufacturer"] = serde_json::Value::Number(manufacturer_id.into())`)

#### ⚠️ Special Cases (2 methods)
- `create_tenant` - `group` field requires explicit null handling (not just omit)
- `create_interface` - `device` field uses direct number (may be intentional for required field)

### 2. Slug Generation Helper (`generate_slug`)

#### ✅ Currently Auto-Generating (12 methods)
- `create_tenant` - `name.to_lowercase().replace(' ', "-")`
- `create_tenant_group` - `name.to_lowercase().replace(' ', "-")`
- `create_site` - `name.to_lowercase().replace(' ', "-")`
- `create_region` - `name.to_lowercase().replace(' ', "-")`
- `create_site_group` - `name.to_lowercase().replace(' ', "-")`
- `create_location` - `name.to_lowercase().replace(' ', "-")`
- `create_device_role` - `name.to_lowercase().replace(' ', "-")`
- `create_manufacturer` - `name.to_lowercase().replace(' ', "-")`
- `create_platform` - `name.to_lowercase().replace(' ', "-")`
- `create_device_type` - `name.to_lowercase().replace(' ', "-")`
- `create_role` - `name.to_lowercase().replace(' ', "-")`
- `create_tag` - `name.to_lowercase().replace(' ', "-")`

**Pattern:** All use identical logic: `if let Some(slug_str) = slug { slug_str.to_string() } else { name.to_lowercase().replace(' ', "-") }`

### 3. Optional Field Helpers

#### String Fields
**Pattern:** `if let Some(value) = field { body["field"] = serde_json::Value::String(value); }`

**Used in:** All create/update methods (27 methods)

#### Number Fields
**Pattern:** `if let Some(value) = field { body["field"] = serde_json::Value::Number(value.into()); }`

**Used in:**
- `create_site` - latitude, longitude
- `create_role` - weight
- `create_interface` - mtu
- `create_device_type` - u_height
- `update_interface` - mtu

#### Boolean Fields
**Pattern:** `if let Some(value) = field { body["field"] = serde_json::Value::Bool(value); }`

**Used in:**
- `create_interface` - enabled
- `create_device_role` - vm_role
- `create_device_type` - is_full_depth
- `update_interface` - enabled

#### Enum/Choice Fields
**Pattern:** `if let Some(value) = field { body["field"] = serde_json::to_value(value)?; }`

**Used in:**
- `create_ip_address` - status (IPAddressStatus enum)
- `update_ip_address` - status (IPAddressStatus enum)
- `create_prefix` - status (string)
- `update_prefix` - status (string)
- `create_vlan` - status (string)
- `update_vlan` - status (string)
- `create_device` - status (string)
- `update_device` - status (string)

## Recommendations

### High Priority

1. **Extend `add_nested_reference` to handle required nested fields**
   - Currently only handles `Option<u64>`
   - Need variant for required `u64` fields
   - Methods: `create_aggregate` (rir), `create_interface` (device), `create_device_type` (manufacturer)

2. **Create `generate_slug` helper**
   - Eliminate 12 duplicate implementations
   - Standardize slug generation logic

3. **Create `add_optional_string_field` helper**
   - Most common pattern (used in all 27 methods)
   - Reduces boilerplate significantly

### Medium Priority

4. **Create `add_optional_number_field` helper**
   - Used in 5 methods
   - Standardize number serialization

5. **Create `add_optional_bool_field` helper**
   - Used in 4 methods
   - Standardize boolean serialization

6. **Create `handle_nullable_nested_reference` helper**
   - Special case for fields that must be explicitly null
   - Used in `create_tenant` (group field)

### Low Priority

7. **Create `add_optional_enum_field` helper**
   - Handle enum/choice field serialization
   - Used in 8 methods

8. **Implement `update_location` method**
   - Currently missing from API
   - Should follow same patterns as other update methods

## Code Duplication Statistics

- **Slug generation:** 12 duplicate implementations (identical logic)
- **Optional string fields:** ~100+ duplicate `if let Some` blocks
- **Optional number fields:** ~10 duplicate blocks
- **Optional boolean fields:** ~8 duplicate blocks
- **Nested references (direct numbers):** 6 methods not using helper
- **Total estimated duplicate code:** ~150+ lines

## Helper Function Signatures (Proposed)

```rust
// Nested references
fn add_nested_reference(&self, body: &mut serde_json::Value, field_name: &str, id: Option<u64>)
fn add_required_nested_reference(&self, body: &mut serde_json::Value, field_name: &str, id: u64)
fn add_nullable_nested_reference(&self, body: &mut serde_json::Value, field_name: &str, id: Option<u64>)

// Slug generation
fn generate_slug(name: &str, provided_slug: Option<&str>) -> String

// Optional fields
fn add_optional_string_field(body: &mut serde_json::Value, field_name: &str, value: Option<&str>)
fn add_optional_string_field_owned(body: &mut serde_json::Value, field_name: &str, value: Option<String>)
fn add_optional_number_field<T: Into<serde_json::Number>>(body: &mut serde_json::Value, field_name: &str, value: Option<T>)
fn add_optional_bool_field(body: &mut serde_json::Value, field_name: &str, value: Option<bool>)
fn add_optional_enum_field<T: Serialize>(body: &mut serde_json::Value, field_name: &str, value: Option<T>) -> Result<(), NetBoxError>
```

## Next Steps

1. Review this audit document
2. Prioritize which helpers to implement first
3. Create implementation plan for selected helpers
4. Refactor methods to use new helpers incrementally
5. Add unit tests for helper functions

