#!/usr/bin/env python3
"""
Show cluster and controller status for DCops on the shared Kind cluster (kind-kind).
"""

import subprocess
import sys

KUBE_CONTEXT = "kind-kind"
CLUSTER_NAME = "kind"
REGISTRY_NAME = "kind-registry"


def run_command(cmd, check=False):
    result = subprocess.run(
        cmd,
        shell=isinstance(cmd, str),
        capture_output=True,
        text=True,
        check=check,
    )
    return result


def main():
    print("📊 DCops Cluster and Controller Status (shared Kind)")
    print("=" * 50)
    print()

    result = run_command("kind get clusters", check=False)
    if CLUSTER_NAME not in result.stdout:
        print(f"❌ Shared Kind cluster '{CLUSTER_NAME}' not found")
        print("   From ../shared-kind-cluster run: just cluster-create")
        sys.exit(1)

    print(f"✅ Shared Kind cluster '{CLUSTER_NAME}' exists")
    print()

    ctx = run_command("kubectl config current-context", check=False)
    if ctx.returncode == 0:
        print(f"📌 kubectl context: {ctx.stdout.strip()}")
        if ctx.stdout.strip() != KUBE_CONTEXT:
            print(f"   ⚠️  Expected {KUBE_CONTEXT}; run: kubectl config use-context {KUBE_CONTEXT}")
    print()

    print("📦 Cluster Nodes:")
    result = run_command("kubectl get nodes", check=False)
    if result.returncode == 0:
        print(result.stdout)
    else:
        print("   ⚠️  Could not get node status")
    print()

    print("📁 DCops Namespaces:")
    for ns in ("dcops-system", "netbox"):
        result = run_command(["kubectl", "get", "namespace", ns], check=False)
        if result.returncode == 0:
            print(f"   ✅ {ns}")
        else:
            print(f"   ❌ {ns} (not found — run 'just dev-up')")
    print()

    print("📝 CRDs:")
    crds = [
        "bootprofiles.dcops.microscaler.io",
        "bootintents.dcops.microscaler.io",
        "ippools.dcops.microscaler.io",
        "ipclaims.dcops.microscaler.io",
    ]
    for crd in crds:
        result = run_command(["kubectl", "get", "crd", crd], check=False)
        if result.returncode == 0:
            print(f"   ✅ {crd}")
        else:
            print(f"   ❌ {crd} (not installed)")
    print()

    print("🎮 Deployed workloads:")
    for ns, label in (
        ("dcops-system", "app.kubernetes.io/part-of=dcops"),
        ("netbox", "app=netbox"),
    ):
        result = run_command(
            f"kubectl get pods -n {ns} -l {label} -o wide",
            check=False,
        )
        if result.returncode == 0 and result.stdout.strip():
            print(f"   [{ns}]")
            print(result.stdout)
        else:
            print(f"   [{ns}] no matching pods (Tilt may be down)")
    print()

    print("📦 Shared Registry:")
    result = run_command("docker ps --format '{{.Names}}'", check=False)
    if REGISTRY_NAME in result.stdout:
        print(f"   ✅ {REGISTRY_NAME} is running (localhost:5001)")
    else:
        print(f"   ⚠️  {REGISTRY_NAME} is not running — from shared-kind-cluster: just registry")
    print()

    print("🔗 Local URLs (when Tilt is up on port 10354):")
    print("   NetBox UI: http://localhost:8011")
    print("   Kea Control Agent: http://localhost:8010")
    print("   DCops UI: http://localhost:8800")
    print("   Tilt UI: http://localhost:10354")
    print()


if __name__ == "__main__":
    main()
