#!/usr/bin/env python3
"""
Verify NetBox CR reconciliation status.

This script verifies that:
1. CRDs exist in Kubernetes
2. CRs exist and have status populated
3. Resources actually exist in NetBox database

Usage:
    # Verify all NetBox CRDs
    python3 scripts/verify_netbox_crs.py --all

    # Verify specific CRD type
    python3 scripts/verify_netbox_crs.py --crd netboxprefixes

    # Verify specific CR
    python3 scripts/verify_netbox_crs.py --crd netboxprefixes --name control-plane-prefix --namespace default
"""

import argparse
import json
import subprocess
import sys
from typing import Dict, List, Optional, Tuple

# Mapping of CRD types to NetBox database tables and key fields
CRD_TO_DB_MAP = {
    'netboxprefixes': {
        'table': 'ipam_prefix',
        'id_field': 'id',
        'name_field': 'prefix',  # Use prefix as identifier
        'spec_field': 'prefix',  # Field in CR spec to match
    },
    'netboxtenants': {
        'table': 'tenancy_tenant',
        'id_field': 'id',
        'name_field': 'name',
        'spec_field': 'name',
    },
    'netboxsites': {
        'table': 'dcim_site',
        'id_field': 'id',
        'name_field': 'name',
        'spec_field': 'name',
    },
    'netboxroles': {
        'table': 'ipam_role',
        'id_field': 'id',
        'name_field': 'name',
        'spec_field': 'name',
    },
    'netboxtags': {
        'table': 'extras_tag',
        'id_field': 'id',
        'name_field': 'name',
        'spec_field': 'name',
    },
    'netboxaggregates': {
        'table': 'ipam_aggregate',
        'id_field': 'id',
        'name_field': 'prefix',  # Aggregate uses prefix as identifier
        'spec_field': 'prefix',
    },
    'netboxvlans': {
        'table': 'ipam_vlan',
        'id_field': 'id',
        'name_field': 'vid',  # VLAN uses VID as identifier
        'spec_field': 'vid',
    },
    'netboxdeviceroles': {
        'table': 'dcim_devicerole',
        'id_field': 'id',
        'name_field': 'name',
        'spec_field': 'name',
    },
    'netboxmanufacturers': {
        'table': 'dcim_manufacturer',
        'id_field': 'id',
        'name_field': 'name',
        'spec_field': 'name',
    },
    'netboxplatforms': {
        'table': 'dcim_platform',
        'id_field': 'id',
        'name_field': 'name',
        'spec_field': 'name',
    },
    'netboxdevicetypes': {
        'table': 'dcim_devicetype',
        'id_field': 'id',
        'name_field': 'model',  # Device type uses model as identifier
        'spec_field': 'model',
    },
    'netboxregions': {
        'table': 'dcim_region',
        'id_field': 'id',
        'name_field': 'name',
        'spec_field': 'name',
    },
    'netboxsitegroups': {
        'table': 'dcim_sitegroup',
        'id_field': 'id',
        'name_field': 'name',
        'spec_field': 'name',
    },
    'netboxlocations': {
        'table': 'dcim_location',
        'id_field': 'id',
        'name_field': 'name',
        'spec_field': 'name',
    },
}

def log_info(message):
    print(f"ℹ️  {message}")

def log_error(message):
    print(f"❌ {message}", file=sys.stderr)

def log_success(message):
    print(f"✅ {message}")

def log_warning(message):
    print(f"⚠️  {message}")

def run_kubectl(cmd: List[str], json_output: bool = True) -> Optional[Dict]:
    """Run kubectl command and return JSON output."""
    cmd_list = ['kubectl'] + cmd
    if json_output:
        cmd_list.extend(['-o', 'json'])
    
    try:
        result = subprocess.run(
            cmd_list,
            capture_output=True,
            text=True,
            check=True
        )
        if json_output:
            return json.loads(result.stdout)
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        log_error(f"kubectl command failed: {' '.join(cmd_list)}")
        log_error(f"Error: {e.stderr}")
        return None
    except json.JSONDecodeError as e:
        log_error(f"Failed to parse JSON: {e}")
        return None

