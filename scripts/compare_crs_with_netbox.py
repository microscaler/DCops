#!/usr/bin/env python3
"""
Compare Kubernetes CRs with NetBox API state and identify inconsistencies.

This script:
1. Discovers all CR YAML files in config/examples/
2. For each CR, queries NetBox API to get the actual resource
3. Compares each field from CR spec with NetBox response
4. Tabulates all inconsistencies
5. Provides gap analysis of what's not working correctly

Usage:
    python3 scripts/compare_crs_with_netbox.py [--netbox-url URL] [--netbox-token TOKEN]
    
Environment variables:
    NETBOX_URL: NetBox base URL (default: http://netbox.netbox:80)
    NETBOX_TOKEN: NetBox API token (required)
"""

import argparse
import base64
import json
import os
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Set
import requests
from urllib.parse import urljoin
import yaml

# NetBox API endpoints mapping
NETBOX_ENDPOINTS = {
    "NetBoxIPAddress": "/api/ipam/ip-addresses/",
    "NetBoxIPRange": "/api/ipam/ip-ranges/",
    "NetBoxPrefix": "/api/ipam/prefixes/",
    "NetBoxVRF": "/api/ipam/vrfs/",
    "NetBoxRouteTarget": "/api/ipam/route-targets/",
    "NetBoxAggregate": "/api/ipam/aggregates/",
    "NetBoxVLAN": "/api/ipam/vlans/",
    "NetBoxRIR": "/api/ipam/rirs/",
    "NetBoxRole": "/api/ipam/roles/",
    "NetBoxTenant": "/api/tenancy/tenants/",
    "NetBoxTenantGroup": "/api/tenancy/tenant-groups/",
    "NetBoxTag": "/api/extras/tags/",
    "NetBoxSite": "/api/dcim/sites/",
    "NetBoxRegion": "/api/dcim/regions/",
    "NetBoxSiteGroup": "/api/dcim/site-groups/",
    "NetBoxLocation": "/api/dcim/locations/",
    "NetBoxDevice": "/api/dcim/devices/",
    "NetBoxInterface": "/api/dcim/interfaces/",
    "NetBoxMACAddress": "/api/dcim/mac-addresses/",
    "NetBoxDeviceRole": "/api/dcim/device-roles/",
    "NetBoxManufacturer": "/api/dcim/manufacturers/",
    "NetBoxPlatform": "/api/dcim/platforms/",
    "NetBoxDeviceType": "/api/dcim/device-types/",
}

@dataclass
class FieldInconsistency:
    """Represents a field inconsistency between CR and NetBox."""
    cr_name: str
    cr_kind: str
    field_name: str
    cr_value: Any
    netbox_value: Any
    netbox_id: Optional[int] = None
    issue_type: str = "mismatch"  # mismatch, missing_in_netbox, missing_in_cr, type_mismatch
    gap_analysis: str = ""

@dataclass
class ResourceComparison:
    """Comparison result for a single resource."""
    cr_name: str
    cr_kind: str
    cr_namespace: str
    netbox_id: Optional[int] = None
    netbox_url: Optional[str] = None
    found_in_netbox: bool = False
    inconsistencies: List[FieldInconsistency] = field(default_factory=list)
    errors: List[str] = field(default_factory=list)

def get_netbox_url() -> str:
    """Get NetBox URL from environment or default."""
    url = os.getenv("NETBOX_URL")
    if url:
        return url
    
    # Try to get from Kubernetes service
    try:
        result = subprocess.run(
            ["kubectl", "get", "svc", "netbox", "-n", "netbox", "-o", "jsonpath={.spec.clusterIP}"],
            capture_output=True,
            text=True,
            check=False
        )
        if result.returncode == 0 and result.stdout.strip():
            return f"http://{result.stdout.strip()}:80"
    except Exception:
        pass
    
    return "http://netbox.netbox:80"

