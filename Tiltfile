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

# ====================
# Configuration
# ====================

# Restrict to kind cluster
allow_k8s_contexts(['kind-dcops'])

# Configure default registry for Kind cluster
# Tilt will automatically push docker_build images to this registry
# The registry is set up by scripts/setup_kind.py
default_registry('localhost:5000')

# Get the directory where this Tiltfile is located
DCops_DIR = '.'

# ====================
# NetBox Deployment
# ====================
# NetBox is deployed via kustomize
# Port forwards are configured here for convenient access

# Deploy NetBox using kustomize
k8s_yaml(kustomize('%s/config/netbox' % DCops_DIR))

# Configure NetBox resource with port forwards
# Forward directly to pod container port 8080 for stability
# Format: 'local_port:container_port' where container_port is the actual port the app listens on
k8s_resource(
    'netbox',
    labels=['infrastructure'],
    port_forwards=[
        '8001:8080',  # NetBox web UI: localhost:8001 -> pod:8080 (direct to container)
    ],
)

# PostgreSQL (optional port forward for debugging)
k8s_resource(
    'postgres',
    labels=['infrastructure'],
    port_forwards=[
        # '5432:5432',  # Uncomment if you need direct database access
    ],
)

# Redis (optional port forward for debugging)
k8s_resource(
    'redis',
    labels=['infrastructure'],
    port_forwards=[
        # '6379:6379',  # Uncomment if you need direct Redis access
    ],
)

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
    cmd='python3 scripts/get_netbox_token_from_db.py 2>&1 || echo "⚠️  Token not found in database. Create token in NetBox UI at http://localhost:8001/user/api-tokens/ with key \"dcops-controller\", then this script will retrieve it automatically."',
    deps=[
        'scripts/get_netbox_token_from_db.py',
    ],
    resource_deps=['netbox', 'postgres'],  # Wait for NetBox and PostgreSQL to be ready
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
    resource_deps=['manage-netbox-token'],  # Ensure token is set before controllers start
    labels=['infrastructure'],
    allow_parallel=True,
)

# ====================
# NetBox CRD Examples
# ====================
# Apply example NetBox CRDs for development/testing
# These are applied after CRDs are generated and before controllers start
# This ensures the controller has resources to reconcile on startup
# The script automatically discovers all YAML files in config/examples/
# and applies them. This is more maintainable than a static list.
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
REGISTRY = 'localhost:5000'
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
    cmd='python3 scripts/verify_netbox_crs.py --all 2>&1 || echo "⚠️  Some CRs failed verification. Check controller logs and CR status."',
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
# Future Controllers
# ====================
# Additional controllers will be added here as they're implemented

