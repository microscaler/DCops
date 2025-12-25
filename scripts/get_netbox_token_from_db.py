#!/usr/bin/env python3
"""
Retrieves NetBox API token directly from PostgreSQL database.

This script is designed for CI/CD environments where manual token creation
is not possible. It uses kubectl exec to query the NetBox PostgreSQL database
and retrieves an existing token.

Usage:
    python3 scripts/get_netbox_token_from_db.py [--token-key TOKEN_KEY] [--namespace NAMESPACE]

The script uses kubectl exec to run psql commands, so it works in CI/CD
environments without requiring database client libraries.
"""

import argparse
import json
import os
import subprocess
import sys
from base64 import b64encode

def log_info(message):
    print(f"ℹ️  {message}")

def log_error(message):
    print(f"❌ {message}", file=sys.stderr)

def log_success(message):
    print(f"✅ {message}")

def run_psql_query(postgres_pod, namespace, query, dbname='netbox', user='netbox', password='netbox'):
    """Run a PostgreSQL query using kubectl exec."""
    # Use PGPASSWORD environment variable for password
    env = os.environ.copy()
    env['PGPASSWORD'] = password
    
    # Run psql command via kubectl exec
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

def get_postgres_pod(namespace='netbox'):
    """Get the name of the PostgreSQL pod."""
    try:
        result = subprocess.run(
            ['kubectl', 'get', 'pod', '-n', namespace, '-l', 'app=postgres', '-o', 'jsonpath={.items[0].metadata.name}'],
            check=True,
            capture_output=True,
            text=True
        )
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        log_error(f"Failed to get PostgreSQL pod: {e.stderr}")
        return None

def get_token_from_db(postgres_pod, namespace, description=None, username='admin', dbname='netbox', user='netbox', password='netbox'):
    """Retrieve token from database by description or get most recent token for user.
    
    Note: NetBox token 'key' field is the actual token value (40-char hex), not a label.
    We search by description (if provided) or get the most recent token for the user.
    """
    if description:
        # Search by description
        query = f"""
            SELECT ut.key, ut.user_id, u.username, ut.description
            FROM users_token ut
            JOIN users_user u ON ut.user_id = u.id
            WHERE ut.description = '{description}' AND u.username = '{username}'
            ORDER BY ut.created DESC
            LIMIT 1
        """
    else:
        # Get most recent token for user
        query = f"""
            SELECT ut.key, ut.user_id, u.username, ut.description
            FROM users_token ut
            JOIN users_user u ON ut.user_id = u.id
            WHERE u.username = '{username}'
            ORDER BY ut.created DESC
            LIMIT 1
        """
    
    result = run_psql_query(postgres_pod, namespace, query, dbname, user, password)
    if not result:
        return None
    
    # Parse result (format: key|user_id|username|description)
    if result and '|' in result:
        parts = result.split('|')
        if len(parts) >= 3:
            token_key, user_id, username_found = parts[0], parts[1], parts[2]
            desc = parts[3] if len(parts) > 3 else ''
            if description:
                log_info(f"Found token with description '{description}' for user '{username_found}' (user_id: {user_id})")
            else:
                log_info(f"Found most recent token for user '{username_found}' (user_id: {user_id}, description: '{desc or '(none)'}')")
            return token_key
    
    if description:
        log_info(f"Token with description '{description}' not found for user '{username}'")
    else:
        log_info(f"No tokens found for user '{username}'")
    return None

def update_secret(token, namespace='dcops-system', secret_name='netbox-token'):
    """Update Kubernetes secret with the token."""
    try:
        secret_yaml = f"""apiVersion: v1
kind: Secret
metadata:
  name: {secret_name}
  namespace: {namespace}
type: Opaque
stringData:
  token: {token}
"""
        result = subprocess.run(
            ['kubectl', 'apply', '-f', '-'],
            input=secret_yaml.encode(),
            check=True,
            capture_output=True
        )
        log_success(f"Updated secret {secret_name} in namespace {namespace}")
        return True
    except subprocess.CalledProcessError as e:
        error_msg = e.stderr.decode() if e.stderr else 'Unknown error'
        log_error(f"Failed to update secret: {error_msg}")
        return False

def main():
    parser = argparse.ArgumentParser(description='Get NetBox token from PostgreSQL database')
    parser.add_argument('--description', default='DCops Controller API token', help='Token description to search for (default: DCops Controller API token). If not found, gets most recent token for user.')
    parser.add_argument('--username', default='admin', help='NetBox username (default: admin)')
    parser.add_argument('--namespace', default='netbox', help='NetBox namespace (default: netbox)')
    parser.add_argument('--secret-namespace', default='dcops-system', help='Kubernetes namespace for secret (default: dcops-system)')
    parser.add_argument('--secret-name', default='netbox-token', help='Secret name (default: netbox-token)')
    parser.add_argument('--postgres-db', default='netbox', help='Database name (default: netbox)')
    parser.add_argument('--postgres-user', default='netbox', help='Database user (default: netbox)')
    parser.add_argument('--postgres-password', default='netbox', help='Database password (default: netbox)')
    
    args = parser.parse_args()
    
    if args.description:
        log_info(f"Looking for token with description '{args.description}' for user '{args.username}'")
    else:
        log_info(f"Looking for most recent token for user '{args.username}'")
    
    # Get PostgreSQL pod name
    postgres_pod = get_postgres_pod(args.namespace)
    if not postgres_pod:
        log_error(f"Failed to find PostgreSQL pod in namespace {args.namespace}")
        sys.exit(1)
    
    log_info(f"Using PostgreSQL pod: {postgres_pod}")
    
    # Try to get existing token by description first
    token = get_token_from_db(
        postgres_pod,
        args.namespace,
        args.description,
        args.username,
        args.postgres_db,
        args.postgres_user,
        args.postgres_password
    )
    
    # If not found by description, try getting most recent token
    if not token and args.description:
        log_info(f"Token with description '{args.description}' not found, trying most recent token...")
        token = get_token_from_db(
            postgres_pod,
            args.namespace,
            None,  # No description filter
            args.username,
            args.postgres_db,
            args.postgres_user,
            args.postgres_password
        )
    
    if not token:
        log_error(f"No token found for user '{args.username}'")
        log_error("Please create a token in NetBox UI:")
        log_error(f"  1. Go to NetBox UI and create a token")
        if args.description:
            log_error(f"  2. Set description to '{args.description}' (optional)")
        log_error("  3. Then run this script again to retrieve it from the database")
        sys.exit(1)
    
    # Update Kubernetes secret
    log_info(f"Updating Kubernetes secret {args.secret_name} in namespace {args.secret_namespace}")
    if update_secret(token, args.secret_namespace, args.secret_name):
        log_success("Token management complete!")
        sys.exit(0)
    else:
        sys.exit(1)

if __name__ == '__main__':
    main()

