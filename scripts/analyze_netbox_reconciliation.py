#!/usr/bin/env python3
"""
Analyze NetBox controller logs and verify NetBox API state.

This script:
1. Parses tilt-netbox-controller.logs by reconciler
2. Extracts errors, warnings, and issues
3. Queries NetBox API for each resource type
4. Compares expected vs actual state
5. Generates a comprehensive report
"""

import json
import re
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Set
import requests
from urllib.parse import urljoin

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
class LogEntry:
    """Represents a log entry."""
    timestamp: str
    level: str  # INFO, WARN, ERROR
    reconciler: str
    resource: Optional[str] = None
    message: str = ""
    error_details: Optional[str] = None

@dataclass
class ReconcilerStats:
    """Statistics for a reconciler."""
    name: str
    total_logs: int = 0
    errors: List[LogEntry] = field(default_factory=list)
    warnings: List[LogEntry] = field(default_factory=list)
    info: List[LogEntry] = field(default_factory=list)
    netbox_resources: List[Dict] = field(default_factory=list)
    expected_resources: Set[str] = field(default_factory=set)
    missing_resources: Set[str] = field(default_factory=set)
    issues: List[str] = field(default_factory=list)

def parse_log_line(line: str) -> Optional[LogEntry]:
    """Parse a log line into a LogEntry."""
    # Remove ANSI escape codes first
    ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')
    clean_line = ansi_escape.sub('', line)
    
    # Pattern: timestamp LEVEL reconciling object{object.ref=ResourceType.name.namespace ...}: module: message
    # Or: timestamp LEVEL module: message (for non-reconciling logs)
    
    # Try pattern with reconciling object
    pattern1 = r'(\d{4}-\d{2}-\d{2}T[\d:\.]+Z)\s+(\w+)\s+reconciling object.*?object\.ref\]=([^\s]+).*?:\s+([^:]+):\s+(.+)'
    match = re.search(pattern1, clean_line)
    
    if match:
        timestamp, level, resource_ref, module, message = match.groups()
    else:
        # Try pattern without reconciling object
        pattern2 = r'(\d{4}-\d{2}-\d{2}T[\d:\.]+Z)\s+(\w+)\s+([^:]+):\s+(.+)'
        match = re.search(pattern2, clean_line)
        if not match:
            return None
        timestamp, level, module, message = match.groups()
        resource_ref = None
    
    # Extract reconciler from module path
    # Pattern: netbox_controller::reconciler::ipam::ip_address -> ipam::ip_address
    # Pattern: netbox_controller::reconciler::dcim::device -> dcim::device
    # Try full path first (ipam::ip_address)
    reconciler_match = re.search(r'netbox_controller::reconciler::([^:]+::[^:]+)', module)
    if reconciler_match:
        reconciler = reconciler_match.group(1)
    else:
        # Try single level (ipam, dcim, etc.)
        reconciler_match = re.search(r'netbox_controller::reconciler::([^:]+)', module)
        if reconciler_match:
            reconciler = reconciler_match.group(1)
        elif 'watcher' in module:
            reconciler = 'watcher'
        elif 'token_resolver' in module:
            reconciler = 'token_resolver'
        elif 'reconcile_helpers' in module:
            reconciler = 'reconcile_helpers'
        else:
            reconciler = 'unknown'
    
    # Extract resource name from resource_ref
    # Format: NetBoxIPAddress.v1alpha1.dcops.microscaler.io/dhcp-client-ip-static.default
    resource = None
    if resource_ref:
        # Split by / to get namespace/name part
        if '/' in resource_ref:
            name_part = resource_ref.split('/')[-1]
            resource = name_part
        else:
            parts = resource_ref.split('.')
            if len(parts) >= 2:
                resource = parts[-1]  # Get the resource name
    
    # Extract error details if present
    error_details = None
    if level == "ERROR" and "error:" in message.lower():
        error_match = re.search(r'error[:\s]+(.+)', message, re.IGNORECASE)
        if error_match:
            error_details = error_match.group(1)
    
    return LogEntry(
        timestamp=timestamp,
        level=level,
        reconciler=reconciler,
        resource=resource,
        message=message,
        error_details=error_details
    )

