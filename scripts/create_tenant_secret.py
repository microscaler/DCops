#!/usr/bin/env python3
"""
Creates Kubernetes Secret for NetBox tenant token.

This script can either:
1. Create a new token via NetBox API and retrieve it from the database
2. Retrieve an existing token from the database
3. Use a provided token

Usage:
    python3 scripts/create_tenant_secret.py --tenant-name TENANT_NAME --secret-name SECRET_NAME [options]
"""

import argparse
import subprocess
import sys
import os
import requests
import time
import re

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

def create_token_via_api(netbox_url, username, password, description):
    """Create a NetBox API token via the API and return the token key."""
    session = requests.Session()
    
    try:
        # Step 1: Get CSRF token from login page
        response = session.get(f"{netbox_url}/login/", timeout=10)
        response.raise_for_status()
        
        csrf_match = re.search(r'name="csrfmiddlewaretoken" value="([^"]+)"', response.text)
        if not csrf_match:
            log_error("Could not find CSRF token on login page")
            return None
        
        csrf_token = csrf_match.group(1)
        
        # Step 2: Login
        login_data = {
            'username': username,
            'password': password,
            'csrfmiddlewaretoken': csrf_token,
        }
        response = session.post(f"{netbox_url}/login/", data=login_data, timeout=10)
        response.raise_for_status()
        
        # Check if login was successful
        if 'login' in response.url.lower():
            log_error("Login failed - still on login page")
            return None
        
        # Step 3: Get user ID
        response = session.get(f"{netbox_url}/api/users/users/?username={username}", timeout=10)
        response.raise_for_status()
        user_data = response.json()
        if not user_data.get('results'):
            log_error(f"User {username} not found")
            return None
        user_id = user_data['results'][0]['id']
        
        # Step 4: Check for existing token by description and delete it if found
        # (We'll create a fresh one to ensure it's valid)
        response = session.get(
            f"{netbox_url}/api/users/tokens/?user_id={user_id}&description={description}",
            timeout=10
        )
        response.raise_for_status()
        token_data = response.json()
        if token_data.get('results'):
            token_obj = token_data['results'][0]
            token_id = token_obj.get('id')
            log_info(f"Found existing token with description '{description}' (ID: {token_id}), deleting to create fresh one...")
            # Delete existing token
            csrf_token = session.cookies.get('csrftoken')
            if csrf_token and token_id:
                delete_headers = {
                    "X-CSRFToken": csrf_token,
                    "Referer": f"{netbox_url}/",
                }
                delete_response = session.delete(
                    f"{netbox_url}/api/users/tokens/{token_id}/",
                    headers=delete_headers,
                    timeout=10
                )
                if delete_response.status_code in (200, 204):
                    log_info("Deleted existing token")
                else:
                    log_info(f"Note: Could not delete existing token (status: {delete_response.status_code}), will create new one anyway")
        
        # Step 5: Create new token
        log_info(f"Creating new token with description '{description}'...")
        csrf_token = session.cookies.get('csrftoken')
        if not csrf_token:
            log_error("Could not get CSRF token from session")
            return None
        
        payload = {
            "user": user_id,
            "write_enabled": True,
            "description": description,
        }
        
        headers = {
            "X-CSRFToken": csrf_token,
            "Referer": f"{netbox_url}/",
            "Content-Type": "application/json",
            "Accept": "application/json",
        }
        
        response = session.post(
            f"{netbox_url}/api/users/tokens/",
            json=payload,
            headers=headers,
            timeout=10
        )
        response.raise_for_status()
        
        token_response = response.json()
        token_id = token_response.get('id')
        
        if token_id:
            log_success(f"Created token (ID: {token_id}), will retrieve key from database")
            # Return the token ID so we can query the database for the key
            return token_id
        else:
            log_error("Token created but ID not returned")
            return None
            
    except Exception as e:
        log_error(f"Failed to create token via API: {e}")
        return None

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
        '--netbox-url',
        default='http://localhost:8011',
        help='NetBox URL (default: http://localhost:8011)'
    )
    parser.add_argument(
        '--username',
        default='admin',
        help='NetBox username (default: admin)'
    )
    parser.add_argument(
        '--password',
        default='admin',
        help='NetBox password (default: admin)'
    )
    parser.add_argument(
        '--token',
        help='Token value (if not provided, will create or retrieve from database)'
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
    parser.add_argument(
        '--create-token',
        action='store_true',
        help='Create a new token if one does not exist (default: only retrieve existing)'
    )
    
    args = parser.parse_args()
    
    # Get token
    if args.token:
        log_info(f"Using provided token for tenant {args.tenant_name}")
        token = args.token
    else:
        description = args.token_description or f"DCops Controller API token for {args.tenant_name}"
        
        # Get postgres pod (needed for database queries)
        postgres_pod = get_postgres_pod(args.netbox_namespace)
        if not postgres_pod:
            log_error(f"Failed to find PostgreSQL pod in namespace {args.netbox_namespace}")
            sys.exit(1)
        
        log_info(f"Using PostgreSQL pod: {postgres_pod}")
        
        # If --create-token is set, always create a fresh token (delete old one first)
        if args.create_token:
            log_info(f"Creating fresh token (--create-token flag set)...")
            token_id = create_token_via_api(
                args.netbox_url,
                args.username,
                args.password,
                description
            )
            
            if token_id:
                # Wait a moment for database to be updated
                log_info("Waiting for database to sync...")
                time.sleep(2)
                
                # Query database for the newly created token by description
                log_info(f"Retrieving token key from database for description '{description}'...")
                token = get_token_from_db(
                    postgres_pod,
                    args.netbox_namespace,
                    description,
                    args.username,
                    args.postgres_db,
                    args.postgres_user,
                    args.postgres_password
                )
            else:
                log_error("Failed to create token via API")
                token = None
        else:
            # Try to get existing token from database first
            log_info(f"Checking for existing token in NetBox database (description: '{description}')")
            token = get_token_from_db(
                postgres_pod,
                args.netbox_namespace,
                description,
                args.username,
                args.postgres_db,
                args.postgres_user,
                args.postgres_password
            )
        
        # If still not found and we didn't try creating, try getting most recent token
        if not token and not args.create_token:
            log_info(f"Token not found, creating new token via API...")
            token_id = create_token_via_api(
                args.netbox_url,
                args.username,
                args.password,
                description
            )
            
            if token_id:
                # Wait a moment for database to be updated
                time.sleep(1)
                
                # Query database for the newly created token
                # If token_id is actually a string (token key), use it directly
                if isinstance(token_id, str) and len(token_id) > 40:
                    token = token_id
                else:
                    # Query by ID or description
                    log_info(f"Retrieving token key from database for token ID {token_id}...")
                    token = get_token_from_db(
                        postgres_pod,
                        args.netbox_namespace,
                        description,
                        args.username,
                        args.postgres_db,
                        args.postgres_user,
                        args.postgres_password
                    )
        
            # If still not found, try getting most recent token
            if not token:
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
            if not args.create_token:
                log_error("Options:")
                log_error(f"  1. Create token manually in NetBox UI and run with --token flag")
                log_error(f"  2. Run with --create-token flag to create token automatically")
                log_error(f"  3. Run: python3 scripts/create_tenant_secret.py --tenant-name {args.tenant_name} --secret-name {args.secret_name} --token <your-token>")
            else:
                log_error("Token creation failed. Please check NetBox connectivity and credentials.")
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

