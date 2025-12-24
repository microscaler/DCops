#!/usr/bin/env python3
"""
Show cluster and controller status.

Displays the current state of the Kind cluster and DCops controllers.
"""

import subprocess
import sys


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
    """Show status."""
    print("📊 DCops Cluster and Controller Status")
    print("=" * 50)
    print()
    
    # Check if cluster exists
    result = run_command("kind get clusters", check=False)
    if "dcops" not in result.stdout:
        print("❌ Kind cluster 'dcops' not found")
        print("   Run 'just dev-up' to create the cluster")
        sys.exit(1)
    
    print("✅ Kind cluster 'dcops' exists")
    print()
    
    # Check cluster nodes
    print("📦 Cluster Nodes:")
    result = run_command("kubectl get nodes", check=False)
    if result.returncode == 0:
        print(result.stdout)
    else:
        print("   ⚠️  Could not get node status")
    print()
    
    # Check namespace
    print("📁 Namespace:")
    result = run_command("kubectl get namespace microscaler-system", check=False)
    if result.returncode == 0:
        print("✅ microscaler-system namespace exists")
    else:
        print("   ⚠️  microscaler-system namespace not found")
    print()
    
    # Check CRDs
    print("📝 CRDs:")
    crds = [
        "bootprofiles.dcops.microscaler.io",
        "bootintents.dcops.microscaler.io",
        "ippools.dcops.microscaler.io",
        "ipclaims.dcops.microscaler.io",
    ]
    for crd in crds:
        result = run_command(f"kubectl get crd {crd}", check=False)
        if result.returncode == 0:
            print(f"   ✅ {crd}")
        else:
            print(f"   ❌ {crd} (not installed)")
    print()
    
    # Check controllers
    print("🎮 Controllers:")
    controllers = [
        "pxe-intent-controller",
        "ip-claim-controller",
        "routeros-controller",
    ]
    for controller in controllers:
        result = run_command(
            f"kubectl get pods -n microscaler-system -l app={controller}",
            check=False
        )
        if result.returncode == 0 and controller in result.stdout:
            # Extract pod status
            lines = result.stdout.strip().split('\n')
            if len(lines) > 1:
                # Skip header line
                for line in lines[1:]:
                    parts = line.split()
                    if len(parts) >= 3:
                        pod_name = parts[0]
                        status = parts[2]
                        print(f"   {controller}: {status} ({pod_name})")
            else:
                print(f"   {controller}: No pods found")
        else:
            print(f"   {controller}: Not deployed")
    print()
    
    # Check registry
    print("📦 Local Registry:")
    result = run_command("docker ps --format '{{.Names}}'", check=False)
    if "dcops-registry" in result.stdout:
        print("   ✅ dcops-registry is running")
    else:
        print("   ⚠️  dcops-registry is not running")
    print()


if __name__ == "__main__":
    main()

