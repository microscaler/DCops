#!/usr/bin/env python3
"""
Setup NetBox tenant, user, and API token for DCops.

This script:
1. Waits for NetBox to be ready
2. Creates a tenant in NetBox (or uses existing)
3. Creates an API token for the admin user (or uses existing)
4. Creates a Kubernetes Secret with the token

Usage:
    python3 scripts/setup_netbox_tenant.py --tenant-name TENANT_NAME --secret-name SECRET_NAME [options]
"""

import argparse
import subprocess
import sys
import time
import requests
import json
import secrets

def log_info(message):
    print(f"ℹ️  {message}")

def log_error(message):
    print(f"❌ {message}", file=sys.stderr)

def log_success(message):
    print(f"✅ {message}")

def wait_for_netbox(netbox_url, max_wait=300):
    """Wait for NetBox to be ready."""
    log_info(f"Waiting for NetBox to be ready at {netbox_url}...")
    start_time = time.time()
    
    while time.time() - start_time < max_wait:
        try:
            response = requests.get(f"{netbox_url}/api/", timeout=5)
            if response.status_code == 200:
                log_success("NetBox is ready")
                return True
        except requests.exceptions.RequestException:
            pass
        
        time.sleep(2)
    
    log_error(f"NetBox not ready after {max_wait} seconds")
    return False

def get_or_create_tenant(netbox_url, token, tenant_name, tenant_slug=None, description=None):
    """Get or create a tenant in NetBox."""
    if not tenant_slug:
        tenant_slug = tenant_name.lower().replace(' ', '-')
    
    # Check if tenant exists
    headers = {
        "Authorization": f"Token {token}",
        "Accept": "application/json",
        "Content-Type": "application/json",
    }
    
    # Query for existing tenant
    response = requests.get(
        f"{netbox_url}/api/tenancy/tenants/?name={tenant_name}",
        headers=headers,
        timeout=10
    )
    
    if response.status_code == 200:
        data = response.json()
        if data.get('results'):
            tenant = data['results'][0]
            log_info(f"Tenant '{tenant_name}' already exists (ID: {tenant['id']})")
            return tenant
    
    # Create tenant
    log_info(f"Creating tenant '{tenant_name}' in NetBox...")
    payload = {
        "name": tenant_name,
        "slug": tenant_slug,
    }
    if description:
        payload["description"] = description
    
    response = requests.post(
        f"{netbox_url}/api/tenancy/tenants/",
        headers=headers,
        json=payload,
        timeout=10
    )
    
    if response.status_code in (200, 201):
        tenant = response.json()
        log_success(f"Created tenant '{tenant_name}' (ID: {tenant['id']})")
        return tenant
    else:
        error_msg = response.text
        log_error(f"Failed to create tenant: {response.status_code} - {error_msg}")
        return None

def get_user_id(netbox_url, token, username):
    """Get user ID by username."""
    headers = {
        "Authorization": f"Token {token}",
        "Accept": "application/json",
    }
    
    response = requests.get(
        f"{netbox_url}/api/users/users/?username={username}",
        headers=headers,
        timeout=10
    )
    
    if response.status_code == 200:
        data = response.json()
        if data.get('results'):
            user = data['results'][0]
            return user['id']
    
    return None

def get_user_id_from_session(netbox_url, session, username):
    """Get user ID by username using session."""
    response = session.get(
        f"{netbox_url}/api/users/users/?username={username}",
        timeout=10
    )
    
    if response.status_code == 200:
        data = response.json()
        if data.get('results'):
            user = data['results'][0]
            return user['id']
    
    return None

