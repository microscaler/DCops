#!/usr/bin/env python3
"""
Simple example: Verify NetBoxPrefix CR reconciliation.

This is a simplified example script demonstrating basic verification.
For comprehensive verification, use scripts/verify_netbox_crs.py instead.

Checks:
1. CRD exists
2. CR exists and has status
3. Resource exists in NetBox database

Usage:
    python3 scripts/verify_netbox_prefix_simple.py [cr-name] [namespace]
    
Example:
    python3 scripts/verify_netbox_prefix_simple.py control-plane-prefix default
"""

import subprocess
import sys
import json

def run_kubectl(cmd):
    """Run kubectl command and return JSON output."""
    result = subprocess.run(
        ['kubectl'] + cmd + ['-o', 'json'],
        capture_output=True,
        text=True
    )
    if result.returncode != 0:
        return None
    return json.loads(result.stdout)

def verify_crd():
    """Verify CRD exists."""
    crd = run_kubectl(['get', 'crd', 'netboxprefixes.dcops.microscaler.io'])
    if not crd:
        print("❌ CRD not found")
        return False
    print("✅ CRD exists")
    return True

def verify_cr(name, namespace='default'):
    """Verify CR exists and has status."""
    cr = run_kubectl(['get', 'netboxprefix', f'{namespace}/{name}'])
    if not cr:
        print(f"❌ CR {namespace}/{name} not found")
        return False
    
    status = cr.get('status', {})
    if not status.get('netboxId'):
        print(f"❌ CR {namespace}/{name} missing netboxId in status")
        return False
    
    if status.get('state') != 'Created':
        print(f"❌ CR {namespace}/{name} state is {status.get('state')}, expected 'Created'")
        return False
    
    print(f"✅ CR {namespace}/{name} exists with status")
    return True

def verify_in_netbox_db(netbox_id, prefix_cidr):
    """Verify resource exists in NetBox database."""
    # Get PostgreSQL pod
    result = subprocess.run(
        ['kubectl', 'get', 'pod', '-n', 'netbox', '-l', 'app=postgres', '-o', 'jsonpath={.items[0].metadata.name}'],
        capture_output=True,
        text=True
    )
    if result.returncode != 0:
        print("❌ Could not find PostgreSQL pod")
        return False
    
    postgres_pod = result.stdout.strip()
    
    # Query database
    query = f"SELECT id, prefix FROM ipam_prefix WHERE id = {netbox_id} AND prefix = '{prefix_cidr}';"
    result = subprocess.run(
        ['kubectl', 'exec', '-n', 'netbox', postgres_pod, '--', 'psql', '-U', 'netbox', '-d', 'netbox', '-t', '-A', '-c', query],
        capture_output=True,
        text=True
    )
    
    if result.returncode != 0 or not result.stdout.strip():
        print(f"❌ Resource not found in NetBox database (ID: {netbox_id})")
        return False
    
    print(f"✅ Resource exists in NetBox database (ID: {netbox_id})")
    return True

def main():
    import argparse
    
    parser = argparse.ArgumentParser(description='Verify NetBoxPrefix CR reconciliation (simple example)')
    parser.add_argument('cr_name', nargs='?', default='control-plane-prefix', help='CR name (default: control-plane-prefix)')
    parser.add_argument('--namespace', default='default', help='Kubernetes namespace (default: default)')
    
    args = parser.parse_args()
    
    if not verify_crd():
        sys.exit(1)
    
    if not verify_cr(args.cr_name, args.namespace):
        sys.exit(1)
    
    # Get netbox_id from CR status
    cr = run_kubectl(['get', 'netboxprefix', f'{args.namespace}/{args.cr_name}'])
    netbox_id = cr['status']['netboxId']
    prefix_cidr = cr['spec']['prefix']
    
    if not verify_in_netbox_db(netbox_id, prefix_cidr):
        sys.exit(1)
    
    print("\n✅ All verifications passed!")

if __name__ == '__main__':
    main()

