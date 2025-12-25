#!/usr/bin/env python3
"""
Generate and apply CRDs from Rust code.

This script:
1. Builds the crdgen binary
2. Runs crdgen to generate CRD YAML
3. Applies the CRDs to the Kubernetes cluster

Usage:
    python3 scripts/generate_crds.py
"""

import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path


def log_info(msg):
    """Print info message."""
    print(f"ℹ️  {msg}")


def log_error(msg):
    """Print error message."""
    print(f"❌ {msg}", file=sys.stderr)


def log_success(msg):
    """Print success message."""
    print(f"✅ {msg}")


def run_command(cmd, check=True, capture_output=False, env=None):
    """Run a shell command."""
    if isinstance(cmd, str):
        cmd = cmd.split()
    
    result = subprocess.run(
        cmd,
        check=check,
        capture_output=capture_output,
        text=True,
        env=env
    )
    
    if capture_output:
        if result.stdout:
            print(result.stdout, end="")
        if result.stderr:
            print(result.stderr, end="", file=sys.stderr)
    
    return result


def main():
    """Main function."""
    # Get project root
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    
    # Change to project root
    os.chdir(project_root)
    
    log_info("Generating CRDs from Rust code...")
    
    # Determine which crdgen binary to use
    os_name = platform.system()
    arch = platform.machine()
    
    # Try native build first (faster)
    native_crdgen = project_root / "target" / "debug" / "crdgen"
    target_crdgen = project_root / "target" / "x86_64-unknown-linux-musl" / "release" / "crdgen"
    
    crdgen_path = None
    
    if native_crdgen.exists():
        crdgen_path = native_crdgen
        log_info(f"Using native crdgen: {crdgen_path}")
    elif target_crdgen.exists():
        crdgen_path = target_crdgen
        log_info(f"Using cross-compiled crdgen: {target_crdgen}")
    else:
        # Build native crdgen
        log_info("crdgen not found, building native version...")
        try:
            run_command(
                ["cargo", "build", "-p", "crds", "--bin", "crdgen"],
                check=True,
                capture_output=True
            )
            if native_crdgen.exists():
                crdgen_path = native_crdgen
                log_info(f"Built native crdgen: {crdgen_path}")
            else:
                log_error(f"crdgen binary not found after build at {native_crdgen}")
                sys.exit(1)
        except subprocess.CalledProcessError:
            log_error("Failed to build native crdgen")
            sys.exit(1)
    
    if not crdgen_path or not crdgen_path.exists():
        log_error(f"crdgen binary not found at {crdgen_path}")
        sys.exit(1)
    
    # Generate CRD YAML
    log_info(f"Running crdgen: {crdgen_path}")
    crd_output_path = project_root / "config" / "crd" / "all-crds.yaml"
    
    try:
        with open(crd_output_path, "w") as f:
            result = run_command(
                [str(crdgen_path)],
                check=True,
                capture_output=True,
                env=os.environ.copy()
            )
            # Write output to file
            if result.stdout:
                f.write(result.stdout)
            if result.stderr:
                print(result.stderr, end="", file=sys.stderr)
        
        log_success(f"CRD generated: {crd_output_path}")
    except subprocess.CalledProcessError:
        log_error("Failed to generate CRD")
        sys.exit(1)
    
    # Apply CRD to cluster
    log_info("Applying CRD to cluster...")
    
    # Check if kubectl is available
    if not shutil.which("kubectl"):
        log_error("kubectl not found in PATH")
        log_info("CRD generated but not applied. Apply manually with:")
        log_info(f"   kubectl apply -f {crd_output_path}")
        sys.exit(0)
    
    # Check if cluster is accessible
    try:
        run_command(
            ["kubectl", "cluster-info"],
            check=False,
            capture_output=True
        )
    except FileNotFoundError:
        log_error("kubectl not found")
        log_info("CRD generated but not applied. Apply manually with:")
        log_info(f"   kubectl apply -f {crd_output_path}")
        sys.exit(0)
    
    # Apply CRD (idempotent - updates if changed, no-op if same)
    try:
        run_command(
            ["kubectl", "apply", "-f", str(crd_output_path)],
            check=True,
            capture_output=True
        )
        log_success("CRD applied to cluster")
    except subprocess.CalledProcessError as e:
        # Try with --validate=false if validation fails
        log_info("Standard apply failed, trying with --validate=false...")
        try:
            run_command(
                ["kubectl", "apply", "-f", str(crd_output_path), "--validate=false"],
                check=True,
                capture_output=True
            )
            log_success("CRD applied to cluster (with --validate=false)")
        except subprocess.CalledProcessError:
            log_error("Failed to apply CRD")
            log_info("CRD generated but not applied. Apply manually with:")
            log_info(f"   kubectl apply -f {crd_output_path}")
            sys.exit(1)
    
    # Wait for CRDs to be established
    log_info("Waiting for CRDs to be established...")
    crd_names = [
        "ipclaims.dcops.microscaler.io",
        "ippools.dcops.microscaler.io",
        "bootprofiles.dcops.microscaler.io",
        "bootintents.dcops.microscaler.io",
        "netboxprefixes.dcops.microscaler.io",
        "netboxtenants.dcops.microscaler.io",
        "netboxsites.dcops.microscaler.io",
        "netboxroles.dcops.microscaler.io",
        "netboxtags.dcops.microscaler.io",
        "netboxaggregates.dcops.microscaler.io",
    ]
    
    max_attempts = 30  # Wait up to 1 minute
    for crd_name in crd_names:
        for attempt in range(max_attempts):
            result = run_command(
                ["kubectl", "get", "crd", crd_name, "-o", "jsonpath={.status.conditions[?(@.type==\"Established\")].status}"],
                check=False,
                capture_output=True
            )
            
            if result.returncode == 0 and result.stdout.strip() == "True":
                log_success(f"CRD {crd_name} is established")
                break
            
            if attempt < max_attempts - 1:
                import time
                time.sleep(2)
        else:
            log_error(f"CRD {crd_name} not established after {max_attempts * 2} seconds")
            log_info("Resources may fail to apply if CRD is not ready")
    
    log_success("CRD generation and application complete")


if __name__ == "__main__":
    main()

