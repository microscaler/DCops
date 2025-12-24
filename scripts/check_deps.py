#!/usr/bin/env python3
"""
Check prerequisites for DCops development.

Verifies that all required tools are installed and available.
"""

import shutil
import sys


def check_command(cmd, install_hint=None):
    """Check if a command exists."""
    if shutil.which(cmd):
        print(f"✅ {cmd} is installed")
        return True
    else:
        print(f"❌ {cmd} is not installed", file=sys.stderr)
        if install_hint:
            print(f"   {install_hint}", file=sys.stderr)
        return False


def main():
    """Check all prerequisites."""
    print("Checking prerequisites for DCops development...")
    print()
    
    all_ok = True
    
    # Required tools
    all_ok &= check_command("docker", "Install Docker Desktop: https://www.docker.com/products/docker-desktop")
    all_ok &= check_command("kind", "Install with: brew install kind (macOS) or https://kind.sigs.k8s.io/docs/user/quick-start/")
    all_ok &= check_command("kubectl", "Install with: brew install kubectl (macOS) or https://kubernetes.io/docs/tasks/tools/")
    all_ok &= check_command("cargo", "Install Rust: https://rustup.rs/")
    all_ok &= check_command("just", "Install with: cargo install just or brew install just")
    
    # Optional but recommended
    print()
    print("Optional tools:")
    check_command("tilt", "Install with: brew install tilt (macOS) or https://docs.tilt.dev/install.html")
    check_command("cargo-zigbuild", "Install with: cargo install cargo-zigbuild (for macOS cross-compilation)")
    check_command("musl-gcc", "Install with: apt-get install musl-tools (Linux) or brew install filosottile/musl-cross/musl-cross")
    
    print()
    if all_ok:
        print("✅ All required prerequisites are installed!")
        sys.exit(0)
    else:
        print("❌ Some required prerequisites are missing. Please install them and try again.", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