def get_netbox_url() -> str:
    """Get NetBox URL from deployment or default."""
    try:
        result = subprocess.run(
            ["kubectl", "get", "deployment", "netbox-controller", "-n", "dcops-system",
             "-o", "jsonpath={.spec.template.spec.containers[0].env[?(@.name==\"NETBOX_URL\")].value}"],
            capture_output=True,
            text=True,
            check=False
        )
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.strip()
    except Exception:
        pass
    return "http://netbox.netbox:80"

def get_netbox_token() -> Optional[str]:
    """Get NetBox token from Kubernetes secret."""
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
                import base64
                try:
                    token = base64.b64decode(result.stdout.strip()).decode('utf-8')
                    if token and token != "PLACEHOLDER_DO_NOT_USE" and len(token) > 10:
                        return token
                except Exception:
                    continue
        except Exception:
            continue
    
    # Try getting from pod environment
    try:
        result = subprocess.run(
            ["kubectl", "get", "pod", "-l", "app=netbox-controller", "-n", "dcops-system",
             "-o", "jsonpath={.items[0].spec.containers[0].env[?(@.name==\"NETBOX_TOKEN\")].valueFrom.secretKeyRef.name}"],
            capture_output=True,
            text=True,
            check=False
        )
        if result.returncode == 0 and result.stdout.strip():
            secret_name = result.stdout.strip()
            result2 = subprocess.run(
                ["kubectl", "get", "secret", secret_name, "-n", "dcops-system",
                 "-o", "jsonpath={.data.token}"],
                capture_output=True,
                text=True,
                check=False
            )
            if result2.returncode == 0 and result2.stdout.strip():
                import base64
                token = base64.b64decode(result2.stdout.strip()).decode('utf-8')
                if token and token != "PLACEHOLDER_DO_NOT_USE" and len(token) > 10:
                    return token
    except Exception:
        pass
    
    return None

def query_netbox_api(base_url: str, token: str, endpoint: str) -> List[Dict]:
    """Query NetBox API and return all results."""
    url = urljoin(base_url.rstrip('/') + '/', endpoint.lstrip('/'))
    headers = {
        "Authorization": f"Token {token}",
        "Accept": "application/json",
        "Content-Type": "application/json",
    }
    
    resources = []
    next_url = url
    
    while next_url:
        try:
            response = requests.get(next_url, headers=headers, timeout=10)
            response.raise_for_status()
            data = response.json()
            
            if 'results' in data:
                resources.extend(data['results'])
                next_url = data.get('next')
            else:
                # Single resource
                resources.append(data)
                break
        except Exception as e:
            print(f"⚠️  Error querying {url}: {e}", file=sys.stderr)
            break
    
    return resources

def analyze_logs(log_file: Path) -> Dict[str, ReconcilerStats]:
    """Parse logs and organize by reconciler."""
    stats = defaultdict(lambda: ReconcilerStats(name=""))
    
    with open(log_file, 'r') as f:
        for line in f:
            entry = parse_log_line(line)
            if not entry:
                continue
            
            if stats[entry.reconciler].name == "":
                stats[entry.reconciler].name = entry.reconciler
            
            stats[entry.reconciler].total_logs += 1
            
            if entry.level == "ERROR":
                stats[entry.reconciler].errors.append(entry)
            elif entry.level == "WARN":
                stats[entry.reconciler].warnings.append(entry)
            elif entry.level == "INFO":
                stats[entry.reconciler].info.append(entry)
    
    return dict(stats)

def extract_resource_names_from_logs(stats: Dict[str, ReconcilerStats]) -> Dict[str, Set[str]]:
    """Extract expected resource names from log messages."""
    resource_map = defaultdict(set)
    
    for reconciler_stat in stats.values():
        for entry in reconciler_stat.errors + reconciler_stat.warnings + reconciler_stat.info:
            # Extract resource names from various log patterns
            if entry.resource:
                # Format: NetBoxResourceType.name.namespace
                parts = entry.resource.split('.')
                if len(parts) >= 2:
                    resource_type = parts[0]
                    resource_name = parts[1]
                    resource_map[resource_type].add(resource_name)
    
    return dict(resource_map)

