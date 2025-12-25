#!/usr/bin/env python3
"""
Manages NetBox API token for the IP Claim Controller.

This script:
1. Gets or creates a NetBox API token for the admin user
2. Stores it in a Kubernetes secret that the controller reads

Usage:
    python3 scripts/manage_netbox_token.py [--netbox-url URL] [--netbox-user USER] [--netbox-password PASSWORD] [--namespace NAMESPACE]

Environment variables:
    NETBOX_URL: NetBox base URL (default: http://localhost:8001)
    NETBOX_USER: NetBox admin username (default: admin)
    NETBOX_PASSWORD: NetBox admin password (default: admin)
    KUBECTL_NAMESPACE: Kubernetes namespace for secret (default: dcops-system)
"""

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Optional

# --- Utility Functions ---

def log_info(message):
    print(f"ℹ️  {message}")

def log_warn(message):
    print(f"⚠️  {message}", file=sys.stderr)

def log_error(message):
    print(f"❌ {message}", file=sys.stderr)

def run_command(command, check=True, capture_output=False, env=None, input=None):
    """Runs a shell command."""
    log_info(f"Running: {' '.join(command) if isinstance(command, list) else command}")
    try:
        if capture_output:
            result = subprocess.run(
                command,
                check=check,
                capture_output=True,
                text=True,
                env=env,
                input=input
            )
            if result.stdout:
                print(result.stdout)
            if result.stderr and check:
                print(result.stderr, file=sys.stderr)
            return result
        else:
            return subprocess.run(command, check=check, env=env, input=input)
    except subprocess.CalledProcessError as e:
        log_error(f"Command failed with exit code {e.returncode}: {e.cmd}")
        if e.stdout:
            print(e.stdout)
        if e.stderr:
            print(e.stderr, file=sys.stderr)
        raise
    except FileNotFoundError:
        log_error(f"Command not found: {command[0] if isinstance(command, list) else command.split(' ')[0]}")
        sys.exit(1)

# --- NetBox API Functions ---

