#!/usr/bin/env python3
"""
Undeploy NetBox from Kind cluster.

Removes NetBox, PostgreSQL, and Redis deployments.
"""

import subprocess
import sys
from pathlib import Path


def log_info(msg):
    """Print info message."""
    print(f"[INFO] {msg}")


def log_warn(msg):
    """Print warning message."""
    print(f"[WARN] {msg}")


def run_command(cmd, check=False, capture_output=True):
    """Run a command and return the result."""
    result = subprocess.run(
        cmd,
        shell=isinstance(cmd, str),
        capture_output=capture_output,
        text=True,
        check=check
    )
    return result


def undeploy_netbox():
    """Undeploy NetBox from the cluster."""
    log_info("🛑 Undeploying NetBox from Kind cluster...")
    
    # Get script directory and project root
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    netbox_config = project_root / "config" / "netbox"
    
    if not netbox_config.exists():
        log_warn("NetBox config directory not found, skipping undeploy")
        return
    
    # Remove using kustomize
    log_info("Removing NetBox manifests...")
    result = run_command(
        ["kubectl", "delete", "-k", str(netbox_config)],
        check=False,
        capture_output=True
    )
    
    if result.returncode == 0:
        log_info("✅ NetBox removed successfully")
    else:
        # Check if resources don't exist (that's okay)
        if "not found" in result.stderr.lower() or "NotFound" in result.stderr:
            log_info("✅ NetBox already removed or not deployed")
        else:
            log_warn(f"Some resources may not have been removed: {result.stderr}")
    
    # Note: PVCs are not deleted by default (to preserve data)
    # To delete PVCs, run: kubectl delete pvc -n netbox --all
    log_info("")
    log_info("Note: PVCs are preserved by default to keep data.")
    log_info("To delete PVCs: kubectl delete pvc -n netbox --all")


def main():
    """Main undeployment function."""
    undeploy_netbox()


if __name__ == "__main__":
    main()

