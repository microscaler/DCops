# Site Creation Missing Fields Issue

## Problem

The site in NetBox (ID: 1) was created **without** tenant, region, or site_group fields populated, even though:
1. The CRD specifies these fields
2. The reconciler resolves the tenant/region/site_group IDs
3. The `create_site` call should include them

## Root Cause

The site was likely created **before** the tenant/region/site_group CRDs existed, or the reconciler failed to resolve them during creation. The site was created with only basic fields (name, slug, status).

## Current State

- **NetBox Site ID 1**: Tenant=None, Region=None, Site Group=None
- **CRD**: Specifies tenant, region, and site_group references
- **Reconciler**: Tries to update the site with tenant/region/site_group, but gets validation errors

## Solution

1. **Make tenant required in CRDs** (already done)
2. **Fix the update logic** to always include tenant/region/site_group when they exist (already done)
3. **Force update the existing site** to populate missing fields, or delete and recreate it

## Next Steps

Since the site already exists without these fields, we need to:
1. Ensure the reconciler can update the site even when existing fields are None
2. The update should succeed now that we always include tenant/region/site_group when they exist in the CRD
3. If update still fails, we may need to delete and recreate the site