def get_netbox_token() -> Optional[str]:
    """Get NetBox token from environment or Kubernetes secret."""
    # Try environment variable first
    token = os.getenv("NETBOX_TOKEN")
    if token:
        return token
    
    # Try tenant-specific token first, then main token
    for secret_name in ["netbox-token-datacenter-tenant", "netbox-token"]:
        try:
            result = subprocess.run(
                ["kubectl", "get", "secret", secret_name, "-n", "dcops-system",
                 "-o", "jsonpath={.data.token}"],
                capture_output=True,
                text=True,
                check=False
            )
            if result.returncode == 0 and result.stdout.strip():
                try:
                    token = base64.b64decode(result.stdout.strip()).decode('utf-8')
                    if token and token != "PLACEHOLDER_DO_NOT_USE" and len(token) > 10:
                        return token
                except Exception:
                    continue
        except Exception:
            continue
    
    return None

def find_cr_files(directory: Path) -> List[Path]:
    """Find all CR YAML files in the given directory."""
    cr_files = []
    if directory.exists():
        for file in sorted(directory.rglob("*.yaml")):
            # Skip non-CR files
            if file.name.startswith("netbox-") or "netbox" in file.name.lower():
                cr_files.append(file)
    return cr_files

def load_cr(file_path: Path) -> Optional[Dict]:
    """Load a CR YAML file."""
    try:
        with open(file_path, 'r') as f:
            return yaml.safe_load(f)
    except Exception as e:
        print(f"⚠️  Failed to load {file_path}: {e}", file=sys.stderr)
        return None

def query_netbox_resource(session: requests.Session, base_url: str, endpoint: str, 
                         filters: Dict[str, str]) -> List[Dict]:
    """Query NetBox API for resources matching filters."""
    url = urljoin(base_url, endpoint)
    params = filters.copy()
    
    try:
        response = session.get(url, params=params, timeout=10)
        response.raise_for_status()
        data = response.json()
        return data.get("results", [])
    except requests.exceptions.RequestException as e:
        print(f"⚠️  Failed to query {endpoint}: {e}", file=sys.stderr)
        return []

def get_netbox_resource_by_id(session: requests.Session, base_url: str, 
                              endpoint: str, resource_id: int) -> Optional[Dict]:
    """Get a specific NetBox resource by ID."""
    url = urljoin(base_url, f"{endpoint}{resource_id}/")
    try:
        response = session.get(url, timeout=10)
        response.raise_for_status()
        return response.json()
    except requests.exceptions.RequestException as e:
        return None

def resolve_dependency_id(session: requests.Session, base_url: str, 
                         reference: Dict, kind: str) -> Optional[int]:
    """Resolve a CRD reference to a NetBox ID."""
    if not reference:
        return None
    
    ref_name = reference.get("name")
    if not ref_name:
        return None
    
    # Map CRD kind to NetBox endpoint
    endpoint = NETBOX_ENDPOINTS.get(kind)
    if not endpoint:
        return None
    
    # Query by name
    resources = query_netbox_resource(session, base_url, endpoint, {"name": ref_name})
    if resources:
        return resources[0].get("id")
    return None

def compare_field(cr_value: Any, netbox_value: Any, field_name: str) -> bool:
    """Compare a field value between CR and NetBox."""
    # Handle None/empty cases
    if cr_value is None or cr_value == "":
        return netbox_value is None or netbox_value == "" or netbox_value == []
    if netbox_value is None or netbox_value == "":
        return cr_value is None or cr_value == "" or cr_value == []
    
    # Handle nested objects (references)
    if isinstance(cr_value, dict) and isinstance(netbox_value, dict):
        # Compare IDs if both have id field
        if "id" in netbox_value:
            return cr_value.get("id") == netbox_value.get("id")
        # Compare names
        if "name" in cr_value and "name" in netbox_value:
            return cr_value.get("name") == netbox_value.get("name")
        # Compare display names
        if "display" in netbox_value:
            return cr_value.get("name") == netbox_value.get("display")
        return False
    
    # Handle lists (e.g., tags)
    if isinstance(cr_value, list) and isinstance(netbox_value, list):
        if len(cr_value) == 0 and len(netbox_value) == 0:
            return True
        if len(cr_value) != len(netbox_value):
            return False
        # Extract names/IDs from both lists
        cr_items = set()
        for item in cr_value:
            if isinstance(item, dict):
                cr_items.add(item.get("name", item.get("id")))
            else:
                cr_items.add(str(item))
        
        netbox_items = set()
        for item in netbox_value:
            if isinstance(item, dict):
                netbox_items.add(item.get("name", item.get("id", item.get("display"))))
            else:
                netbox_items.add(str(item))
        
        return cr_items == netbox_items
    
    # Handle boolean comparisons
    if isinstance(cr_value, bool) and isinstance(netbox_value, bool):
        return cr_value == netbox_value
    
    # Handle numeric comparisons
    if isinstance(cr_value, (int, float)) and isinstance(netbox_value, (int, float)):
        return cr_value == netbox_value
    
    # String comparison (case-insensitive for some fields)
    if isinstance(cr_value, str) and isinstance(netbox_value, str):
        # Some fields are case-insensitive
        if field_name.lower() in ["name", "slug", "description", "comments"]:
            return cr_value.lower() == netbox_value.lower()
        return cr_value == netbox_value
    
    # Direct comparison
    return str(cr_value) == str(netbox_value)

