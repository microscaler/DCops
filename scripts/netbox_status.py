#!/usr/bin/env python3
"""
Show NetBox deployment status.

Displays the current state of NetBox, PostgreSQL, and Redis.
"""

import subprocess
import sys


def log_info(msg):
    """Print info message."""
    print(f"[INFO] {msg}")


def run_command(cmd, check=False):
    """Run a command and return the result."""
    result = subprocess.run(
        cmd,
        shell=isinstance(cmd, str),
        capture_output=True,
        text=True,
        check=check
    )
    return result


def main():
    """Show NetBox status."""
    print("📊 NetBox Deployment Status")
    print("=" * 50)
    print()
    
    # Check namespace
    result = run_command("kubectl get namespace netbox", check=False)
    if result.returncode != 0:
        print("❌ NetBox namespace not found")
        print("   Run 'just deploy-netbox' to deploy NetBox")
        sys.exit(1)
    
    print("✅ NetBox namespace exists")
    print()
    
    # Check deployments
    print("📦 Deployments:")
    deployments = ["postgres", "redis", "netbox"]
    for deployment in deployments:
        result = run_command(
            f"kubectl get deployment {deployment} -n netbox",
            check=False
        )
        if result.returncode == 0:
            # Extract status
            lines = result.stdout.strip().split('\n')
            if len(lines) > 1:
                # Skip header line
                parts = lines[1].split()
                if len(parts) >= 3:
                    ready = parts[1]
                    available = parts[3] if len(parts) > 3 else "N/A"
                    print(f"   {deployment}: {ready} ready, {available} available")
            else:
                print(f"   {deployment}: No status")
        else:
            print(f"   {deployment}: Not found")
    print()
    
    # Check pods
    print("🪟 Pods:")
    result = run_command("kubectl get pods -n netbox", check=False)
    if result.returncode == 0:
        print(result.stdout)
    else:
        print("   ⚠️  Could not get pod status")
    print()
    
    # Check services
    print("🔌 Services:")
    result = run_command("kubectl get svc -n netbox", check=False)
    if result.returncode == 0:
        print(result.stdout)
    else:
        print("   ⚠️  Could not get service status")
    print()
    
    # Check PVCs
    print("💾 Persistent Volumes:")
    result = run_command("kubectl get pvc -n netbox", check=False)
    if result.returncode == 0:
        print(result.stdout)
    else:
        print("   ⚠️  Could not get PVC status")
    print()
    
    # Check NetBox API
    print("🌐 NetBox API:")
    result = run_command(
        "kubectl exec -n netbox deployment/netbox -- curl -s -o /dev/null -w '%{http_code}' http://localhost:8001/api/",
        check=False
    )
    if result.returncode == 0 and result.stdout.strip() == "200":
        print("   ✅ NetBox API is responding")
    else:
        print("   ⚠️  NetBox API may not be ready")
    print()
    
    print("To access NetBox:")
    print("  kubectl port-forward -n netbox svc/netbox 8000:80")
    print("  Then open http://localhost:8000 in your browser")


if __name__ == "__main__":
    main()

