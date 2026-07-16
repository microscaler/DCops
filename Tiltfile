# DCops Tiltfile
#
# This Tiltfile manages local development resources:
# - NetBox deployment with port forwards
# - Controllers (to be added as they're implemented)
#
# Usage: tilt up
#
# Resources are organized into parallel streams using labels:
# - 'infrastructure' label: NetBox, PostgreSQL, Redis
# - 'controllers' label: DCops controllers (to be added)
# - 'docs' label: DCops UI documentation site

# ====================
# Configuration
# ====================

# Shared k8s (k3s) platform cluster — no kind. Kubeconfig + context come from the
# sibling repo microscaler/shared-k8s-cluster, matching hauliage and the other
# microscaler projects.
_SHARED_K8S_KCFG = os.path.abspath('../shared-k8s-cluster/kubeconfig/shared-k8s.yaml')
_SHARED_K8S_REGISTRY = '10.177.76.220:5000'
allow_k8s_contexts(['shared-k8s'])
if os.path.exists(_SHARED_K8S_KCFG):
    os.putenv('KUBECONFIG', _SHARED_K8S_KCFG)

# Shared LAN registry (same as hauliage / the other microscaler projects).
default_registry(_SHARED_K8S_REGISTRY)

# Host ports avoid shared-k8s conflicts: PriceWhisperer uses 8000, LLMRouter uses 8001
NETBOX_UI_HOST_PORT = 8011
KEA_CONTROL_AGENT_HOST_PORT = 8010

# Get the directory where this Tiltfile is located
DCops_DIR = '.'

# ====================
# NetBox Deployment
# ====================
# NetBox is deployed via kustomize
# Port forwards are configured here for convenient access

# Deploy NetBox using kustomize
k8s_yaml(kustomize('%s/config/netbox' % DCops_DIR))

# ---- NetBox database lifecycle (label `data`), modeled on hauliage's set ----

# (Re)apply the per-app DB credentials secret on demand. The kustomize above also
# applies it at startup; this is the manual "database-env" button (parity with
# hauliage-database-env).
local_resource(
    'netbox-database-env',
    cmd='kubectl apply -f config/netbox/netbox-db-credentials.yaml',
    deps=['config/netbox/netbox-db-credentials.yaml'],
    labels=['data'],
    trigger_mode=TRIGGER_MODE_MANUAL,
    auto_init=False,
    allow_parallel=True,
)

# Provision the NetBox role + database on the shared Postgres before NetBox
# migrates. Idempotent; mirrors hauliage-db-init. Runs once on `tilt up`, then
# re-runnable from the Tilt UI (MANUAL stops file-watch re-triggers).
local_resource(
    'netbox-db-init',
    cmd='chmod +x scripts/setup-db.sh && bash scripts/setup-db.sh',
    deps=['scripts/setup-db.sh'],
    labels=['data'],
    allow_parallel=False,
    trigger_mode=TRIGGER_MODE_MANUAL,
    auto_init=True,
)

# Configure NetBox resource with port forwards
# Forward directly to pod container port 8080 for stability
# Format: 'local_port:container_port' where container_port is the actual port the app listens on
k8s_resource(
    'netbox',
    labels=['infrastructure'],
    port_forwards=[
        '%d:8080' % NETBOX_UI_HOST_PORT,  # NetBox web UI (8011 avoids shared-k8s 8000/8001)
    ],
    resource_deps=['netbox-db-init'],
    # Applied once on `tilt up` (after the DB is bootstrapped); MANUAL so you
    # control re-applies from the Tilt UI rather than on every manifest change.
    trigger_mode=TRIGGER_MODE_MANUAL,
)

# Run NetBox schema migrations on demand. They also run in the deployment's init
# container, but this lets you (re)apply migrations later without a redeploy.
# (NetBox migrations ship with the image, so there is no separate SQL-generation
# step like hauliage-migrate; this is the apply-migrations equivalent.)
local_resource(
    'netbox-migrate',
    cmd='kubectl exec -n netbox deployment/netbox -- /opt/netbox/venv/bin/python /opt/netbox/netbox/manage.py migrate',
    resource_deps=['netbox'],
    labels=['data'],
    trigger_mode=TRIGGER_MODE_MANUAL,
    auto_init=False,
)

# Postgres and Redis are NOT deployed by DCops — NetBox uses the shared-k8s
# cluster's services in the `data` namespace (postgres.data / redis.data).