def extract_netbox_field_value(netbox_resource: Dict, field_path: str) -> Any:
    """Extract a field value from NetBox resource, handling nested paths."""
    parts = field_path.split(".")
    value = netbox_resource
    for part in parts:
        if isinstance(value, dict):
            value = value.get(part)
        elif isinstance(value, list) and part.isdigit():
            idx = int(part)
            if 0 <= idx < len(value):
                value = value[idx]
            else:
                return None
        else:
            return None
        if value is None:
            return None
    return value

def compare_resource(cr: Dict, netbox_resource: Dict, kind: str, 
                    session: requests.Session, base_url: str) -> ResourceComparison:
    """Compare a CR with its NetBox resource."""
    metadata = cr.get("metadata", {})
    spec = cr.get("spec", {})
    status = cr.get("status", {})
    
    cr_name = metadata.get("name", "unknown")
    cr_namespace = metadata.get("namespace", "default")
    netbox_id = status.get("netboxId") or netbox_resource.get("id")
    
    comparison = ResourceComparison(
        cr_name=cr_name,
        cr_kind=kind,
        cr_namespace=cr_namespace,
        netbox_id=netbox_id,
        netbox_url=netbox_resource.get("url"),
        found_in_netbox=True
    )
    
    # Field mappings: CR field -> NetBox field path
    field_mappings = get_field_mappings(kind)
    
    # Also check for tags (special handling)
    if "tags" in spec:
        cr_tags = spec.get("tags", [])
        netbox_tags = netbox_resource.get("tags", [])
        if not compare_field(cr_tags, netbox_tags, "tags"):
            comparison.inconsistencies.append(FieldInconsistency(
                cr_name=cr_name,
                cr_kind=kind,
                field_name="tags",
                cr_value=[t.get("name") if isinstance(t, dict) else t for t in cr_tags],
                netbox_value=[t.get("name") if isinstance(t, dict) else t for t in netbox_tags],
                netbox_id=netbox_id,
                issue_type="mismatch",
                gap_analysis=f"Tag mismatch: CR has {len(cr_tags)} tags, NetBox has {len(netbox_tags)} tags. Tag reconciliation may not be working correctly."
            ))
    
    # Compare each field
    for cr_field, netbox_field_path in field_mappings.items():
        # Skip tags as we handle them separately above
        if cr_field == "tags":
            continue
        
        cr_value = spec.get(cr_field)
        netbox_value = extract_netbox_field_value(netbox_resource, netbox_field_path)
        
        # Handle reference fields (tenant, region, etc.)
        if cr_field in ["tenant", "region", "siteGroup", "site", "location", "parent", 
                       "deviceType", "deviceRole", "platform", "manufacturer", "vrf", 
                       "vlan", "rir", "role", "group"]:
            if isinstance(cr_value, dict) and cr_value.get("kind"):
                ref_kind = cr_value.get("kind", "")
                ref_name = cr_value.get("name", "")
                ref_id = resolve_dependency_id(session, base_url, cr_value, ref_kind)
                
                netbox_ref_id = None
                netbox_ref_name = None
                if isinstance(netbox_value, dict):
                    netbox_ref_id = netbox_value.get("id")
                    netbox_ref_name = netbox_value.get("name") or netbox_value.get("display")
                
                if ref_id and ref_id != netbox_ref_id:
                    comparison.inconsistencies.append(FieldInconsistency(
                        cr_name=cr_name,
                        cr_kind=kind,
                        field_name=cr_field,
                        cr_value=f"{ref_kind}/{ref_name}:{ref_id}",
                        netbox_value=f"{netbox_ref_name or 'unknown'}:{netbox_ref_id}" if netbox_ref_id else "None",
                        netbox_id=netbox_id,
                        issue_type="mismatch",
                        gap_analysis=f"Reference mismatch: CR expects {ref_kind} '{ref_name}' (ID: {ref_id}), NetBox has '{netbox_ref_name}' (ID: {netbox_ref_id})"
                    ))
                elif not ref_id and netbox_ref_id:
                    comparison.inconsistencies.append(FieldInconsistency(
                        cr_name=cr_name,
                        cr_kind=kind,
                        field_name=cr_field,
                        cr_value=f"{ref_kind}/{ref_name}: Not resolved",
                        netbox_value=f"{netbox_ref_name}:{netbox_ref_id}",
                        netbox_id=netbox_id,
                        issue_type="missing_in_cr",
                        gap_analysis=f"Reference not resolved: CR references {ref_kind} '{ref_name}' but could not find it in NetBox"
                    ))
                continue
        
        # Handle special cases (references, enums, etc.)
        if cr_field.endswith("_ref") or (isinstance(cr_value, dict) and cr_value.get("kind")):
            # This is a reference - resolve it
            if isinstance(cr_value, dict) and cr_value.get("kind"):
                ref_kind = cr_value.get("kind", "")
                ref_name = cr_value.get("name", "")
                ref_id = resolve_dependency_id(session, base_url, cr_value, ref_kind)
                
                netbox_ref_id = None
                netbox_ref_name = None
                if isinstance(netbox_value, dict):
                    netbox_ref_id = netbox_value.get("id")
                    netbox_ref_name = netbox_value.get("name") or netbox_value.get("display")
                
                if ref_id and ref_id != netbox_ref_id:
                    comparison.inconsistencies.append(FieldInconsistency(
                        cr_name=cr_name,
                        cr_kind=kind,
                        field_name=cr_field,
                        cr_value=f"{ref_kind}/{ref_name}:{ref_id}",
                        netbox_value=f"{netbox_ref_name or 'unknown'}:{netbox_ref_id}" if netbox_ref_id else "None",
                        netbox_id=netbox_id,
                        issue_type="mismatch",
                        gap_analysis=f"Reference mismatch: CR expects {ref_kind} '{ref_name}' (ID: {ref_id}), NetBox has '{netbox_ref_name}' (ID: {netbox_ref_id})"
                    ))
                elif not ref_id and netbox_ref_id:
                    comparison.inconsistencies.append(FieldInconsistency(
                        cr_name=cr_name,
                        cr_kind=kind,
                        field_name=cr_field,
                        cr_value=f"{ref_kind}/{ref_name}: Not resolved",
                        netbox_value=f"{netbox_ref_name}:{netbox_ref_id}",
                        netbox_id=netbox_id,
                        issue_type="missing_in_cr",
                        gap_analysis=f"Reference not resolved: CR references {ref_kind} '{ref_name}' but could not find it in NetBox"
                    ))
                continue
        
        # Skip comparison if field is not in spec (optional fields)
        if cr_value is None and netbox_value is None:
            continue
        
        # Regular field comparison
        if not compare_field(cr_value, netbox_value, cr_field):
            # Determine issue type and gap analysis
            if cr_value is None and netbox_value is not None:
                issue_type = "missing_in_cr"
                gap_analysis = f"Field exists in NetBox ('{netbox_value}') but not specified in CR (may be optional or should be added)"
            elif cr_value is not None and netbox_value is None:
                issue_type = "missing_in_netbox"
                gap_analysis = f"CR specifies '{cr_value}' but field is missing/null in NetBox (reconciler may not be updating this field)"
            elif isinstance(cr_value, list) and isinstance(netbox_value, list):
                issue_type = "mismatch"
                gap_analysis = f"List mismatch: CR has {len(cr_value)} items, NetBox has {len(netbox_value)} items. CR: {cr_value}, NetBox: {netbox_value}"
            else:
                issue_type = "mismatch"
                gap_analysis = f"Value mismatch: CR specifies '{cr_value}' but NetBox has '{netbox_value}' (reconciler may not be updating this field or drift detection not working)"
            
            comparison.inconsistencies.append(FieldInconsistency(
                cr_name=cr_name,
                cr_kind=kind,
                field_name=cr_field,
                cr_value=cr_value,
                netbox_value=netbox_value,
                netbox_id=netbox_id,
                issue_type=issue_type,
                gap_analysis=gap_analysis
            ))
    
    return comparison

