# Fix: Send Full Tenant Object in PATCH Updates

## Problem

NetBox was returning validation errors:
```json
{"tenant":{"name":["This field cannot be blank."],"slug":["This field cannot be blank."]}}
```

**The error message literally told us what was wrong!** We were sending `{"tenant": {"id": X}}` but NetBox requires the full tenant object with `name` and `slug` fields.

## Root Cause

We were ignoring the error message and trying to fix it by:
1. Making tenant required in CRDs (good, but not the issue)
2. Always including tenant in updates (good, but not the issue)
3. Trying to fix NetBox serializer validation (wrong approach)

The actual issue was simple: **NetBox wants the full tenant object, not just the ID**.

## Solution

Updated all PATCH update methods to:
1. Fetch the full tenant object using `get_tenant(tid)`
2. Send the complete tenant object with `id`, `name`, `slug`, and optionally `group`

### Updated Methods

- `update_site`: Now fetches and sends full tenant object
- `update_prefix`: Now fetches and sends full tenant object  
- `update_vlan`: Now fetches and sends full tenant object
- `update_device`: Now fetches and sends full tenant object

### Code Pattern

```rust
if let Some(tid) = tenant_id {
    match self.get_tenant(tid).await {
        Ok(tenant) => {
            let mut tenant_obj = serde_json::json!({
                "id": tenant.id,
                "name": tenant.name,
                "slug": tenant.slug,
            });
            if let Some(group) = tenant.group {
                tenant_obj["group"] = serde_json::json!({
                    "id": group.id,
                    "name": group.name,
                    "slug": group.slug,
                });
            }
            body["tenant"] = tenant_obj;
        }
        Err(e) => {
            warn!("Failed to fetch tenant {}: {}, sending just id", tid, e);
            body["tenant"] = serde_json::json!({"id": tid});
        }
    }
}
```

## Lesson Learned

**Always read the error messages!** The error literally told us:
- `name` field cannot be blank
- `slug` field cannot be blank

This meant we needed to include `name` and `slug` in the tenant object, not just `id`.