# ====================
# NetBox Token Management
# ====================
# Automatically manage NetBox API token in Kubernetes secret
# This resource manages the NetBox API token:
# 1. Waits for NetBox and PostgreSQL to be ready
# 2. Queries PostgreSQL database directly to retrieve existing token
# 3. Updates the Kubernetes secret with the token
# Note: This approach works in CI/CD environments where UI access is not available
# The token must exist in NetBox (created via UI or API) before this script runs
local_resource(
    'manage-netbox-token',
    # Wait for PostgreSQL to be ready, then query database for token
    cmd='python3 scripts/get_netbox_token_from_db.py 2>&1 || echo "⚠️  Token not found in database. Create token in NetBox UI at http://localhost:%d/user/api-tokens/ with key \"dcops-controller\", then this script will retrieve it automatically."' % NETBOX_UI_HOST_PORT,
    deps=[
        'scripts/get_netbox_token_from_db.py',
    ],
    resource_deps=['netbox'],  # Wait for NetBox to be ready
    labels=['infrastructure'],
    allow_parallel=False,
    # Runs when script changes or NetBox/PostgreSQL becomes ready
)

# ====================
# NetBox Tenant Setup
# ====================
# Automatically setup NetBox tenant, user, and API token
# This resource:
# 1. Waits for NetBox to be ready
# 2. Creates tenant in NetBox (or uses existing)
# 3. Creates API token for admin user (or uses existing)
# 4. Creates Kubernetes Secret that Tenant CRDs can reference
# Note: The secret name must match the tokenSecret.name in the Tenant CRD
local_resource(
    'setup-netbox-tenant',
    # Setup tenant, user, and token for datacenter-tenant
    cmd='python3 scripts/setup_netbox_tenant.py --tenant-name datacenter-tenant --secret-name netbox-token-datacenter-tenant --namespace default 2>&1 || echo "⚠️  Tenant setup failed. Check NetBox logs and ensure admin credentials are correct."',
    deps=[
        'scripts/setup_netbox_tenant.py',
    ],
    resource_deps=['netbox'],  # Wait for NetBox to be ready
    labels=['infrastructure'],
    allow_parallel=False,
    # Runs when script changes or NetBox/PostgreSQL becomes ready
)

# ====================
# CRD Generation
# ====================
# Generate and apply CRDs when CRD code changes
# This ensures CRDs are always up-to-date with the Rust code
local_resource(
    'generate-crds',
    cmd='python3 scripts/generate_crds.py',
    deps=[
        'crates/crds/src',
        'crates/crds/Cargo.toml',
        'Cargo.toml',
        'Cargo.lock',
        'scripts/generate_crds.py',
    ],
    resource_deps=['manage-netbox-token', 'setup-netbox-tenant'],  # Ensure tokens are set before controllers start
    labels=['infrastructure'],
    allow_parallel=True,
)

# ====================
# NetBox CRD Examples
# ====================
# Apply example NetBox CRDs for development/testing
# These are applied after CRDs are generated and before controllers start
# This ensures the controller has resources to reconcile on startup
# The script automatically discovers all YAML files in config/examples/ and subdirectories
# and applies them. Examples are organized as:
# - config/examples/platform/ - Platform-level resources (manufacturer, device-type, etc.)
# - config/examples/tenant-<name>/ - Tenant-specific resources
local_resource(
    'apply-netbox-examples',
    cmd='python3 scripts/apply_example_crs.py',
    deps=[
        'scripts/apply_example_crs.py',
        'config/examples',
    ],
    resource_deps=['generate-crds'],  # Wait for CRDs to be generated and applied
    labels=['infrastructure'],
    allow_parallel=False,  # Apply sequentially to respect dependencies
    # This will run when example files change or when CRDs are ready
)

# ====================
# NetBox Controller
# ====================
# Build the NetBox Controller binary
# Uses host_aware_build.py for cross-compilation (macOS -> Linux)
# Note: host_aware_build.py passes all args to cargo, so --release works
local_resource(
    'build-netbox-controller',
    cmd='python3 scripts/host_aware_build.py --release -p netbox-controller',
    deps=[
        'controllers/netbox/src',
        'controllers/netbox/Cargo.toml',
        'crates/crds/src',
        'crates/netbox-client/src',
        'Cargo.toml',
        'Cargo.lock',
        'scripts/host_aware_build.py',
    ],
    resource_deps=['generate-crds'],  # Wait for CRDs to be generated and applied
    labels=['controllers'],
    allow_parallel=True,
)

# Build Docker image for NetBox Controller
# Use custom_build to ensure binary exists before Docker build
# This matches the pattern from secret-manager-controller
# Note: We build for linux/amd64 platform even on Apple Silicon
# because the binary is cross-compiled for x86_64-unknown-linux-musl
# The 'deps' parameter ensures the binary exists before Docker build
BINARY_PATH = 'target/x86_64-unknown-linux-musl/release/netbox-controller'
IMAGE_NAME = 'netbox-controller'
REGISTRY = _SHARED_K8S_REGISTRY
FULL_IMAGE_NAME = '%s/%s' % (REGISTRY, IMAGE_NAME)

