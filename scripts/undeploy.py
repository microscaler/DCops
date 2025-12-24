#!/usr/bin/env python3
"""
Undeploy DCops controllers from Kubernetes.

Removes all DCops resources from the cluster.
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


def main():
    """Undeploy controllers."""
    log_info("🛑 Undeploying DCops controllers...")
    
    # Check if config directory exists
    config_dir = Path("config")
    if not config_dir.exists():
        log_warn("config/ directory not found, skipping undeploy")
        return
    
    # Remove using kustomize (idempotent)
    log_info("Removing controllers using kustomize...")
    result = run_command(
        ["kubectl", "delete", "-k", str(config_dir)],
        check=False,
        capture_output=True
    )
    
    if result.returncode == 0:
        log_info("✅ Controllers removed successfully")
    else:
        # Check if resources don't exist (that's okay)
        if "not found" in result.stderr.lower() or "NotFound" in result.stderr:
            log_info("✅ Controllers already removed or not deployed")
        else:
            log_warn(f"Some resources may not have been removed: {result.stderr}")
    
    # Remove CRDs (optional - comment out if you want to keep CRDs)
    log_info("Removing CRDs...")
    crd_dir = config_dir / "crd"
    if crd_dir.exists():
        result = run_command(
            ["kubectl", "delete", "-f", str(crd_dir)],
            check=False,
            capture_output=True
        )
        if result.returncode == 0:
            log_info("✅ CRDs removed")
        else:
            if "not found" in result.stderr.lower():
                log_info("✅ CRDs already removed")
            else:
                log_warn(f"CRD removal had issues: {result.stderr}")
    
    log_info("✅ Undeploy complete")


if __name__ == "__main__":
    main()

