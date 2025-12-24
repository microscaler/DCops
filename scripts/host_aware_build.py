#!/usr/bin/env python3
"""
Host-aware build script for Rust binaries.

Selects the correct build strategy based on host OS/arch:
- macOS: cargo zigbuild --target x86_64-unknown-linux-musl
- Linux x86_64: cargo build --target x86_64-unknown-linux-musl with musl-gcc linker

This script replaces host-aware-build.sh and provides better error handling.

Usage: host_aware_build.py [extra cargo args...]
"""

import os
import platform
import shutil
import subprocess
import sys


def check_command(cmd):
    """Check if a command exists in PATH."""
    return shutil.which(cmd) is not None


def main():
    """Main build function."""
    os_name = platform.system()
    arch = platform.machine()
    
    # Get extra cargo args from command line
    cargo_args = sys.argv[1:] if len(sys.argv) > 1 else []
    
    use_zigbuild = True
    if os_name == "Linux" and arch == "x86_64":
        use_zigbuild = False
    
    if use_zigbuild:
        # macOS: Use cargo zigbuild (handles OpenSSL cross-compilation automatically)
        # cargo-zigbuild provides a proper linker for musl targets on macOS
        if not check_command("cargo-zigbuild"):
            print("❌ Error: cargo-zigbuild is required for musl builds on macOS", file=sys.stderr)
            print("   Install it with: cargo install cargo-zigbuild", file=sys.stderr)
            sys.exit(1)
        
        cmd = ["cargo", "zigbuild", "--target", "x86_64-unknown-linux-musl"] + cargo_args
        os.execvp("cargo", ["cargo", "zigbuild", "--target", "x86_64-unknown-linux-musl"] + cargo_args)
    else:
        # Linux x86_64: Use musl-gcc linker
        if not check_command("musl-gcc"):
            print("❌ Error: musl-gcc is required for musl builds on Linux", file=sys.stderr)
            print("   Install it with your package manager (e.g., apt-get install musl-tools)", file=sys.stderr)
            sys.exit(1)
        
        env = os.environ.copy()
        env["CC_x86_64_unknown_linux_musl"] = "musl-gcc"
        env["CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER"] = "musl-gcc"
        
        cmd = ["cargo", "build", "--target", "x86_64-unknown-linux-musl"] + cargo_args
        os.execvpe("cargo", cmd, env)


if __name__ == "__main__":
    main()

