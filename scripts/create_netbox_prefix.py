#!/usr/bin/env python3
"""
Create a NetBox prefix for testing.

Usage:
    python3 scripts/create_netbox_prefix.py --token <token> --prefix 192.168.1.0/24
"""

import argparse
import json
import os
import subprocess
import sys

def log_info(message):
    print(f"ℹ️  {message}")

def log_error(message):
    print(f"❌ {message}", file=sys.stderr)

def create_prefix(netbox_url: str, token: str, prefix: str, description: str = "DCops test prefix"):
    """Create a prefix in NetBox."""
    url = f"{netbox_url}/api/ipam/prefixes/"
    
    payload = {
        "prefix": prefix,
        "description": description,
        "status": "active"
    }
    
    cmd = [
        "curl", "-s", "-X", "POST",
        "-H", f"Authorization: Token {token}",
        "-H", "Content-Type: application/json",
        "-H", "Accept: application/json",
        "-d", json.dumps(payload),
        url
    ]
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    
    if result.returncode != 0:
        log_error(f"Failed to create prefix: {result.stderr}")
        return None
    
    try:
        data = json.loads(result.stdout)
        if "id" in data:
            log_info(f"✅ Created prefix {prefix} with ID: {data['id']}")
            return data['id']
        else:
            log_error(f"Failed to create prefix: {result.stdout}")
            return None
    except json.JSONDecodeError:
        log_error(f"Invalid JSON response: {result.stdout}")
        return None

def main():
    parser = argparse.ArgumentParser(description="Create a NetBox prefix")
    parser.add_argument("--netbox-url", default=os.getenv("NETBOX_URL", "http://localhost:8001"))
    parser.add_argument("--token", required=True, help="NetBox API token")
    parser.add_argument("--prefix", required=True, help="Prefix CIDR (e.g., 192.168.1.0/24)")
    parser.add_argument("--description", default="DCops test prefix")
    
    args = parser.parse_args()
    
    prefix_id = create_prefix(args.netbox_url, args.token, args.prefix, args.description)
    
    if prefix_id:
        print(f"\n✅ Prefix created successfully!")
        print(f"   Prefix ID: {prefix_id}")
        print(f"   Update your IPPool example with: id: \"{prefix_id}\"")
        sys.exit(0)
    else:
        sys.exit(1)

if __name__ == "__main__":
    main()

