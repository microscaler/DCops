#!/usr/bin/env python3
"""
Diagnostic script to investigate why resources are not being created in NetBox.

This script:
1. Checks CR statuses in Kubernetes
2. Identifies resources without netbox_id
3. Checks for common issues (RBAC, token resolution, etc.)
4. Provides actionable recommendations

Usage:
    python3 scripts/diagnose_missing_resources.py [--namespace NAMESPACE]
"""

import argparse
import subprocess
import json
import sys
from pathlib import Path
from typing import Dict, List, Optional, Tuple
import yaml

def run_kubectl(command: List[str]) -> Tuple[bool, str]:
    """Run a kubectl command and return success status and output."""
    try:
        result = subprocess.run(
            ["kubectl"] + command,
            capture_output=True,
            text=True,
            check=True
        )
        return True, result.stdout
    except subprocess.CalledProcessError as e:
        return False, e.stderr
    except FileNotFoundError:
        return False, "kubectl not found in PATH"

def get_cr_status(kind: str, name: str, namespace: str = "default") -> Optional[Dict]:
    """Get the status of a CR."""
    success, output = run_kubectl([
        "get", kind, name,
        "-n", namespace,
        "-o", "json"
    ])
    if not success:
        return None
    
    try:
        cr = json.loads(output)
        return cr.get("status")
    except json.JSONDecodeError:
        return None

def check_rbac(kind: str) -> Tuple[bool, str]:
    """Check if RBAC permissions exist for a CRD kind."""
    # Check ClusterRole for list permission
    success, output = run_kubectl([
        "get", "clusterrole", "netbox-controller",
        "-o", "json"
    ])
    if not success:
        return False, "Could not check ClusterRole"
    
    try:
        role = json.loads(output)
        rules = role.get("rules", [])
        
        # Check if this kind has list permission
        for rule in rules:
            api_groups = rule.get("apiGroups", [])
            resources = rule.get("resources", [])
            verbs = rule.get("verbs", [])
            
            if "dcops.microscaler.io" in api_groups:
                # Convert kind to resource name (e.g., NetBoxDevice -> netboxdevices)
                resource_name = kind.lower().replace("netbox", "netbox").replace("Box", "")
                # More accurate: NetBoxDevice -> netboxdevices
                if kind.startswith("NetBox"):
                    resource_name = kind[6:].lower() + "s"  # Remove "NetBox" prefix, add 's'
                    resource_name = "netbox" + resource_name
                
                # Check all possible resource name formats
                possible_names = [
                    resource_name,
                    kind.lower() + "s",
                    kind.lower(),
                ]
                
                for res_name in possible_names:
                    if res_name in resources and "list" in verbs:
                        return True, f"RBAC permission found for {kind}"
        
        return False, f"No RBAC list permission found for {kind}"
    except json.JSONDecodeError:
        return False, "Could not parse ClusterRole"

def check_cr_exists(kind: str, name: str, namespace: str = "default") -> Tuple[bool, Optional[Dict]]:
    """Check if a CR exists and return its full spec."""
    success, output = run_kubectl([
        "get", kind, name,
        "-n", namespace,
        "-o", "json"
    ])
    if not success:
        return False, None
    
    try:
        return True, json.loads(output)
    except json.JSONDecodeError:
        return False, None

def diagnose_resource(kind: str, name: str, namespace: str = "default") -> Dict:
    """Diagnose a single resource."""
    result = {
        "kind": kind,
        "name": name,
        "namespace": namespace,
        "exists": False,
        "has_status": False,
        "has_netbox_id": False,
        "status_state": None,
        "status_error": None,
        "rbac_ok": False,
        "rbac_message": "",
        "issues": [],
        "recommendations": []
    }
    
    # Check if CR exists
    exists, cr = check_cr_exists(kind, name, namespace)
    result["exists"] = exists
    
    if not exists:
        result["issues"].append(f"CR {kind}/{name} does not exist in namespace {namespace}")
        result["recommendations"].append(f"Create the CR: kubectl apply -f config/examples/.../{name}.yaml")
        return result
    
    # Check status
    status = cr.get("status")
    if status:
        result["has_status"] = True
        result["status_state"] = status.get("state")
        result["status_error"] = status.get("error")
        result["has_netbox_id"] = status.get("netboxId") is not None and status.get("netboxId") != 0
        
        if not result["has_netbox_id"]:
            result["issues"].append("CR exists but has no netbox_id in status")
            if result["status_state"] == "Failed":
                result["issues"].append(f"Status shows Failed state: {result['status_error']}")
                result["recommendations"].append("Check controller logs for this resource")
            elif result["status_state"] == "Pending":
                result["issues"].append("Status shows Pending - resource may be waiting for dependencies")
                result["recommendations"].append("Check if all dependencies are created")
            else:
                result["recommendations"].append("Resource may be in process of creation - check controller logs")
    else:
        result["issues"].append("CR exists but has no status field")
        result["recommendations"].append("Controller may not have reconciled this resource yet")
    
    # Check RBAC
    rbac_ok, rbac_msg = check_rbac(kind)
    result["rbac_ok"] = rbac_ok
    result["rbac_message"] = rbac_msg
    if not rbac_ok:
        result["issues"].append(f"RBAC issue: {rbac_msg}")
        result["recommendations"].append("Check and update RBAC permissions in config/rbac/")
    
    # Check spec for common issues
    spec = cr.get("spec", {})
    if not spec:
        result["issues"].append("CR has no spec field")
        result["recommendations"].append("CR spec is invalid")
    
    return result