custom_build(
    IMAGE_NAME,
    'docker buildx build --platform linux/amd64 -f dockerfiles/Dockerfile.netbox-controller.dev -t %s:tilt . && docker tag %s:tilt %s:tilt && docker push %s:tilt' % (
        IMAGE_NAME,
        IMAGE_NAME,
        FULL_IMAGE_NAME,
        FULL_IMAGE_NAME
    ),
    deps=[
        BINARY_PATH,  # File dependency ensures binary exists before Docker build
        'dockerfiles/Dockerfile.netbox-controller.dev',
    ],
    tag='tilt',
    live_update=[
        sync(BINARY_PATH, '/app/netbox-controller'),
        run('kill -HUP 1', trigger=[BINARY_PATH]),
    ],
)

# Deploy NetBox Controller
# This includes: namespace, serviceaccount, role (RBAC), rolebinding, secret, deployment
# RBAC permissions are automatically applied via kustomize
k8s_yaml(kustomize('%s/config/netbox-controller' % DCops_DIR))

k8s_resource(
    'netbox-controller',
    labels=['controllers'],
    resource_deps=['build-netbox-controller'],  # Wait for binary to be built before deploying
)

# ====================
# NetBox CR Verification
# ====================
# Automatically verify NetBox CR reconciliation status
# This runs periodically to ensure CRs are properly reconciled and exist in NetBox database
# Verification runs:
# - When the script changes
# - When triggered manually from Tilt UI
# - After controller becomes ready (via resource_deps)
local_resource(
    'verify-netbox-crs',
    # Script exits with code 1 on failures - don't mask it with || echo
    cmd='python3 scripts/verify_netbox_crs.py --all',
    deps=[
        'scripts/verify_netbox_crs.py',
    ],
    resource_deps=['netbox-controller'],  # Wait for controller to be running
    labels=['controllers'],
    allow_parallel=True,
    # Runs when script changes or when manually triggered from Tilt UI
    # Use Tilt UI to trigger verification manually, or it will run after controller starts
)

# ====================
# Kea DHCP Server
# ====================
# Deploy ISC Kea DHCP server with Control Agent
# Image is built and pushed by GitHub Actions in the Kea fork
# Available at:
#   - docker.io/microscaler/kea-dhcp:latest (Docker Hub)
#   - ghcr.io/microscaler/kea-dhcp:latest (GitHub Container Registry)
k8s_yaml(kustomize('%s/config/kea-dhcp' % DCops_DIR))

k8s_resource(
    'kea-dhcp',
    labels=['infrastructure'],
    port_forwards=[
        '%d:8000' % KEA_CONTROL_AGENT_HOST_PORT,  # Kea Control Agent REST API (8010 avoids host 8000)
    ],
)

# ====================
# DHCP Controller
# ====================
# Build the DHCP Controller binary
# Uses host_aware_build.py for cross-compilation (macOS -> Linux)
# Note: host_aware_build.py passes all args to cargo, so --release works
local_resource(
    'build-dhcp-controller',
    cmd='python3 scripts/host_aware_build.py --release -p dhcp-controller',
    deps=[
        'controllers/dhcp/src',
        'controllers/dhcp/Cargo.toml',
        'crates/crds/src',
        'Cargo.toml',
        'Cargo.lock',
        'scripts/host_aware_build.py',
    ],
    resource_deps=['generate-crds'],  # Wait for CRDs to be generated and applied
    labels=['controllers'],
    allow_parallel=True,
)

# Build Docker image for DHCP Controller
# Use custom_build to ensure binary exists before Docker build
# This matches the pattern from netbox-controller
# Note: We build for linux/amd64 platform even on Apple Silicon
# because the binary is cross-compiled for x86_64-unknown-linux-musl
# The 'deps' parameter ensures the binary exists before Docker build
DHCP_BINARY_PATH = 'target/x86_64-unknown-linux-musl/release/dhcp-controller'
DHCP_IMAGE_NAME = 'dhcp-controller'
DHCP_FULL_IMAGE_NAME = '%s/%s' % (REGISTRY, DHCP_IMAGE_NAME)

custom_build(
    DHCP_IMAGE_NAME,
    'docker buildx build --platform linux/amd64 -f dockerfiles/Dockerfile.dhcp-controller.dev -t %s:tilt . && docker tag %s:tilt %s:tilt && docker push %s:tilt' % (
        DHCP_IMAGE_NAME,
        DHCP_IMAGE_NAME,
        DHCP_FULL_IMAGE_NAME,
        DHCP_FULL_IMAGE_NAME
    ),
    deps=[
        DHCP_BINARY_PATH,  # File dependency ensures binary exists before Docker build
        'dockerfiles/Dockerfile.dhcp-controller.dev',
    ],
    tag='tilt',
    live_update=[
        sync(DHCP_BINARY_PATH, '/app/dhcp-controller'),
        run('kill -HUP 1', trigger=[DHCP_BINARY_PATH]),
    ],
)