def get_field_mappings(kind: str) -> Dict[str, str]:
    """Get field mappings for a CRD kind."""
    # Base mappings common to most resources
    base_mappings = {
        "name": "name",
        "description": "description",
        "comments": "comments",
    }
    
    # Reference fields that need special handling (will be handled separately)
    reference_fields = {
        "tenant": "tenant",
        "region": "region",
        "siteGroup": "site_group",
        "site": "site",
        "location": "location",
        "parent": "parent",
        "deviceType": "device_type",
        "deviceRole": "device_role",
        "platform": "platform",
        "manufacturer": "manufacturer",
        "vrf": "vrf",
        "vlan": "vlan",
        "rir": "rir",
        "role": "role",
        "group": "group",
        "interface": "assigned_object",
        "ipRange": "ip_range",
        "primaryIp4": "primary_ip4",
        "primaryIp6": "primary_ip6",
    }
    
    # Kind-specific mappings
    kind_mappings = {
        "NetBoxManufacturer": {
            **base_mappings,
            "slug": "slug",
        },
        "NetBoxPlatform": {
            **base_mappings,
            "slug": "slug",
        },
        "NetBoxDeviceType": {
            **base_mappings,
            "model": "model",
            "slug": "slug",
            "partNumber": "part_number",
            "uHeight": "u_height",
            "isFullDepth": "is_full_depth",
        },
        "NetBoxDeviceRole": {
            **base_mappings,
            "slug": "slug",
            "color": "color",
            "vmRole": "vm_role",
        },
        "NetBoxSite": {
            **base_mappings,
            "slug": "slug",
            "status": "status.value",
            "facility": "facility",
            "timeZone": "time_zone",
        },
        "NetBoxRegion": {
            **base_mappings,
            "slug": "slug",
        },
        "NetBoxSiteGroup": {
            **base_mappings,
            "slug": "slug",
        },
        "NetBoxLocation": {
            **base_mappings,
            "slug": "slug",
            "facility": "facility",
        },
        "NetBoxVLAN": {
            **base_mappings,
            "vid": "vid",
            "status": "status.value",
        },
        "NetBoxRIR": {
            **base_mappings,
            "slug": "slug",
            "isPrivate": "is_private",
        },
        "NetBoxRole": {
            **base_mappings,
            "slug": "slug",
            "weight": "weight",
        },
        "NetBoxTag": {
            **base_mappings,
            "slug": "slug",
            "color": "color",
        },
        "NetBoxTenant": {
            **base_mappings,
            "slug": "slug",
        },
        "NetBoxTenantGroup": {
            **base_mappings,
            "slug": "slug",
        },
        "NetBoxAggregate": {
            **base_mappings,
            "prefix": "prefix",
        },
        "NetBoxPrefix": {
            **base_mappings,
            "prefix": "prefix",
            "status": "status.value",
        },
        "NetBoxIPRange": {
            **base_mappings,
            "startAddress": "start_address",
            "endAddress": "end_address",
            "status": "status.value",
            "markPopulated": "mark_utilized",  # Note: NetBox uses mark_utilized, we use markPopulated
            "markUtilized": "mark_utilized",
        },
        "NetBoxIPAddress": {
            **base_mappings,
            "address": "address",
            "status": "status.value",
            "dnsName": "dns_name",
        },
        "NetBoxVRF": {
            **base_mappings,
            "routeDistinguisher": "rd",
        },
        "NetBoxRouteTarget": {
            **base_mappings,
        },
        "NetBoxDevice": {
            **base_mappings,
            "status": "status.value",
            "serial": "serial",
            "assetTag": "asset_tag",
        },
        "NetBoxInterface": {
            **base_mappings,
            "type": "type",
            "enabled": "enabled",
            "macAddress": "mac_address",
            "mtu": "mtu",
            "name": "name",  # Interface name
        },
        "NetBoxMACAddress": {
            **base_mappings,
            "macAddress": "mac_address",
        },
    }
    
    return kind_mappings.get(kind, base_mappings)

