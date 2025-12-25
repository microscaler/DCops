#!/usr/bin/env python3
"""
Apply all example CRs from config/examples/ directory.

This script:
1. Discovers all YAML files in config/examples/
2. Applies them to the Kubernetes cluster
3. Handles errors gracefully (some CRs may fail if dependencies aren't ready)
4. Provides clear output about what was applied

Usage:
    python3 scripts/apply_example_crs.py
"""

import os
import sys
import subprocess
from pathlib import Path

# Get the project root directory (parent of scripts/)
SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent
EXAMPLES_DIR = PROJECT_ROOT / "config" / "examples"


def find_yaml_files(directory: Path) -> list[Path]:
    """Find all YAML files in the given directory."""
    yaml_files = []
    if directory.exists():
        for file in sorted(directory.glob("*.yaml")):
            yaml_files.append(file)
    return yaml_files


def apply_yaml_file(file_path: Path) -> tuple[bool, str]:
    """Apply a single YAML file using kubectl."""
    try:
        result = subprocess.run(
            ["kubectl", "apply", "-f", str(file_path)],
            capture_output=True,
            text=True,
            check=False,  # Don't raise on error, we'll handle it
        )
        if result.returncode == 0:
            return True, result.stdout.strip()
        else:
            return False, result.stderr.strip() or result.stdout.strip()
    except Exception as e:
        return False, str(e)


def main():
    """Main entry point."""
    print("📋 Discovering example CRs in config/examples/...")
    
    # Find all YAML files
    yaml_files = find_yaml_files(EXAMPLES_DIR)
    
    if not yaml_files:
        print(f"⚠️  No YAML files found in {EXAMPLES_DIR}")
        return 0
    
    print(f"📦 Found {len(yaml_files)} example CR file(s)")
    print()
    
    # Apply each file
    applied = 0
    failed = 0
    errors = []
    
    for yaml_file in yaml_files:
        file_name = yaml_file.name
        print(f"  Applying {file_name}...", end=" ", flush=True)
        
        success, output = apply_yaml_file(yaml_file)
        
        if success:
            print("✅")
            applied += 1
        else:
            print("❌")
            failed += 1
            errors.append((file_name, output))
    
    print()
    print("=" * 60)
    print(f"📊 Summary: {applied} applied, {failed} failed")
    
    if errors:
        print()
        print("⚠️  Errors:")
        for file_name, error in errors:
            print(f"  {file_name}:")
            # Print first few lines of error
            error_lines = error.split("\n")[:3]
            for line in error_lines:
                print(f"    {line}")
            if len(error.split("\n")) > 3:
                print(f"    ... ({len(error.split('\n')) - 3} more lines)")
    
    # Return 0 if all applied, 1 if any failed
    # Note: Some failures are expected if dependencies aren't ready yet
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())

