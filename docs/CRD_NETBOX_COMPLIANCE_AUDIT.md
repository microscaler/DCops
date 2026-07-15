# CRD NetBox API Compliance Audit

## Problem Statement

We are creating resources in NetBox with **incomplete data**, violating NetBox API requirements. This causes:
1. Resources created without required fields (tenant, facility, parent, etc.)
2. Validation errors when trying to update resources
3. Inconsistent state between CRDs and NetBox

## Root Cause

Our CRDs are **not aligned with NetBox API requirements**. We're making fields optional when NetBox requires them, or missing fields entirely.

## Examples of Issues

### Location
- **NetBox Location ID 1**: Tenant=None, Parent=None, Facility=""
- **Our CRD**: Missing `tenant` and `facility` fields entirely
- **Result**: Location created without tenant/facility, violating NetBox requirements

### Site
- **NetBox Site ID 1**: Tenant=None, Region=None, Site Group=None
- **Our CRD**: Has tenant/region/site_group but they were optional
- **Result**: Site created without these fields, causing validation errors on updates

## Required Actions

1. **Audit NetBox API Requirements**: For each resource type (Site, Location, Device, Prefix, VLAN, etc.):
   - Check NetBox serializer/model to identify required fields
   - Check NetBox API documentation
   - Check existing NetBox resources to see what's actually required

2. **Update CRDs**: Make fields required in CRDs where NetBox requires them:
   - `tenant` should be required for Site, Location, Device, Prefix, VLAN
   - `facility` may be required for Location
   - Other fields as identified

3. **Update CRs**: Ensure all existing CRs include required fields

4. **Update Reconcilers**: Ensure create/update calls include all required fields

## Next Steps

1. Check NetBox Location serializer to see what fields are required
2. Update Location CRD to include tenant and facility if required
3. Update create_location call to include tenant and facility
4. Repeat for all other resource types
5. Update existing CRs to include missing required fields