def get_or_create_netbox_token_via_api(netbox_url: str, username: str, password: str, token_key: str = "dcops-controller") -> Optional[str]:
    """
    Get or create a NetBox API token using the REST API via curl.
    
    Uses curl to interact with NetBox API because it handles cookies and CSRF tokens better.
    """
    import base64
    
    # Use curl to login and get session cookie
    # NetBox requires CSRF token for API token creation, so we use the web UI flow
    log_info("Attempting to get or create token via NetBox API...")
    
    # First, try to get existing tokens using basic auth (if NetBox supports it)
    # Or use curl with session-based auth
    
    # For simplicity, we'll use curl to:
    # 1. Login and get session cookie + CSRF token
    # 2. Query existing tokens
    # 3. Create new token if needed
    
    try:
        # Step 1: Get CSRF token from login page
        login_page_cmd = [
            "curl", "-s", "-c", "/tmp/netbox_cookies.txt",
            f"{netbox_url}/login/"
        ]
        result = run_command(login_page_cmd, check=False, capture_output=True)
        
        if result.returncode != 0:
            log_warn("Could not access NetBox login page, trying alternative method...")
        
        # Step 2: Login and get session
        login_data = f"username={username}&password={password}"
        login_cmd = [
            "curl", "-s", "-b", "/tmp/netbox_cookies.txt", "-c", "/tmp/netbox_cookies.txt",
            "-X", "POST",
            "-H", "Content-Type: application/x-www-form-urlencoded",
            "-H", "Referer: {}/login/".format(netbox_url),
            "-d", login_data,
            f"{netbox_url}/login/"
        ]
        result = run_command(login_cmd, check=False, capture_output=True)
        
        # Step 3: Get user ID
        user_cmd = [
            "curl", "-s", "-b", "/tmp/netbox_cookies.txt",
            "-H", "Accept: application/json",
            f"{netbox_url}/api/users/users/?username={username}"
        ]
        result = run_command(user_cmd, check=True, capture_output=True)
        import json
        user_data = json.loads(result.stdout)
        if not user_data.get('results'):
            log_error(f"User {username} not found")
            return None
        user_id = user_data['results'][0]['id']
        
        # Step 4: Check for existing token
        token_cmd = [
            "curl", "-s", "-b", "/tmp/netbox_cookies.txt",
            "-H", "Accept: application/json",
            f"{netbox_url}/api/users/tokens/?user_id={user_id}&key={token_key}"
        ]
        result = run_command(token_cmd, check=False, capture_output=True)
        if result.returncode == 0:
            token_data = json.loads(result.stdout)
            if token_data.get('results'):
                token = token_data['results'][0]['key']
                log_info(f"Found existing token with key '{token_key}'")
                return token
        
        # Step 5: Create new token (requires CSRF token)
        # Get CSRF token from cookies or headers
        csrf_token = None
        try:
            with open("/tmp/netbox_cookies.txt", "r") as f:
                for line in f:
                    if "csrftoken" in line:
                        csrf_token = line.split()[6]  # Cookie value is in column 6
                        break
        except:
            pass
        
        if not csrf_token:
            log_warn("Could not extract CSRF token, token creation may fail")
            log_warn("Please create token manually in NetBox UI and use --token flag")
            return None
        
        # Create token
        token_payload = {
            "user": user_id,
            "key": token_key,
            "write_enabled": True,
            "description": "DCops IP Claim Controller API token"
        }
        import tempfile
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            json.dump(token_payload, f)
            payload_file = f.name
        
        create_cmd = [
            "curl", "-s", "-b", "/tmp/netbox_cookies.txt", "-c", "/tmp/netbox_cookies.txt",
            "-X", "POST",
            "-H", "Content-Type: application/json",
            "-H", "Accept: application/json",
            "-H", f"X-CSRFToken: {csrf_token}",
            "-H", f"Referer: {netbox_url}",
            "-d", f"@{payload_file}",
            f"{netbox_url}/api/users/tokens/"
        ]
        result = run_command(create_cmd, check=False, capture_output=True)
        
        import os
        os.unlink(payload_file)
        
        if result.returncode == 0:
            try:
                token_data = json.loads(result.stdout)
                token = token_data.get('key') or token_data.get('id')
                if token:
                    log_info(f"Created new token with key '{token_key}'")
                    return token
            except:
                pass
        
        log_error("Failed to create token via API")
        log_warn("Please create token manually in NetBox UI:")
        log_warn(f"  1. Go to {netbox_url}/user/api-tokens/")
        log_warn(f"  2. Create a new token")
        log_warn(f"  3. Copy the token and use --token flag")
        return None
        
    except Exception as e:
        log_error(f"Failed to get/create token: {e}")
        log_warn("Please create token manually in NetBox UI and use --token flag")
        return None

# --- Kubernetes Secret Functions ---

def get_current_token(namespace: str, secret_name: str) -> Optional[str]:
    """Get current token from Kubernetes secret."""
    result = run_command(
        ["kubectl", "get", "secret", secret_name, "-n", namespace, "-o", "jsonpath='{.data.token}'"],
        check=False,
        capture_output=True
    )
    
    if result.returncode == 0 and result.stdout.strip():
        import base64
        try:
            # Remove quotes from jsonpath output
            token_b64 = result.stdout.strip().strip("'\"")
            token = base64.b64decode(token_b64).decode('utf-8')
            return token
        except Exception:
            return None
    return None