def get_postgres_pod(namespace: str = 'netbox') -> Optional[str]:
    """Get the name of the PostgreSQL pod."""
    result = run_kubectl(
        ['get', 'pod', '-n', namespace, '-l', 'app=postgres', '-o', 'jsonpath={.items[0].metadata.name}'],
        json_output=False
    )
    return result

def run_psql_query(postgres_pod: str, namespace: str, query: str, 
                   dbname: str = 'netbox', user: str = 'netbox', password: str = 'netbox') -> Optional[str]:
    """Run a PostgreSQL query using kubectl exec."""
    import os
    env = os.environ.copy()
    env['PGPASSWORD'] = password
    
    cmd = [
        'kubectl', 'exec', '-n', namespace, postgres_pod,
        '--', 'psql', '-U', user, '-d', dbname, '-t', '-A', '-c', query
    ]
    
    try:
        result = subprocess.run(
            cmd,
            env=env,
            check=True,
            capture_output=True,
            text=True
        )
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        log_error(f"Failed to run psql query: {e.stderr}")
        return None

def verify_crd_exists(crd_name: str) -> bool:
    """Verify CRD exists in Kubernetes."""
    crd = run_kubectl(['get', 'crd', crd_name])
    if not crd:
        log_error(f"CRD {crd_name} not found")
        return False
    log_success(f"CRD {crd_name} exists")
    return True

def verify_cr_status(cr: Dict, crd_type: str) -> Tuple[bool, Optional[int], Optional[str], Optional[str]]:
    """Verify CR has proper status and return netbox_id, identifier, and state.
    
    Returns: (success, netbox_id, identifier, state)
    """
    metadata = cr.get('metadata', {})
    name = metadata.get('name', 'unknown')
    namespace = metadata.get('namespace', 'default')
    
    status = cr.get('status', {})
    netbox_id = status.get('netboxId') or status.get('netbox_id')  # Handle both camelCase and snake_case
    
    if not netbox_id:
        log_error(f"CR {namespace}/{name} missing netboxId in status")
        return False, None, None, None
    
    state = status.get('state', 'Unknown')
    # Check if state is 'Created' (case-insensitive for comparison, but warn on mismatch)
    state_normalized = state.lower() if state else ''
    is_created = state_normalized == 'created'
    is_failed = state_normalized == 'failed'
    
    if is_failed:
        log_error(f"CR {namespace}/{name} state is '{state}' (FAILED)")
    elif not is_created:
        log_warning(f"CR {namespace}/{name} state is '{state}', expected 'Created'")
    
    # Get identifier from spec
    spec = cr.get('spec', {})
    db_map = CRD_TO_DB_MAP.get(crd_type, {})
    spec_field = db_map.get('spec_field', 'name')
    identifier = spec.get(spec_field)
    
    if not identifier:
        log_warning(f"CR {namespace}/{name} missing '{spec_field}' in spec")
        return True, netbox_id, None, state
    
    if is_created and not is_failed:
        log_success(f"CR {namespace}/{name} has status (netboxId: {netbox_id}, state: {state})")
    return True, netbox_id, identifier, state

