#!/usr/bin/env python3
"""
Generate CRDs from Rust code — local-only, no cluster interaction.

This script:
1. Builds the crdgen binary
2. Runs crdgen to generate CRD YAML to config/crd/all-crds.yaml

It does NOT apply CRDs to the cluster — that's handled by:
- Flux in production
- Manual `kubectl apply` when needed during development

Usage:
    python3 scripts/generate_crds_local.py
"""

import os
import platform
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
    
    log_success("CRD generation complete (local only — not applied to cluster)")


if __name__ == "__main__":
    main()