def main():
    """Main analysis function."""
    project_root = Path(__file__).parent.parent
    log_file = project_root / "tilt-netbox-controller.logs"
    
    if not log_file.exists():
        print(f"❌ Log file not found: {log_file}")
        sys.exit(1)
    
    print("📊 Analyzing NetBox Controller Logs...")
    print(f"📁 Log file: {log_file}")
    print()
    
    # Parse logs
    print("🔍 Parsing logs...")
    stats = analyze_logs(log_file)
    
    # Get NetBox connection info
    print("🔗 Getting NetBox connection info...")
    netbox_url = get_netbox_url()
    netbox_token = get_netbox_token()
    
    if not netbox_token:
        print("⚠️  Warning: Could not get NetBox token, API queries will be skipped")
        netbox_token = "PLACEHOLDER"
    
    print(f"🌐 NetBox URL: {netbox_url}")
    print(f"🔑 Token: {'***' if netbox_token != 'PLACEHOLDER' else 'NOT FOUND'}")
    print()
    
    # Extract expected resources
    expected_resources = extract_resource_names_from_logs(stats)
    
    # Query NetBox API for each resource type
    print("📡 Querying NetBox API...")
    netbox_data = {}
    
    if netbox_token != "PLACEHOLDER":
        for resource_type, endpoint in NETBOX_ENDPOINTS.items():
            print(f"  Querying {resource_type}...", end=" ")
            resources = query_netbox_api(netbox_url, netbox_token, endpoint)
            netbox_data[resource_type] = resources
            print(f"Found {len(resources)} resources")
    
    print()
    
    # Generate report
    print("=" * 80)
    print("RECONCILER ANALYSIS REPORT")
    print("=" * 80)
    print()
    
    # Summary table
    print("📋 SUMMARY BY RECONCILER")
    print("-" * 80)
    print(f"{'Reconciler':<30} {'Total':<10} {'Errors':<10} {'Warnings':<10} {'Info':<10}")
    print("-" * 80)
    
    for reconciler_name in sorted(stats.keys()):
        stat = stats[reconciler_name]
        print(f"{reconciler_name:<30} {stat.total_logs:<10} {len(stat.errors):<10} {len(stat.warnings):<10} {len(stat.info):<10}")
    
    print()
    
    # Detailed analysis per reconciler
    for reconciler_name in sorted(stats.keys()):
        stat = stats[reconciler_name]
        
        print("=" * 80)
        print(f"RECONCILER: {reconciler_name.upper()}")
        print("=" * 80)
        print(f"Total logs: {stat.total_logs}")
        print(f"Errors: {len(stat.errors)}")
        print(f"Warnings: {len(stat.warnings)}")
        print(f"Info: {len(stat.info)}")
        print()
        
        # Errors
        if stat.errors:
            print("❌ ERRORS:")
            unique_errors = {}
            for entry in stat.errors:
                error_key = f"{entry.resource}: {entry.message[:100]}"
                if error_key not in unique_errors:
                    unique_errors[error_key] = entry
            
            for error_key, entry in list(unique_errors.items())[:10]:  # Show top 10
                print(f"  • {entry.resource or 'N/A'}: {entry.message[:150]}")
                if entry.error_details:
                    print(f"    Details: {entry.error_details[:200]}")
            if len(unique_errors) > 10:
                print(f"  ... and {len(unique_errors) - 10} more errors")
            print()
        
        # Warnings
        if stat.warnings:
            print("⚠️  WARNINGS:")
            unique_warnings = {}
            for entry in stat.warnings:
                warn_key = f"{entry.resource}: {entry.message[:100]}"
                if warn_key not in unique_warnings:
                    unique_warnings[warn_key] = entry
            
            for warn_key, entry in list(unique_warnings.items())[:10]:  # Show top 10
                print(f"  • {entry.resource or 'N/A'}: {entry.message[:150]}")
            if len(unique_warnings) > 10:
                print(f"  ... and {len(unique_warnings) - 10} more warnings")
            print()
        
        # NetBox API state
        if netbox_token != "PLACEHOLDER":
            # Find matching resource type
            resource_type = None
            for rt in NETBOX_ENDPOINTS.keys():
                if reconciler_name.replace('_', '').replace('netbox', '').lower() in rt.lower().replace('netbox', '').lower():
                    resource_type = rt
                    break
            
            if resource_type and resource_type in netbox_data:
                resources = netbox_data[resource_type]
                print(f"📦 NETBOX API STATE ({resource_type}):")
                print(f"  Found {len(resources)} resources in NetBox")
                
                if resources:
                    print("  Sample resources:")
                    for res in resources[:5]:
                        name = res.get('name') or res.get('display', 'N/A')
                        id_val = res.get('id', 'N/A')
                        print(f"    • {name} (ID: {id_val})")
                    if len(resources) > 5:
                        print(f"    ... and {len(resources) - 5} more")
                print()
        
        print()
    
    # Cross-cutting issues
    print("=" * 80)
    print("CROSS-CUTTING ISSUES")
    print("=" * 80)
    print()
    
    # Missing tags
    if netbox_token != "PLACEHOLDER" and "NetBoxTag" in netbox_data:
        netbox_tags = {tag.get('name') for tag in netbox_data["NetBoxTag"]}
        missing_tags = set()
        
        for stat in stats.values():
            for entry in stat.warnings:
                if "not found in NetBox" in entry.message and "Tag" in entry.message:
                    tag_match = re.search(r"Tag '([^']+)'", entry.message)
                    if tag_match:
                        tag_name = tag_match.group(1)
                        if tag_name not in netbox_tags:
                            missing_tags.add(tag_name)
        
        if missing_tags:
            print("🏷️  MISSING TAGS:")
            for tag in sorted(missing_tags):
                print(f"  • {tag}")
            print()
    
    # Missing CRDs
    missing_crds = set()
    for stat in stats.values():
        for entry in stat.warnings:
            if "CRD" in entry.message and "not found" in entry.message:
                crd_match = re.search(r"CRD '([^']+)'", entry.message)
                if crd_match:
                    missing_crds.add(crd_match.group(1))
    
    if missing_crds:
        print("📋 MISSING CRDs (referenced but not found):")
        for crd in sorted(missing_crds):
            print(f"  • {crd}")
        print()
    
    # NetBox validation errors
    netbox_validation_errors = []
    for stat in stats.values():
        for entry in stat.errors:
            if "NetBox API error" in entry.message or "400 Bad Request" in entry.message:
                netbox_validation_errors.append(entry)
    
    if netbox_validation_errors:
        print("🔴 NETBOX VALIDATION ERRORS:")
        unique_errors = {}
        for entry in netbox_validation_errors:
            error_key = entry.message[:200]
            if error_key not in unique_errors:
                unique_errors[error_key] = entry
        
        for error_key, entry in list(unique_errors.items())[:5]:
            print(f"  • {entry.resource or 'N/A'}:")
            print(f"    {entry.message[:300]}")
        if len(unique_errors) > 5:
            print(f"  ... and {len(unique_errors) - 5} more validation errors")
        print()
    
    # Save detailed report
    report_file = project_root / "netbox_reconciliation_analysis.json"
    report_data = {
        "timestamp": datetime.now().isoformat(),
        "netbox_url": netbox_url,
        "reconcilers": {
            name: {
                "total_logs": stat.total_logs,
                "error_count": len(stat.errors),
                "warning_count": len(stat.warnings),
                "info_count": len(stat.info),
                "errors": [
                    {
                        "resource": e.resource,
                        "message": e.message,
                        "timestamp": e.timestamp
                    }
                    for e in stat.errors[:20]  # Limit to top 20
                ],
                "warnings": [
                    {
                        "resource": w.resource,
                        "message": w.message,
                        "timestamp": w.timestamp
                    }
                    for w in stat.warnings[:20]  # Limit to top 20
                ]
            }
            for name, stat in stats.items()
        },
        "netbox_resources": {
            resource_type: len(resources)
            for resource_type, resources in netbox_data.items()
        }
    }
    
    with open(report_file, 'w') as f:
        json.dump(report_data, f, indent=2)
    
    print(f"💾 Detailed report saved to: {report_file}")
    print()
    print("✅ Analysis complete!")

if __name__ == "__main__":
    main()