def generate_token_key(prefix="dcops", length=40):
    """Generate a 40-character token key."""
    # NetBox requires exactly 40 characters
    # Format: prefix (up to 10 chars) + random hex to make 40 total
    prefix_clean = prefix.replace('-', '').replace('_', '')[:10]  # Max 10 chars for prefix
    remaining_length = length - len(prefix_clean)
    # Generate enough hex bytes to fill remaining length (round up to ensure we have enough)
    hex_bytes_needed = (remaining_length + 1) // 2  # Round up
    random_part = secrets.token_hex(hex_bytes_needed)
    # Combine and truncate to exactly 40 chars
    token_key = f"{prefix_clean}{random_part}"[:length]
    # Ensure exactly 40 chars (pad if needed, though shouldn't be necessary)
    if len(token_key) < length:
        token_key = token_key + secrets.token_hex(1)[:length - len(token_key)]
    return token_key[:length]

def get_or_create_token(netbox_url, session, user_id, token_key, description):
    """Get or create an API token."""
    # If token_key is provided but less than 40 chars, generate a proper one
    if token_key and len(token_key) < 40:
        # Generate a 40-char key based on the prefix
        token_key = generate_token_key(token_key.replace('-', '').replace('_', ''), 40)
        log_info(f"Generated 40-character token key: {token_key[:20]}...")
    
    # Check if token already exists by description (more reliable than key)
    response = session.get(
        f"{netbox_url}/api/users/tokens/?user_id={user_id}&description={description}",
        timeout=10
    )
    
    if response.status_code == 200:
        data = response.json()
        if data.get('results'):
            token = data['results'][0]
            log_info(f"Token with description '{description}' already exists")
            return token['key']
    
    # Create new token
    # If no key provided or key is None, let NetBox auto-generate it
    log_info(f"Creating API token with description '{description}'...")
    
    # Get CSRF token from cookies
    csrf_token = session.cookies.get('csrftoken')
    if not csrf_token:
        log_error("Could not get CSRF token from session")
        return None
    
    # Build payload - only include key if we have a valid 40-char one
    payload = {
        "user": user_id,
        "write_enabled": True,
        "description": description,
    }
    
    # Only include key if it's 40+ characters (let NetBox auto-generate otherwise)
    if token_key and len(token_key) >= 40:
        payload["key"] = token_key
        log_info(f"Using provided token key: {token_key[:20]}...")
    else:
        log_info("Letting NetBox auto-generate token key")
    
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
    
    if response.status_code in (200, 201):
        token_data = response.json()
        token_key = token_data.get('key')
        if token_key:
            log_success(f"Created API token with key '{token_key}'")
            return token_key
        else:
            log_error("Token created but key not returned")
            return None
    else:
        error_msg = response.text
        log_error(f"Failed to create token: {response.status_code} - {error_msg}")
        return None

