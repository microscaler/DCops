#!/usr/bin/env python3
"""
Creates Kubernetes Secret for NetBox tenant token.

This script retrieves a NetBox API token from the database and creates
a Kubernetes Secret that can be referenced by NetBoxTenant CRDs.

Usage:
    python3 scripts/create_tenant_secret.py --tenant-name TENANT_NAME --secret-name SECRET_NAME [options]
"""

import argparse
import subprocess
import sys
import os

# Import functions from get_netbox_token_from_db.py
sys.path.insert(0, os.path.dirname(__file__))
from get_netbox_token_from_db import (
    get_postgres_pod,
    get_token_from_db,
    log_info,
    log_error,
    log_success,
)

def create_secret(token, namespace, secret_name):
    """Create or update Kubernetes secret with token."""
    if not token:
        log_error("No token provided")
        return False
    
    secret_yaml = f"""apiVersion: v1
kind: Secret
metadata:
  name: {secret_name}
  namespace: {namespace}
type: Opaque
stringData:
  token: {token}
"""
    
    try:
        result = subprocess.run(
            ['kubectl', 'apply', '-f', '-'],
            input=secret_yaml.encode(),
            check=True,
            capture_output=True
        )
        log_success(f"Created/updated secret {secret_name} in namespace {namespace}")
        return True
    except subprocess.CalledProcessError as e:
        error_msg = e.stderr.decode() if e.stderr else 'Unknown error'
        log_error(f"Failed to create secret: {error_msg}")
        return False

def main():
    parser = argparse.ArgumentParser(
        description='Create Kubernetes Secret for NetBox tenant token'
    )
    parser.add_argument(
        '--tenant-name',
        required=True,
        help='Name of the tenant (used for token description lookup)'
    )
    parser.add_argument(
        '--secret-name',
        required=True,
        help='Name of the Kubernetes Secret to create'
    )
    parser.add_argument(
        '--namespace',
        default='default',
        help='Kubernetes namespace for secret (default: default)'
    )
    parser.add_argument(
        '--netbox-namespace',
        default='netbox',
        help='NetBox namespace (default: netbox)'
    )
    parser.add_argument(
        '--username',
        default='admin',
        help='NetBox username (default: admin)'
    )
    parser.add_argument(
        '--token',
        help='Token value (if not provided, will retrieve from database)'
    )
    parser.add_argument(
        '--token-description',
        help='Token description to search for in database (default: uses tenant-name)'
    )
    parser.add_argument(
        '--postgres-db',
        default='netbox',
        help='Database name (default: netbox)'
    )
    parser.add_argument(
        '--postgres-user',
        default='netbox',
        help='Database user (default: netbox)'
    )
    parser.add_argument(
        '--postgres-password',
        default='netbox',
        help='Database password (default: netbox)'
    )
    
    args = parser.parse_args()
    
    # Get token
    if args.token:
        log_info(f"Using provided token for tenant {args.tenant_name}")
        token = args.token
    else:
        description = args.token_description or f"DCops Controller API token for {args.tenant_name}"
        log_info(f"Retrieving token from NetBox database (description: '{description}')")
        
        # Get postgres pod
        postgres_pod = get_postgres_pod(args.netbox_namespace)
        if not postgres_pod:
            log_error(f"Failed to find PostgreSQL pod in namespace {args.netbox_namespace}")
            sys.exit(1)
        
        log_info(f"Using PostgreSQL pod: {postgres_pod}")
        
        # Get token from database
        token = get_token_from_db(
            postgres_pod,
            args.netbox_namespace,
            description,
            args.username,
            args.postgres_db,
            args.postgres_user,
            args.postgres_password
        )
        
        # If not found by description, try getting most recent token
        if not token and description:
            log_info(f"Token with description '{description}' not found, trying most recent token...")
            token = get_token_from_db(
                postgres_pod,
                args.netbox_namespace,
                None,  # No description filter
                args.username,
                args.postgres_db,
                args.postgres_user,
                args.postgres_password
            )
        
        if not token:
            log_error(f"No token found for tenant {args.tenant_name}")
            log_error("Please create a token in NetBox UI or provide one with --token:")
            log_error(f"  1. Go to NetBox UI and create a token")
            log_error(f"  2. Set description to '{description}' (optional)")
            log_error(f"  3. Run: python3 scripts/create_tenant_secret.py --tenant-name {args.tenant_name} --secret-name {args.secret_name} --token <your-token>")
            sys.exit(1)
    
    # Create secret
    log_info(f"Creating secret {args.secret_name} in namespace {args.namespace}")
    if create_secret(token, args.namespace, args.secret_name):
        log_success(f"Tenant secret management complete for {args.tenant_name}!")
        log_info(f"   Secret: {args.secret_name} in namespace {args.namespace}")
        sys.exit(0)
    else:
        sys.exit(1)

if __name__ == '__main__':
    main()