def analyze_all_crs(base_url: str, token: str) -> List[ResourceComparison]:
    """Analyze all CRs and compare with NetBox."""
    project_root = Path(__file__).parent.parent
    examples_dir = project_root / "config" / "examples"
    
    cr_files = find_cr_files(examples_dir)
    print(f"📋 Found {len(cr_files)} CR files to analyze\n")
    
    session = requests.Session()
    session.headers.update({
        "Authorization": f"Token {token}",
        "Accept": "application/json",
    })
    
    comparisons = []
    
    for cr_file in cr_files:
        cr = load_cr(cr_file)
        if not cr:
            continue
        
        kind = cr.get("kind", "")
        if not kind.startswith("NetBox"):
            continue
        
        metadata = cr.get("metadata", {})
        cr_name = metadata.get("name", "unknown")
        status = cr.get("status", {})
        netbox_id = status.get("netboxId")
        
        print(f"🔍 Analyzing {kind}/{cr_name}...", end=" ")
        
        endpoint = NETBOX_ENDPOINTS.get(kind)
        if not endpoint:
            print(f"❌ Unknown endpoint for {kind}")
            continue
        
        # Try to find resource in NetBox
        netbox_resource = None
        if netbox_id:
            netbox_resource = get_netbox_resource_by_id(session, base_url, endpoint, netbox_id)
        
        if not netbox_resource:
            # Try querying by name
            resources = query_netbox_resource(session, base_url, endpoint, {"name": cr_name})
            if resources:
                netbox_resource = resources[0]
        
        if not netbox_resource:
            comparison = ResourceComparison(
                cr_name=cr_name,
                cr_kind=kind,
                cr_namespace=metadata.get("namespace", "default"),
                found_in_netbox=False,
                errors=[f"Resource not found in NetBox (expected ID: {netbox_id})"]
            )
            comparisons.append(comparison)
            print("❌ Not found in NetBox")
            continue
        
        # Compare resource
        comparison = compare_resource(cr, netbox_resource, kind, session, base_url)
        comparisons.append(comparison)
        
        if comparison.inconsistencies:
            print(f"⚠️  {len(comparison.inconsistencies)} inconsistencies")
        else:
            print("✅ Match")
    
    return comparisons