def login_to_netbox(netbox_url, username, password):
    """Login to NetBox and return a session with cookies."""
    session = requests.Session()
    
    # Get CSRF token from login page
    response = session.get(f"{netbox_url}/login/", timeout=10)
    if response.status_code != 200:
        log_error(f"Failed to load login page: {response.status_code}")
        return None
    
    # Extract CSRF token
    import re
    csrf_match = re.search(r'name="csrfmiddlewaretoken" value="([^"]+)"', response.text)
    if not csrf_match:
        log_error("Could not find CSRF token on login page")
        return None
    
    csrf_token = csrf_match.group(1)
    
    # Login
    login_data = {
        'username': username,
        'password': password,
        'csrfmiddlewaretoken': csrf_token,
    }
    
    response = session.post(f"{netbox_url}/login/", data=login_data, timeout=10)
    if response.status_code != 200:
        log_error(f"Failed to login: {response.status_code}")
        return None
    
    # Check if login was successful (redirect to home page)
    if 'login' in response.url.lower():
        log_error("Login failed - still on login page")
        return None
    
    log_success(f"Logged in to NetBox as {username}")
    return session

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
        description='Setup NetBox tenant, user, and API token'
    )
    parser.add_argument(
        '--tenant-name',
        default='datacenter-tenant',
        help='Name of the tenant to create (default: datacenter-tenant)'
    )
    parser.add_argument(
        '--tenant-slug',
        help='Slug for the tenant (default: auto-generated from name)'
    )
    parser.add_argument(
        '--tenant-description',
        help='Description for the tenant'
    )
    parser.add_argument(
        '--secret-name',
        default='netbox-token-datacenter-tenant',
        help='Name of the Kubernetes Secret to create (default: netbox-token-datacenter-tenant)'
    )
    parser.add_argument(
        '--namespace',
        default='default',
        help='Kubernetes namespace for secret (default: default)'
    )
    parser.add_argument(
        '--netbox-url',
        default='http://localhost:8001',
        help='NetBox URL (default: http://localhost:8001)'
    )
    parser.add_argument(
        '--netbox-user',
        default='admin',
        help='NetBox username (default: admin)'
    )
    parser.add_argument(
        '--netbox-password',
        default='admin',
        help='NetBox password (default: admin)'
    )
    parser.add_argument(
        '--token-key',
        help='API token key (default: auto-generated from tenant name)'
    )
    parser.add_argument(
        '--token-description',
        help='API token description (default: auto-generated from tenant name)'
    )
    parser.add_argument(
        '--admin-token',
        help='Existing admin API token (if not provided, will login via UI)'
    )
    
    args = parser.parse_args()
    
    # Wait for NetBox to be ready
    if not wait_for_netbox(args.netbox_url):
        sys.exit(1)
    
    # Login to get session (needed for token creation)
    log_info(f"Logging in to NetBox as {args.netbox_user}...")
    session = login_to_netbox(args.netbox_url, args.netbox_user, args.netbox_password)
    if not session:
        log_error("Failed to login to NetBox")
        sys.exit(1)
    
    # Get user ID
    user_id = get_user_id_from_session(args.netbox_url, session, args.netbox_user)
    if not user_id:
        log_error(f"User '{args.netbox_user}' not found")
        sys.exit(1)
    
    # Get or create admin token for API calls
    if args.admin_token:
        admin_token = args.admin_token
        log_info("Using provided admin token")
    else:
        # Try to get existing admin token by description, or create one
        # Don't specify a key - let NetBox auto-generate it
        admin_token = get_or_create_token(
            args.netbox_url,
            session,
            user_id,
            None,  # Let NetBox auto-generate the key
            "DCops admin token for setup"
        )
        
        if not admin_token:
            log_error("Failed to get or create admin token")
            sys.exit(1)
    
    # Create tenant
    tenant = get_or_create_tenant(
        args.netbox_url,
        admin_token,
        args.tenant_name,
        args.tenant_slug,
        args.tenant_description or f"Tenant for {args.tenant_name}"
    )
    
    if not tenant:
        log_error("Failed to create or get tenant")
        sys.exit(1)
    
    # Get or create token for the tenant (using existing session)
    # Generate a proper token key if provided, otherwise let NetBox auto-generate
    token_key = args.token_key
    if token_key and len(token_key) < 40:
        # Generate a 40-char key based on the prefix
        prefix = token_key.replace('-', '').replace('_', '')
        token_key = generate_token_key(prefix, 40)
    
    token_description = args.token_description or f"DCops Controller API token for {args.tenant_name}"
    
    tenant_token = get_or_create_token(
        args.netbox_url,
        session,
        user_id,
        token_key,  # Will be None or a valid 40-char key
        token_description
    )
    
    if not tenant_token:
        log_error("Failed to create tenant token")
        sys.exit(1)
    
    # Create Kubernetes secret
    log_info(f"Creating Kubernetes secret {args.secret_name} in namespace {args.namespace}")
    if create_secret(tenant_token, args.namespace, args.secret_name):
        log_success(f"NetBox tenant setup complete for {args.tenant_name}!")
        log_info(f"   Tenant: {args.tenant_name} (ID: {tenant['id']})")
        log_info(f"   Secret: {args.secret_name} in namespace {args.namespace}")
        sys.exit(0)
    else:
        sys.exit(1)

if __name__ == '__main__':
    main()