def main():
    parser = argparse.ArgumentParser(description="Diagnose missing NetBox resources")
    parser.add_argument("--namespace", default="default", help="Namespace to check (default: default)")
    parser.add_argument("--kind", help="Specific kind to check (e.g., NetBoxDeviceRole)")
    parser.add_argument("--name", help="Specific resource name to check")
    args = parser.parse_args()
    
    # List of missing resources from reconciliation analysis
    missing_resources = [
        ("NetBoxDeviceRole", "kubernetes-control-plane"),
        ("NetBoxManufacturer", "raspberry-pi"),
        ("NetBoxPlatform", "talos-linux"),
        ("NetBoxInterface", "talos-control-plane-01-eth0"),
        ("NetBoxLocation", "datacenter-1-rack-a"),
        ("NetBoxRegion", "us-east"),
        ("NetBoxRIR", "arin"),
        ("NetBoxRole", "control-plane"),
        ("NetBoxRouteTarget", "production-rt-65000-100"),
        ("NetBoxRouteTarget", "shared-services-rt-65000-200"),
        ("NetBoxSite", "datacenter-1"),
        ("NetBoxSiteGroup", "production-sites"),
        ("NetBoxTenantGroup", "default"),
        ("NetBoxVLAN", "control-plane-vlan"),
        ("NetBoxVRF", "production-vrf"),
    ]
    
    if args.kind and args.name:
        resources_to_check = [(args.kind, args.name)]
    else:
        resources_to_check = missing_resources
    
    print("=" * 80)
    print("NetBox Resource Diagnostic Tool")
    print("=" * 80)
    print(f"Checking {len(resources_to_check)} resource(s) in namespace '{args.namespace}'\n")
    
    results = []
    for kind, name in resources_to_check:
        print(f"Diagnosing {kind}/{name}...")
        result = diagnose_resource(kind, name, args.namespace)
        results.append(result)
    
    # Print summary
    print("\n" + "=" * 80)
    print("DIAGNOSTIC SUMMARY")
    print("=" * 80)
    
    for result in results:
        print(f"\n{result['kind']}/{result['name']}:")
        print(f"  Exists: {result['exists']}")
        print(f"  Has Status: {result['has_status']}")
        print(f"  Has NetBox ID: {result['has_netbox_id']}")
        print(f"  Status State: {result['status_state']}")
        if result['status_error']:
            print(f"  Status Error: {result['status_error']}")
        print(f"  RBAC: {result['rbac_ok']} - {result['rbac_message']}")
        
        if result['issues']:
            print(f"  Issues ({len(result['issues'])}):")
            for issue in result['issues']:
                print(f"    - {issue}")
        
        if result['recommendations']:
            print(f"  Recommendations ({len(result['recommendations'])}):")
            for rec in result['recommendations']:
                print(f"    - {rec}")
    
    # Overall statistics
    print("\n" + "=" * 80)
    print("OVERALL STATISTICS")
    print("=" * 80)
    total = len(results)
    exists = sum(1 for r in results if r['exists'])
    has_status = sum(1 for r in results if r['has_status'])
    has_netbox_id = sum(1 for r in results if r['has_netbox_id'])
    rbac_ok = sum(1 for r in results if r['rbac_ok'])
    
    print(f"Total resources checked: {total}")
    print(f"  - CRs exist: {exists}/{total} ({exists*100//total if total > 0 else 0}%)")
    print(f"  - Have status: {has_status}/{total} ({has_status*100//total if total > 0 else 0}%)")
    print(f"  - Have netbox_id: {has_netbox_id}/{total} ({has_netbox_id*100//total if total > 0 else 0}%)")
    print(f"  - RBAC OK: {rbac_ok}/{total} ({rbac_ok*100//total if total > 0 else 0}%)")
    
    resources_without_netbox_id = [r for r in results if r['exists'] and not r['has_netbox_id']]
    if resources_without_netbox_id:
        print(f"\nResources without netbox_id ({len(resources_without_netbox_id)}):")
        for r in resources_without_netbox_id:
            print(f"  - {r['kind']}/{r['name']} (state: {r['status_state']})")
    
    print("\n" + "=" * 80)
    print("Next Steps:")
    print("=" * 80)
    print("1. Check controller logs: kubectl logs -n dcops-system deployment/netbox-controller")
    print("2. Check RBAC: kubectl get clusterrole netbox-controller -o yaml")
    print("3. Check CR statuses: kubectl get <kind> <name> -o yaml")
    print("4. Verify token resolution: Check if secrets exist for tenant references")
    print("5. Check NetBox API connectivity: Verify NetBox service is accessible")

if __name__ == "__main__":
    main()