def generate_report(comparisons: List[ResourceComparison]) -> str:
    """Generate a comprehensive report."""
    lines = []
    lines.append("=" * 100)
    lines.append("NETBOX CR COMPARISON REPORT")
    lines.append("=" * 100)
    lines.append("")
    
    # Summary
    total_crs = len(comparisons)
    found_in_netbox = sum(1 for c in comparisons if c.found_in_netbox)
    total_inconsistencies = sum(len(c.inconsistencies) for c in comparisons)
    resources_with_issues = sum(1 for c in comparisons if c.inconsistencies or c.errors)
    
    lines.append("SUMMARY")
    lines.append("-" * 100)
    lines.append(f"Total CRs analyzed: {total_crs}")
    lines.append(f"Found in NetBox: {found_in_netbox}")
    lines.append(f"Not found in NetBox: {total_crs - found_in_netbox}")
    lines.append(f"Resources with inconsistencies: {resources_with_issues}")
    lines.append(f"Total field inconsistencies: {total_inconsistencies}")
    lines.append("")
    
    # Detailed inconsistencies table
    if total_inconsistencies > 0:
        lines.append("=" * 100)
        lines.append("DETAILED INCONSISTENCIES TABLE")
        lines.append("=" * 100)
        lines.append("")
        lines.append(f"{'CR Name':<30} {'Kind':<25} {'Field':<25} {'CR Value':<30} {'NetBox Value':<30} {'Issue':<15} {'Gap Analysis'}")
        lines.append("-" * 100)
        
        for comparison in comparisons:
            for inconsistency in comparison.inconsistencies:
                cr_val_str = str(inconsistency.cr_value)[:28] + ".." if len(str(inconsistency.cr_value)) > 30 else str(inconsistency.cr_value)
                nb_val_str = str(inconsistency.netbox_value)[:28] + ".." if len(str(inconsistency.netbox_value)) > 30 else str(inconsistency.netbox_value)
                gap_str = inconsistency.gap_analysis[:50] + ".." if len(inconsistency.gap_analysis) > 52 else inconsistency.gap_analysis
                
                lines.append(f"{inconsistency.cr_name:<30} {inconsistency.cr_kind:<25} {inconsistency.field_name:<25} "
                           f"{cr_val_str:<30} {nb_val_str:<30} {inconsistency.issue_type:<15} {gap_str}")
        
        lines.append("")
    
    # Resources not found in NetBox
    missing = [c for c in comparisons if not c.found_in_netbox]
    if missing:
        lines.append("=" * 100)
        lines.append("RESOURCES NOT FOUND IN NETBOX")
        lines.append("=" * 100)
        lines.append("")
        for comparison in missing:
            lines.append(f"❌ {comparison.cr_kind}/{comparison.cr_name} (namespace: {comparison.cr_namespace})")
            for error in comparison.errors:
                lines.append(f"   Error: {error}")
        lines.append("")
    
    # Gap Analysis Summary
    if total_inconsistencies > 0:
        lines.append("=" * 100)
        lines.append("GAP ANALYSIS SUMMARY")
        lines.append("=" * 100)
        lines.append("")
        
        # Group by issue type
        by_issue_type = defaultdict(list)
        for comparison in comparisons:
            for inconsistency in comparison.inconsistencies:
                by_issue_type[inconsistency.issue_type].append(inconsistency)
        
        for issue_type, inconsistencies in by_issue_type.items():
            lines.append(f"\n{issue_type.upper()} Issues ({len(inconsistencies)}):")
            lines.append("-" * 100)
            
            # Group by field
            by_field = defaultdict(list)
            for inc in inconsistencies:
                by_field[inc.field_name].append(inc)
            
            for field, field_issues in by_field.items():
                lines.append(f"  Field: {field} ({len(field_issues)} occurrences)")
                for inc in field_issues[:3]:  # Show first 3 examples
                    lines.append(f"    - {inc.cr_kind}/{inc.cr_name}: {inc.gap_analysis}")
                if len(field_issues) > 3:
                    lines.append(f"    ... and {len(field_issues) - 3} more")
        
        lines.append("")
    
    return "\n".join(lines)

