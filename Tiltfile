# DCops Tiltfile
#
# This Tiltfile manages local development resources:
# - Image builds + pushes to shared LAN registry (Flux discovers these)
# - Infrastructure: NetBox, kubectl proxy for local dev
#
# Flux owns all workload deployment: namespaces, HelmReleases, runtime reconciliation.
# Tilt does NOT deploy workloads to the cluster — it only builds and publishes images.
#
# Usage: tilt up
#
# Resources are organized into parallel streams using labels:
# - 'infrastructure' label: NetBox, kubectl proxy
# - 'controllers' label: controller image builds
# - 'docs' label: docs/dashboard Vite dev servers
# - 'docker' label: image builds

# ====================
# Configuration
# ====================

# Shared k8s (k3s) platform cluster — no kind. Kubeconfig + context come from the
# sibling repo microscaler/shared-gitops-k8s-cluster, matching hauliage and the other
# microscaler projects.
_SHARED_K8S_KCFG = os.path.abspath('../shared-gitops-k8s-cluster/kubeconfig/shared-k8s.yaml')
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
# NetBox is deployed via kustomize (Flux or Tilt-applied once for dev)
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
    cmd='python3 scripts/get_netbox_token_from_db.py 2>&1 || echo "Token not found in database. Create token in NetBox UI at http://localhost:%d/user/api-tokens/ with key \\\\"dcops-controller\\", then this script will retrieve it automatically."' % NETBOX_UI_HOST_PORT,
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
    cmd='python3 scripts/setup_netbox_tenant.py --tenant-name datacenter-tenant --secret-name netbox-token-datacenter-tenant --namespace default 2>&1 || echo "Tenant setup failed. Check NetBox logs and ensure admin credentials are correct."',
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
# Generate CRDs from Rust code to config/crd/all-crds.yaml.
# This is a local build step only — it does NOT apply CRDs to the cluster.
# CRDs in the cluster are managed by Flux (production) or applied manually.
local_resource(
    'generate-crds',
    cmd='python3 scripts/generate_crds_local.py',
    deps=[
        'crates/crds/src',
        'crates/crds/Cargo.toml',
        'Cargo.toml',
        'Cargo.lock',
        'scripts/generate_crds_local.py',
    ],
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
# Image Builds (Tilt builds only — Flux deploys)
# ====================
# Mirrors hauliage's pattern: local_resource runs copy-binary + docker build + push.
# Flux discovers the dev-<timestamp> tagged images via ImagePolicyWatch.
#
# Tilt does NOT apply k8s manifests for workloads — Flux owns all deployment.

# ---- NetBox Controller ----
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

# Publish NetBox Controller image for Flux ImagePolicy discovery
# local_resource instead of custom_build: no Tilt image hash mangling, predictable :tilt tag
# Pushed alongside a dev-<timestamp> tag for Flux image-automation-controller
NETBOX_BINARY_PATH = 'target/x86_64-unknown-linux-musl/release/netbox-controller'
NETBOX_IMAGE_NAME = 'netbox-controller'
NETBOX_FULL_IMAGE_NAME = '%s/%s' % (_SHARED_K8S_REGISTRY, NETBOX_IMAGE_NAME)

local_resource(
    'image-%s' % NETBOX_IMAGE_NAME,
    '''set -eu
# Build the image with the predictable :tilt tag (Tilt live_update target)
docker buildx build --platform linux/amd64 -f dockerfiles/Dockerfile.netbox-controller.dev -t %s:tilt .
# Tag and push a dev-<timestamp> image for Flux image-automation-controller to discover
DEV_REF="%s:dev-$(date +%%s%%N)"
docker tag %s:tilt "$DEV_REF"
docker push "$DEV_REF"
echo "Published $DEV_REF for Flux image discovery"
''' % (
        NETBOX_IMAGE_NAME,
        NETBOX_FULL_IMAGE_NAME,
        NETBOX_IMAGE_NAME,
    ),
    deps=[
        NETBOX_BINARY_PATH,  # Build must succeed before docker build
        'dockerfiles/Dockerfile.netbox-controller.dev',
    ],
    labels=['controllers'],
    allow_parallel=True,
)

# ---- DHCP Controller ----
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

# Publish DHCP Controller image for Flux ImagePolicy discovery
DHCP_BINARY_PATH = 'target/x86_64-unknown-linux-musl/release/dhcp-controller'
DHCP_IMAGE_NAME = 'dhcp-controller'
DHCP_FULL_IMAGE_NAME = '%s/%s' % (_SHARED_K8S_REGISTRY, DHCP_IMAGE_NAME)

local_resource(
    'image-%s' % DHCP_IMAGE_NAME,
    '''set -eu
docker buildx build --platform linux/amd64 -f dockerfiles/Dockerfile.dhcp-controller.dev -t %s:tilt .
DEV_REF="%s:dev-$(date +%%s%%N)"
docker tag %s:tilt "$DEV_REF"
docker push "$DEV_REF"
echo "Published $DEV_REF for Flux image discovery"
''' % (
        DHCP_IMAGE_NAME,
        DHCP_FULL_IMAGE_NAME,
        DHCP_IMAGE_NAME,
    ),
    deps=[
        DHCP_BINARY_PATH,
        'dockerfiles/Dockerfile.dhcp-controller.dev',
    ],
    labels=['controllers'],
    allow_parallel=True,
)

# ---- PXE Server ----
# Build the PXE Server binary
# Uses host_aware_build.py for cross-compilation (macOS -> Linux)
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

# Publish PXE Server image for Flux ImagePolicy discovery
PXE_BINARY_PATH = 'target/x86_64-unknown-linux-musl/release/pxe-server'
PXE_IMAGE_NAME = 'pxe-server'
PXE_FULL_IMAGE_NAME = '%s/%s' % (_SHARED_K8S_REGISTRY, PXE_IMAGE_NAME)

# Use a staging directory for the docker build context to avoid Tilt
# detecting file changes in target/ (which causes infinite rebuild loops).
# The staging dir is unwatched by Tilt, so docker operations there are silent.
PXE_STAGING_DIR = '.tilt-staging/pxe-server'

local_resource(
    'image-%s' % PXE_IMAGE_NAME,
    '''set -eu
# Copy binary to unwatched staging dir — Tilt won't see docker ops there
rm -rf %s
mkdir -p %s
cp %s %s/pxe-server
cp dockerfiles/Dockerfile.pxe-server.dev %s/Dockerfile
docker buildx build --platform linux/amd64 -f %s/Dockerfile -t %s:tilt %s
DEV_REF="%s:dev-$(date +%%s%%N)"
docker tag %s:tilt "$DEV_REF"
docker push "$DEV_REF"
echo "Published $DEV_REF for Flux image discovery"
''' % (
        PXE_STAGING_DIR,
        PXE_STAGING_DIR,
        PXE_BINARY_PATH,
        PXE_STAGING_DIR,
        PXE_STAGING_DIR,
        PXE_STAGING_DIR,
        PXE_IMAGE_NAME,
        PXE_STAGING_DIR,
        PXE_IMAGE_NAME,
        PXE_FULL_IMAGE_NAME,
        PXE_IMAGE_NAME,
    ),
    deps=[
        PXE_BINARY_PATH,
        'dockerfiles/Dockerfile.pxe-server.dev',
    ],
    labels=['infrastructure'],
    allow_parallel=True,
)

# ====================
# Dev-only: Vite servers for local development (not deployed by Flux)
# ====================

# DCops Documentation Site (docs UI) — Vite dev server
# For local development only; in production Flux deploys the built artifacts
local_resource(
    'build-docs',
    cmd='yarn --cwd ui-docs dev --port 8801 --host 0.0.0.0',
    deps=[
        'ui-docs/src',
        'ui-docs/package.json',
        'ui-docs/yarn.lock',
    ],
    labels=['docs'],
    allow_parallel=True,
    trigger_mode=TRIGGER_MODE_MANUAL,
    auto_init=False,
)

# DCops Dashboard SPA — Vite dev server
# For local development only; in production Flux deploys the built artifacts
local_resource(
    'build-dashboard',
    cmd='yarn --cwd ui-dashboard dev --port 8802 --host 0.0.0.0',
    deps=[
        'ui-dashboard/src',
        'ui-dashboard/package.json',
        'ui-dashboard/yarn.lock',
    ],
    labels=['docs'],
    allow_parallel=True,
    trigger_mode=TRIGGER_MODE_MANUAL,
    auto_init=False,
)
