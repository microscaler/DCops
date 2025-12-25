#!/usr/bin/env python3
"""
Simple script to create a NetBox token and update Kubernetes secret.

This script uses port-forwarding to access NetBox from localhost.
It requires NetBox to be accessible on localhost:8001 (via Tilt port-forward).

Usage:
    python3 scripts/create_netbox_token_simple.py [--token TOKEN]
    
If --token is not provided, the script will attempt to create one via the NetBox UI.
"""

import argparse
import json
import subprocess
import sys
import requests
from base64 import b64encode

def log_info(message):
    print(f"ℹ️  {message}")

def log_error(message):
    print(f"❌ {message}", file=sys.stderr)

def log_success(message):
    print(f"✅ {message}")

def create_token_via_ui(netbox_url, username, password):
    """Create a token by logging into NetBox UI and using the API."""
    session = requests.Session()
    
    # Step 1: Get CSRF token
    try:
        response = session.get(f"{netbox_url}/login/")
        response.raise_for_status()
        
        # Extract CSRF token from the page
        import re
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
        response = session.post(f"{netbox_url}/login/", data=login_data)
        response.raise_for_status()
        
        # Step 3: Get user ID
        response = session.get(f"{netbox_url}/api/users/users/?username={username}")
        response.raise_for_status()
        user_data = response.json()
        if not user_data.get('results'):
            log_error(f"User {username} not found")
            return None
        user_id = user_data['results'][0]['id']
        
        # Step 4: Check for existing token
        response = session.get(f"{netbox_url}/api/users/tokens/?user_id={user_id}&key=dcops-controller")
        response.raise_for_status()
        token_data = response.json()
        if token_data.get('results'):
            token = token_data['results'][0]['key']
            log_info(f"Found existing token with key 'dcops-controller'")
            return token
        
        # Step 5: Get CSRF token for API
        response = session.get(f"{netbox_url}/api/users/tokens/")
        csrf_token = session.cookies.get('csrftoken')
        
        # Step 6: Create new token
        token_payload = {
            'user': user_id,
            'key': 'dcops-controller',
            'write_enabled': True,
            'description': 'DCops Controller API token'
        }
        headers = {
            'X-CSRFToken': csrf_token,
            'Referer': f"{netbox_url}/",
        }
        response = session.post(f"{netbox_url}/api/users/tokens/", json=token_payload, headers=headers)
        response.raise_for_status()
        token_response = response.json()
        token = token_response.get('key')
        if token:
            log_success(f"Created new token with key 'dcops-controller'")
            return token
        else:
            log_error("Token created but key not returned")
            return None
            
    except Exception as e:
        log_error(f"Failed to create token: {e}")
        return None

def update_secret(token, namespace='dcops-system', secret_name='netbox-token'):
    """Update Kubernetes secret with the token."""
    # Use kubectl create secret with --dry-run=client -o yaml | kubectl apply
    # This is the most reliable way to create or update a secret
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
        log_info("The controller will pick up the new token on the next reconciliation")
        return True
    except subprocess.CalledProcessError as e:
        error_msg = e.stderr.decode() if e.stderr else 'Unknown error'
        log_error(f"Failed to update secret: {error_msg}")
        return False

def main():
    parser = argparse.ArgumentParser(description='Create NetBox token and update Kubernetes secret')
    parser.add_argument('--netbox-url', default='http://localhost:8001', help='NetBox URL (default: http://localhost:8001)')
    parser.add_argument('--netbox-user', default='admin', help='NetBox username (default: admin)')
    parser.add_argument('--netbox-password', default='admin', help='NetBox password (default: admin)')
    parser.add_argument('--token', help='Existing token to use (skips token creation)')
    parser.add_argument('--namespace', default='dcops-system', help='Kubernetes namespace (default: dcops-system)')
    parser.add_argument('--secret-name', default='netbox-token', help='Secret name (default: netbox-token)')
    
    args = parser.parse_args()
    
    # Get or create token
    if args.token:
        token = args.token
        log_info("Using provided token")
    else:
        log_info(f"Creating token via NetBox API at {args.netbox_url}")
        token = create_token_via_ui(args.netbox_url, args.netbox_user, args.netbox_password)
        if not token:
            log_error("Failed to create token. Please create one manually:")
            log_error(f"  1. Go to {args.netbox_url}/user/api-tokens/")
            log_error("  2. Create a new token with key 'dcops-controller'")
            log_error(f"  3. Run: python3 scripts/create_netbox_token_simple.py --token <your-token>")
            sys.exit(1)
    
    # Update secret
    log_info(f"Updating Kubernetes secret {args.secret_name} in namespace {args.namespace}")
    if update_secret(token, args.namespace, args.secret_name):
        log_success("Token management complete!")
        sys.exit(0)
    else:
        sys.exit(1)

if __name__ == '__main__':
    main()

