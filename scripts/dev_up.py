#!/usr/bin/env python3
"""
Development environment startup script.

Deploys DCops to the shared-k8s (k3s) cluster via Tilt.
Does not create or destroy any cluster — the shared-k8s cluster is managed by the
sibling repo microscaler/shared-k8s-cluster.
"""

import subprocess
import sys

TILT_PORT = "10354"
NETBOX_UI_PORT = "8011"


def log_info(msg: str) -> None:
    print(f"[INFO] {msg}")


def start_tilt() -> None:
    log_info("Starting Tilt...")
    log_info(f"   Tilt UI: http://0.0.0.0:{TILT_PORT} (LAN: http://<this-host>:{TILT_PORT}/)")
    log_info(f"   NetBox UI: http://localhost:{NETBOX_UI_PORT} (via Tilt port forward)")
    log_info("   Kea Control Agent: http://localhost:8010 (via Tilt port forward)")
    log_info("   Default NetBox credentials: admin / admin")
    subprocess.run(
        ["tilt", "up", "--host", "0.0.0.0", "--port", TILT_PORT],
        check=False,
    )


def main() -> None:
    log_info("Starting DCops development environment (shared-k8s cluster)...")

    for cmd in ("docker", "kubectl", "tilt"):
        import shutil

        if not shutil.which(cmd):
            print(f"[ERROR] {cmd} is not installed. Please install it first.", file=sys.stderr)
            sys.exit(1)

    start_tilt()


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print()
        log_info("Shutting down gracefully...")
        log_info("   Tilt has been stopped")
        log_info("   The shared-k8s cluster is still running")
        log_info("   Use 'just dev-down' to stop Tilt without touching the cluster")
        print()
        log_info("Shutdown complete")
        sys.exit(0)
