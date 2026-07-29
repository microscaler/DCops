#!/usr/bin/env python3
"""
Verify prerequisites for deploying DCops to the shared Kind cluster (kind-kind).

DCops does not create its own cluster or registry. Use shared-kind-cluster first:
  cd ../shared-kind-cluster && just dev-up   # or just cluster-create + just registry-wire
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

CLUSTER_NAME = "kind"
KUBE_CONTEXT = "kind-kind"
REGISTRY_NAME = "kind-registry"
REGISTRY_PORT = "5001"
DEFAULT_SHARED_KIND_ROOT = Path(__file__).resolve().parent.parent.parent / "shared-kind-cluster"


def log_info(msg: str) -> None:
    print(f"[INFO] {msg}")


def log_error(msg: str) -> None:
    print(f"[ERROR] {msg}", file=sys.stderr)


def check_command(cmd: str) -> None:
    if not shutil.which(cmd):
        log_error(f"{cmd} is not installed. Please install it first.")
        sys.exit(1)


def shared_kind_root() -> Path:
    override = os.environ.get("SHARED_KIND_CLUSTER_ROOT")
    if override:
        return Path(override).expanduser().resolve()
    return DEFAULT_SHARED_KIND_ROOT


def run(cmd: list[str], *, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, capture_output=True, text=True, check=check)


def verify_docker() -> None:
    log_info("Checking Docker daemon...")
    result = run(["docker", "info"], check=False)
    if result.returncode != 0:
        log_error("Docker daemon is not running")
        sys.exit(1)
    log_info("Docker daemon is running")


def verify_kind_cluster() -> None:
    log_info(f"Checking Kind cluster '{CLUSTER_NAME}'...")
    result = run(["kind", "get", "clusters"], check=False)
    if result.returncode != 0 or CLUSTER_NAME not in result.stdout.split():
        log_error(
            f"Shared Kind cluster '{CLUSTER_NAME}' not found. "
            f"From {shared_kind_root()} run: just cluster-create"
        )
        sys.exit(1)
    log_info(f"Kind cluster '{CLUSTER_NAME}' exists")


def use_kube_context() -> None:
    log_info(f"Setting kubectl context to {KUBE_CONTEXT}...")
    result = run(["kubectl", "config", "use-context", KUBE_CONTEXT], check=False)
    if result.returncode != 0:
        log_error(f"Could not switch to context {KUBE_CONTEXT}: {result.stderr.strip()}")
        sys.exit(1)
    log_info(f"Context set to {KUBE_CONTEXT}")


def verify_registry() -> None:
    log_info(f"Checking local registry '{REGISTRY_NAME}' on port {REGISTRY_PORT}...")
    result = run(["docker", "ps", "--format", "{{.Names}}"], check=False)
    if REGISTRY_NAME not in result.stdout.split():
        root = shared_kind_root()
        log_error(
            f"Registry '{REGISTRY_NAME}' is not running. "
            f"From {root} run: just registry && just registry-wire"
        )
        sys.exit(1)
    log_info(f"Registry '{REGISTRY_NAME}' is running")


def wire_registry_if_needed() -> None:
    root = shared_kind_root()
    justfile = root / "justfile"
    if not justfile.is_file():
        log_info(
            f"shared-kind-cluster not found at {root}; skipping registry-wire "
            "(set SHARED_KIND_CLUSTER_ROOT if your layout differs)"
        )
        return

    log_info("Ensuring registry is wired to Kind nodes...")
    result = subprocess.run(
        ["just", "registry-wire"],
        cwd=root,
        check=False,
    )
    if result.returncode != 0:
        log_error("registry-wire failed; fix shared-kind-cluster before continuing")
        sys.exit(1)


def apply_platform_namespaces() -> None:
    root = shared_kind_root()
    platform_ns = root / "k8s" / "platform-namespaces.yaml"
    if not platform_ns.is_file():
        log_info(f"No platform-namespaces.yaml at {platform_ns}; skipping")
        return

    log_info("Applying shared platform namespaces...")
    result = run(["kubectl", "apply", "-f", str(platform_ns)], check=False)
    if result.returncode != 0:
        log_error(f"Failed to apply platform namespaces: {result.stderr.strip()}")
        sys.exit(1)


def main() -> None:
    log_info("Verifying shared Kind cluster prerequisites for DCops...")
    check_command("docker")
    check_command("kind")
    check_command("kubectl")

    verify_docker()
    verify_kind_cluster()
    use_kube_context()
    verify_registry()
    wire_registry_if_needed()
    apply_platform_namespaces()
    log_info("Shared Kind cluster is ready for DCops Tilt")


if __name__ == "__main__":
    main()