# Deploy DHCP Controller
# This includes: namespace, serviceaccount, role (RBAC), rolebinding, deployment
# RBAC permissions are automatically applied via kustomize
k8s_yaml(kustomize('%s/config/dhcp-controller' % DCops_DIR))

k8s_resource(
    'dhcp-controller',
    labels=['controllers'],
    resource_deps=['build-dhcp-controller', 'kea-dhcp'],  # Wait for binary and Kea to be ready
)

# Host port 8088 → pxe-server HTTP (8080 taken by BRRTRouter on shared-k8s)
PXE_SERVER_HOST_PORT = 8088

# ====================
# PXE Server (HTTP / iPXE)
# ====================
local_resource(
    'build-pxe-server',
    cmd='python3 scripts/host_aware_build.py --release -p pxe-server',
    deps=[
        'crates/pxe-server/src',
        'crates/pxe-server/Cargo.toml',
        'crates/crds/src',
        'Cargo.toml',
        'Cargo.lock',
        'scripts/host_aware_build.py',
    ],
    resource_deps=['generate-crds'],
    labels=['infrastructure'],
    allow_parallel=True,
)

PXE_BINARY_PATH = 'target/x86_64-unknown-linux-musl/release/pxe-server'
PXE_IMAGE_NAME = 'pxe-server'
PXE_FULL_IMAGE_NAME = '%s/%s' % (REGISTRY, PXE_IMAGE_NAME)

custom_build(
    PXE_IMAGE_NAME,
    'docker buildx build --platform linux/amd64 -f dockerfiles/Dockerfile.pxe-server.dev -t %s:tilt . && docker tag %s:tilt %s:tilt && docker push %s:tilt' % (
        PXE_IMAGE_NAME,
        PXE_IMAGE_NAME,
        PXE_FULL_IMAGE_NAME,
        PXE_FULL_IMAGE_NAME
    ),
    deps=[
        PXE_BINARY_PATH,
        'dockerfiles/Dockerfile.pxe-server.dev',
    ],
    tag='tilt',
    live_update=[
        sync(PXE_BINARY_PATH, '/app/pxe-server'),
        run('kill -HUP 1', trigger=[PXE_BINARY_PATH]),
    ],
)

k8s_yaml(kustomize('%s/config/pxe-server' % DCops_DIR))

k8s_resource(
    'pxe-server',
    labels=['infrastructure'],
    resource_deps=['build-pxe-server'],
    port_forwards=[
        '%d:8080' % PXE_SERVER_HOST_PORT,
    ],
)

local_resource(
    'apply-cylon-regenesis-examples',
    cmd='kubectl apply -k config/examples/cylon-regenesis',
    deps=[
        'config/examples/cylon-regenesis',
    ],
    resource_deps=['generate-crds', 'pxe-server'],
    labels=['infrastructure'],
    allow_parallel=False,
)

# ====================
# Future Controllers
# ====================
# Additional controllers will be added here as they're implemented

# ====================
# DCops UI Documentation Site
# ====================

# Build documentation site Docker image
# Tilt will watch ui/ for changes and rebuild
# Note: The build process runs 'yarn build' in the Dockerfile
docker_build(
    'dcops-ui',
    '.',
    dockerfile='./dockerfiles/Dockerfile.dcops-ui',
    platform='linux/amd64',
    only=[
        './ui',
        './dockerfiles/Dockerfile.dcops-ui',
        './dockerfiles/nginx.dcops-ui.conf',
    ],
    ignore=[
        'ui/node_modules',
        'ui/dist',
        'ui/.git',
    ],
    # Fast path: sync src → /app, then run `yarn build` in the container.
    # This avoids a full image rebuild for every source change.
    live_update=[
        sync('./ui', '/app/ui'),
        run('cd /app && yarn build', trigger=['./ui']),
    ],
)

# ====================
# kubectl proxy for local dev
# ====================
# Provides a local HTTP proxy to the K8s API cluster (port 8001).
# Used by the Dashboard SPA to fetch CR data in Tilt dev mode.
local_resource(
    'kubectl-proxy',
    cmd='kubectl proxy --port=8001 --address=127.0.0.1 --accept-hosts="^localhost$|^127\\.0\\.0\\.1$"',
    labels=['infrastructure'],
)

# Documentation site service (ClusterIP with port forward)
k8s_yaml(kustomize('%s/config/dcops-ui' % DCops_DIR))

k8s_resource(
    'dcops-ui',
    port_forwards='8800:80',
    labels=['docs'],
    resource_deps=['kubectl-proxy'],  # ensure proxy is running before UI loads
)