def verify_in_netbox_db(crd_type: str, netbox_id: int, identifier: str, 
                        postgres_pod: str, namespace: str = 'netbox') -> bool:
    """Verify resource exists in NetBox database."""
    db_map = CRD_TO_DB_MAP.get(crd_type)
    if not db_map:
        log_warning(f"No database mapping for {crd_type}, skipping DB verification")
        return True  # Don't fail if we don't have mapping
    
    table = db_map['table']
    id_field = db_map['id_field']
    name_field = db_map['name_field']
    
    # Build query - escape single quotes in identifier
    # Convert identifier to string if it's not already (handles int/None cases)
    identifier_str = str(identifier) if identifier is not None else ''
    identifier_escaped = identifier_str.replace("'", "''")
    query = f"SELECT {id_field}, {name_field} FROM {table} WHERE {id_field} = {netbox_id} AND {name_field} = '{identifier_escaped}';"
    
    result = run_psql_query(postgres_pod, namespace, query)
    if not result or not result.strip():
        log_error(f"Resource not found in NetBox database (ID: {netbox_id}, {name_field}: {identifier})")
        return False
    
    log_success(f"Resource exists in NetBox database (ID: {netbox_id}, {name_field}: {identifier})")
    return True

def verify_crd_type(crd_type: str, namespace: str = 'default', 
                    specific_name: Optional[str] = None,
                    postgres_pod: Optional[str] = None,
                    netbox_namespace: str = 'netbox') -> Tuple[bool, List[str], List[str], List[str]]:
    """Verify all CRs of a specific CRD type.
    
    Returns: (success, failures, warnings, missing)
    """
    print(f"\n{'='*60}")
    print(f"Verifying {crd_type}")
    print(f"{'='*60}")
    
    failures = []
    warnings = []
    missing = []
    
    # Verify CRD exists
    crd_name = f"{crd_type}.dcops.microscaler.io"
    if not verify_crd_exists(crd_name):
        failures.append(f"CRD {crd_name} does not exist")
        return False, failures, warnings, missing
    
    # Get all CRs
    if specific_name:
        crs = run_kubectl(['get', crd_type, f'{namespace}/{specific_name}', '-n', namespace])
        if not crs:
            failures.append(f"CR {namespace}/{specific_name} not found")
            return False, failures, warnings, missing
        crs_list = [crs]
    else:
        crs_list_obj = run_kubectl(['get', crd_type, '-A'])
        if not crs_list_obj:
            log_info(f"No CRs found for {crd_type}")
            return True, failures, warnings, missing  # Not an error if no CRs exist
        crs_list = crs_list_obj.get('items', [])
    
    if not crs_list:
        log_info(f"No CRs found for {crd_type}")
        return True, failures, warnings, missing
    
    # Get PostgreSQL pod if needed
    if not postgres_pod:
        postgres_pod = get_postgres_pod(netbox_namespace)
        if not postgres_pod:
            failures.append("Could not find PostgreSQL pod")
            return False, failures, warnings, missing
    
    # Verify each CR
    for cr in crs_list:
        metadata = cr.get('metadata', {})
        name = metadata.get('name', 'unknown')
        cr_namespace = metadata.get('namespace', 'default')
        cr_full_name = f"{cr_namespace}/{name}"
        
        print(f"\n  Checking {cr_full_name}...")
        
        # Check status
        has_status, netbox_id, identifier, state = verify_cr_status(cr, crd_type)
        if not has_status:
            failures.append(f"{crd_type}/{cr_full_name}: missing netboxId in status")
            continue
        
        if not netbox_id:
            failures.append(f"{crd_type}/{cr_full_name}: netboxId is None")
            continue
        
        # Check state
        if state:
            state_lower = state.lower()
            if state_lower == 'failed':
                failures.append(f"{crd_type}/{cr_full_name}: state is 'failed'")
            elif state_lower != 'created':
                warnings.append(f"{crd_type}/{cr_full_name}: state is '{state}' (expected 'Created')")
        
        # Verify in database
        if identifier:
            if not verify_in_netbox_db(crd_type, netbox_id, identifier, postgres_pod, netbox_namespace):
                failures.append(f"{crd_type}/{cr_full_name}: not found in NetBox database (ID: {netbox_id})")
        else:
            warnings.append(f"{crd_type}/{cr_full_name}: skipping DB verification (no identifier)")
    
    success = len(failures) == 0
    return success, failures, warnings, missing

