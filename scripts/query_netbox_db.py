#!/usr/bin/env python3
"""
Quick NetBox database query script for dev validation.

Usage:
    # Query a specific resource by ID
    python3 scripts/query_netbox_db.py --table dcim_site --id 1
    
    # Query by name
    python3 scripts/query_netbox_db.py --table dcim_site --name "Data Center 1"
    
    # List all resources in a table
    python3 scripts/query_netbox_db.py --table dcim_site --list
    
    # Query tenant by ID
    python3 scripts/query_netbox_db.py --table tenancy_tenant --id 2
    
    # Query site with tenant info
    python3 scripts/query_netbox_db.py --table dcim_site --id 1 --include-relations
"""

import argparse
import subprocess
import sys
import os

def get_postgres_pod(namespace='netbox'):
    """Get the name of the PostgreSQL pod."""
    result = subprocess.run(
        ['kubectl', 'get', 'pod', '-n', namespace, '-l', 'app=postgres', '-o', 'jsonpath={.items[0].metadata.name}'],
        capture_output=True,
        text=True
    )
    if result.returncode != 0:
        return None
    return result.stdout.strip()

def run_psql_query(postgres_pod, namespace, query, dbname='netbox', user='netbox', password='netbox'):
    """Run a PostgreSQL query using kubectl exec."""
    env = os.environ.copy()
    env['PGPASSWORD'] = password
    
    cmd = [
        'kubectl', 'exec', '-n', namespace, postgres_pod,
        '--', 'psql', '-U', user, '-d', dbname, '-t', '-A', '-F', '|', '-c', query
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
        print(f"❌ Failed to run psql query: {e.stderr}", file=sys.stderr)
        return None

def query_by_id(table, resource_id, include_relations=False):
    """Query a resource by ID."""
    postgres_pod = get_postgres_pod()
    if not postgres_pod:
        print("❌ Could not find PostgreSQL pod", file=sys.stderr)
        return
    
    # Get all columns for the table
    query = f"SELECT * FROM {table} WHERE id = {resource_id};"
    result = run_psql_query(postgres_pod, 'netbox', query)
    
    if not result or not result.strip():
        print(f"❌ No resource found with ID {resource_id} in {table}")
        return
    
    # Print result in a readable format
    lines = result.strip().split('\n')
    if lines:
        # Get column names
        col_query = f"SELECT column_name FROM information_schema.columns WHERE table_name = '{table}' ORDER BY ordinal_position;"
        cols_result = run_psql_query(postgres_pod, 'netbox', col_query)
        if cols_result:
            columns = [c.strip() for c in cols_result.strip().split('\n') if c.strip()]
            values = lines[0].split('|')
            
            print(f"\n✅ Resource found in {table} (ID: {resource_id}):")
            print("-" * 60)
            for col, val in zip(columns, values):
                if val and val.strip():
                    print(f"  {col:30} = {val}")
            
            # If include_relations, show related resources
            if include_relations:
                if table == 'dcim_site':
                    # Show tenant info
                    tenant_id = None
                    for col, val in zip(columns, values):
                        if col == 'tenant_id' and val:
                            tenant_id = val.strip()
                            break
                    if tenant_id:
                        print(f"\n  Related Tenant (ID: {tenant_id}):")
                        tenant_query = f"SELECT id, name, slug FROM tenancy_tenant WHERE id = {tenant_id};"
                        tenant_result = run_psql_query(postgres_pod, 'netbox', tenant_query)
                        if tenant_result:
                            print(f"    {tenant_result}")

def query_by_name(table, name, name_field='name'):
    """Query a resource by name."""
    postgres_pod = get_postgres_pod()
    if not postgres_pod:
        print("❌ Could not find PostgreSQL pod", file=sys.stderr)
        return
    
    # Escape single quotes
    name_escaped = name.replace("'", "''")
    query = f"SELECT id, {name_field} FROM {table} WHERE {name_field} = '{name_escaped}';"
    result = run_psql_query(postgres_pod, 'netbox', query)
    
    if not result or not result.strip():
        print(f"❌ No resource found with {name_field} '{name}' in {table}")
        return
    
    print(f"\n✅ Resource found in {table}:")
    print(result)

def list_resources(table, limit=10):
    """List all resources in a table."""
    postgres_pod = get_postgres_pod()
    if not postgres_pod:
        print("❌ Could not find PostgreSQL pod", file=sys.stderr)
        return
    
    # Determine name field based on table
    name_fields = {
        'dcim_site': 'name',
        'tenancy_tenant': 'name',
        'dcim_region': 'name',
        'dcim_sitegroup': 'name',
        'dcim_location': 'name',
        'ipam_prefix': 'prefix',
        'ipam_vlan': 'vid',
        'ipam_aggregate': 'prefix',
        'dcim_manufacturer': 'name',
        'dcim_platform': 'name',
        'dcim_devicetype': 'model',
        'dcim_devicerole': 'name',
        'ipam_role': 'name',
        'extras_tag': 'name',
    }
    
    name_field = name_fields.get(table, 'name')
    query = f"SELECT id, {name_field} FROM {table} ORDER BY id LIMIT {limit};"
    result = run_psql_query(postgres_pod, 'netbox', query)
    
    if not result or not result.strip():
        print(f"❌ No resources found in {table}")
        return
    
    print(f"\n✅ Resources in {table} (showing first {limit}):")
    print(f"{'ID':<10} {name_field}")
    print("-" * 60)
    for line in result.strip().split('\n'):
        if line.strip():
            parts = line.split('|')
            if len(parts) >= 2:
                print(f"{parts[0]:<10} {parts[1]}")

def main():
    parser = argparse.ArgumentParser(description='Query NetBox database for dev validation')
    parser.add_argument('--table', required=True, help='Table name (e.g., dcim_site, tenancy_tenant)')
    parser.add_argument('--id', type=int, help='Resource ID to query')
    parser.add_argument('--name', help='Resource name to query')
    parser.add_argument('--name-field', default='name', help='Field name to use for name query (default: name)')
    parser.add_argument('--list', action='store_true', help='List all resources in table')
    parser.add_argument('--include-relations', action='store_true', help='Include related resources')
    parser.add_argument('--limit', type=int, default=10, help='Limit for list query (default: 10)')
    
    args = parser.parse_args()
    
    if args.list:
        list_resources(args.table, args.limit)
    elif args.id:
        query_by_id(args.table, args.id, args.include_relations)
    elif args.name:
        query_by_name(args.table, args.name, args.name_field)
    else:
        print("❌ Must specify --id, --name, or --list", file=sys.stderr)
        sys.exit(1)

if __name__ == '__main__':
    main()