def main():
    parser = argparse.ArgumentParser(
        description="Compare Kubernetes CRs with NetBox API state",
        formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--netbox-url",
        default=None,
        help="NetBox base URL (default: from NETBOX_URL env or auto-detect)"
    )
    parser.add_argument(
        "--netbox-token",
        default=None,
        help="NetBox API token (default: from NETBOX_TOKEN env or Kubernetes secret)"
    )
    parser.add_argument(
        "--output",
        default=None,
        help="Output file path (default: print to stdout)"
    )
    
    args = parser.parse_args()
    
    # Get NetBox URL and token
    base_url = args.netbox_url or get_netbox_url()
    token = args.netbox_token or get_netbox_token()
    
    if not token:
        print("❌ Error: NetBox token is required", file=sys.stderr)
        print("   Set NETBOX_TOKEN environment variable or use --netbox-token", file=sys.stderr)
        sys.exit(1)
    
    print(f"🔗 Connecting to NetBox at {base_url}")
    print(f"🔑 Using token: {'*' * 20}{token[-4:] if len(token) > 4 else ''}\n")
    
    # Analyze all CRs
    comparisons = analyze_all_crs(base_url, token)
    
    # Generate report
    report = generate_report(comparisons)
    
    # Output report
    if args.output:
        with open(args.output, 'w') as f:
            f.write(report)
        print(f"\n✅ Report written to {args.output}")
    else:
        print("\n" + report)

if __name__ == "__main__":
    main()