def verify_all_crds(namespace: str = 'default', netbox_namespace: str = 'netbox') -> Tuple[bool, List[str], List[str], List[str]]:
    """Verify all NetBox CRD types.
    
    Returns: (success, failures, warnings, missing)
    """
    print("="*60)
    print("Verifying All NetBox CRDs")
    print("="*60)
    
    # Get PostgreSQL pod once
    postgres_pod = get_postgres_pod(netbox_namespace)
    if not postgres_pod:
        log_error("Could not find PostgreSQL pod")
        return False, ["Could not find PostgreSQL pod"], [], []
    
    log_info(f"Using PostgreSQL pod: {postgres_pod}")
    
    all_failures = []
    all_warnings = []
    all_missing = []
    
    for crd_type in sorted(CRD_TO_DB_MAP.keys()):
        success, failures, warnings, missing = verify_crd_type(
            crd_type, namespace, postgres_pod=postgres_pod, netbox_namespace=netbox_namespace
        )
        all_failures.extend(failures)
        all_warnings.extend(warnings)
        all_missing.extend(missing)
    
    success = len(all_failures) == 0
    return success, all_failures, all_warnings, all_missing

def main():
    parser = argparse.ArgumentParser(
        description='Verify NetBox CR reconciliation status',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Verify all NetBox CRDs
  python3 scripts/verify_netbox_crs.py --all

  # Verify specific CRD type
  python3 scripts/verify_netbox_crs.py --crd netboxprefixes

  # Verify specific CR
  python3 scripts/verify_netbox_crs.py --crd netboxprefixes --name control-plane-prefix

  # Verify with custom namespaces
  python3 scripts/verify_netbox_crs.py --all --namespace default --netbox-namespace netbox
        """
    )
    
    parser.add_argument(
        '--all',
        action='store_true',
        help='Verify all NetBox CRD types'
    )
    parser.add_argument(
        '--crd',
        choices=list(CRD_TO_DB_MAP.keys()),
        help='Specific CRD type to verify'
    )
    parser.add_argument(
        '--name',
        help='Specific CR name to verify (requires --crd)'
    )
    parser.add_argument(
        '--namespace',
        default='default',
        help='Kubernetes namespace for CRs (default: default)'
    )
    parser.add_argument(
        '--netbox-namespace',
        default='netbox',
        help='Kubernetes namespace for NetBox (default: netbox)'
    )
    
    args = parser.parse_args()
    
    if args.all:
        success, failures, warnings, missing = verify_all_crds(args.namespace, args.netbox_namespace)
    elif args.crd:
        if args.name:
            success, failures, warnings, missing = verify_crd_type(
                args.crd, args.namespace, args.name, netbox_namespace=args.netbox_namespace
            )
        else:
            success, failures, warnings, missing = verify_crd_type(
                args.crd, args.namespace, netbox_namespace=args.netbox_namespace
            )
    else:
        parser.print_help()
        sys.exit(1)
    
    # Print summary
    print("\n" + "="*60)
    print("Verification Summary")
    print("="*60)
    
    if failures:
        print(f"\n❌ Failures ({len(failures)}):")
        for failure in failures:
            print(f"   • {failure}")
    
    if warnings:
        print(f"\n⚠️  Warnings ({len(warnings)}):")
        for warning in warnings:
            print(f"   • {warning}")
    
    if missing:
        print(f"\n📋 Missing ({len(missing)}):")
        for item in missing:
            print(f"   • {item}")
    
    if not failures and not warnings and not missing:
        log_success("All verifications passed!")
        sys.exit(0)
    elif failures:
        log_error(f"Verification failed: {len(failures)} failure(s), {len(warnings)} warning(s)")
        sys.exit(1)
    else:
        log_warning(f"Verification completed with {len(warnings)} warning(s) (no failures)")
        sys.exit(0)  # Warnings don't fail the script, but failures do

if __name__ == '__main__':
    main()