def create_or_update_secret(namespace: str, secret_name: str, token: str):
    """Create or update Kubernetes secret with NetBox token."""
    # Check if token has changed
    current_token = get_current_token(namespace, secret_name)
    if current_token == token:
        log_info(f"Token in secret {secret_name} is already up-to-date, skipping update")
        return
    
    log_info(f"Creating/updating secret {secret_name} in namespace {namespace}")
    
    # Check if secret exists
    result = run_command(
        ["kubectl", "get", "secret", secret_name, "-n", namespace],
        check=False,
        capture_output=True
    )
    
    if result.returncode == 0:
        log_info(f"Secret {secret_name} already exists, updating with new token...")
        # Update existing secret
        run_command(
            ["kubectl", "create", "secret", "generic", secret_name,
             f"--from-literal=token={token}",
             "-n", namespace,
             "--dry-run=client", "-o", "yaml"],
            check=True,
            capture_output=True
        )
        # Apply the update
        run_command(
            ["kubectl", "apply", "-f", "-"],
            check=True,
            input=f"apiVersion: v1\nkind: Secret\nmetadata:\n  name: {secret_name}\n  namespace: {namespace}\ntype: Opaque\nstringData:\n  token: {token}\n"
        )
    else:
        log_info(f"Creating new secret {secret_name}...")
        # Create new secret
        run_command(
            ["kubectl", "create", "secret", "generic", secret_name,
             f"--from-literal=token={token}",
             "-n", namespace],
            check=True
        )
    
    log_info(f"✅ Secret {secret_name} created/updated successfully")

# --- Main Function ---

def main():
    parser = argparse.ArgumentParser(
        description="Manage NetBox API token for IP Claim Controller",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Get token from NetBox and store in secret
  python3 scripts/manage_netbox_token.py --netbox-url http://localhost:8001

  # Use existing token
  python3 scripts/manage_netbox_token.py --token abc123def456

  # Specify custom namespace
  python3 scripts/manage_netbox_token.py --namespace my-namespace
        """
    )
    
    parser.add_argument(
        "--netbox-url",
        default=os.getenv("NETBOX_URL", "http://localhost:8001"),
        help="NetBox base URL (default: http://localhost:8001 or NETBOX_URL env var)"
    )
    parser.add_argument(
        "--netbox-user",
        default=os.getenv("NETBOX_USER", "admin"),
        help="NetBox admin username (default: admin or NETBOX_USER env var)"
    )
    parser.add_argument(
        "--netbox-password",
        default=os.getenv("NETBOX_PASSWORD", "admin"),
        help="NetBox admin password (default: admin or NETBOX_PASSWORD env var)"
    )
    parser.add_argument(
        "--token",
        help="NetBox API token (if provided, skips token creation)"
    )
    parser.add_argument(
        "--namespace",
        default=os.getenv("KUBECTL_NAMESPACE", "dcops-system"),
        help="Kubernetes namespace for secret (default: dcops-system or KUBECTL_NAMESPACE env var)"
    )
    parser.add_argument(
        "--secret-name",
        default="netbox-token",
        help="Kubernetes secret name (default: netbox-token)"
    )
    
    args = parser.parse_args()
    
    # Get or create token
    if args.token:
        log_info("Using provided token")
        token = args.token
    else:
        log_info(f"Getting or creating NetBox API token from {args.netbox_url}")
        log_info(f"Using credentials: {args.netbox_user} / {'*' * len(args.netbox_password)}")
        
        # Try to get/create token via API
        token = get_or_create_netbox_token_via_api(
            args.netbox_url,
            args.netbox_user,
            args.netbox_password
        )
        
        if not token:
            log_error("Failed to get or create token automatically.")
            log_error("Please create a token manually in NetBox UI and use --token flag:")
            log_error(f"  1. Go to {args.netbox_url}/user/api-tokens/")
            log_error(f"  2. Create a new token")
            log_error(f"  3. Run: python3 scripts/manage_netbox_token.py --token <your-token>")
            sys.exit(1)
    
    # Store token in Kubernetes secret
    create_or_update_secret(args.namespace, args.secret_name, token)
    
    log_info("✅ NetBox token management complete!")
    log_info(f"   Secret: {args.secret_name} in namespace {args.namespace}")
    log_info(f"   The IP Claim Controller will automatically use this token")

if __name__ == "__main__":
    main()

