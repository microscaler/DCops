#!/usr/bin/env python3
"""
Development environment shutdown script.

Stops DCops Tilt only. The shared Kind cluster and kind-registry are left running.
"""

import subprocess
import sys

TILT_PORT = "10354"


def log_info(msg: str) -> None:
    print(f"[INFO] {msg}")


def stop_tilt() -> None:
    log_info("Stopping DCops Tilt...")
    result = subprocess.run(
        ["tilt", "down", "--port", TILT_PORT],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode == 0:
        log_info("Tilt stopped")
        return

    # Fallback if tilt down fails (e.g. no session on that port)
    subprocess.run(["pkill", "-f", f"tilt up --host 0.0.0.0 --port {TILT_PORT}"], check=False)
    log_info("Tilt stop attempted (tilt down + pkill fallback)")


def main() -> None:
    log_info("Stopping DCops development environment (Tilt only)...")
    stop_tilt()
    log_info("Shared Kind cluster and kind-registry were not modified")


if __name__ == "__main__":
    main()
