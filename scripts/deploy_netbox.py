#!/usr/bin/env python3
"""
Deploy NetBox to Kind cluster.

Deploys NetBox with PostgreSQL and Redis for local development.
"""

import subprocess
import sys
import time
from pathlib import Path


def log_info(msg):
    """Print info message."""
    print(f"[INFO] {msg}")


def log_warn(msg):
    """Print warning message."""
    print(f"[WARN] {msg}")


def log_error(msg):
    """Print error message."""
    print(f"[ERROR] {msg}", file=sys.stderr)


def run_command(cmd, check=True, capture_output=True, **kwargs):
    """Run a command and return the result."""
    result = subprocess.run(
        cmd,
        shell=isinstance(cmd, str),
        capture_output=capture_output,
        text=True,
        check=check,
        **kwargs
    )
    return result


def wait_for_deployment(namespace, deployment_name, timeout=300):
    """Wait for deployment to be ready."""
    log_info(f"Waiting for {deployment_name} to be ready...")
    cmd = f"kubectl wait --for=condition=available --timeout={timeout}s deployment/{deployment_name} -n {namespace}"
    result = run_command(cmd, check=False, capture_output=True)
    
    if result.returncode == 0:
        log_info(f"✅ {deployment_name} is ready")
        return True
    else:
        log_warn(f"⚠️  {deployment_name} may not be ready: {result.stderr}")
        return False


def wait_for_pvc(namespace, pvc_name, timeout=60):
    """Wait for PVC to be bound."""
    log_info(f"Waiting for PVC {pvc_name} to be bound...")
    max_attempts = timeout // 5
    for i in range(max_attempts):
        cmd = f"kubectl get pvc {pvc_name} -n {namespace} -o jsonpath='{{.status.phase}}'"
        result = run_command(cmd, check=False, capture_output=True)
        
        if result.returncode == 0 and result.stdout.strip() == "Bound":
            log_info(f"✅ PVC {pvc_name} is bound")
            return True
        
        if i < max_attempts - 1:
            time.sleep(5)
    
    log_warn(f"⚠️  PVC {pvc_name} may not be bound")
    return False


def deploy_netbox():
    """Deploy NetBox to the cluster."""
    log_info("🚀 Deploying NetBox to Kind cluster...")
    
    # Get script directory and project root
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    netbox_config = project_root / "config" / "netbox"
    
    if not netbox_config.exists():
        log_error(f"NetBox config directory not found at {netbox_config}")
        sys.exit(1)
    
    # Deploy using kustomize
    log_info("Applying NetBox manifests...")
    result = run_command(
        ["kubectl", "apply", "-k", str(netbox_config)],
        check=False,
        capture_output=True
    )
    
    if result.returncode != 0:
        log_error(f"Failed to deploy NetBox: {result.stderr}")
        sys.exit(1)
    
    log_info("✅ NetBox manifests applied")
    
    # Wait for PVCs
    wait_for_pvc("netbox", "postgres-data")
    
    # Wait for PostgreSQL
    wait_for_deployment("netbox", "postgres", timeout=120)
    
    # Wait for Redis
    wait_for_deployment("netbox", "redis", timeout=120)
    
    # Wait for NetBox (longer timeout due to migrations)
    wait_for_deployment("netbox", "netbox", timeout=600)
    
    log_info("✅ NetBox deployment complete!")
    log_info("")
    log_info("Access NetBox:")
    log_info("  kubectl port-forward -n netbox svc/netbox 8000:80")
    log_info("  Then open http://localhost:8000 in your browser")
    log_info("")
    log_info("Default credentials:")
    log_info("  Username: admin")
    log_info("  Password: admin")
    log_info("")
    log_info("To create an API token:")
    log_info("  1. Log in to NetBox")
    log_info("  2. Go to User Menu > API Tokens")
    log_info("  3. Create a new token")
    log_info("  4. Use the token in your NetBox client configuration")


def main():
    """Main deployment function."""
    # Check prerequisites
    result = run_command("kubectl version --client", check=False, capture_output=True)
    if result.returncode != 0:
        log_error("kubectl is not available or cluster is not accessible")
        sys.exit(1)
    
    deploy_netbox()


if __name__ == "__main__":
    main()

