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
# IP Claim Controller
# ====================
# Build the IP Claim Controller binary
# Uses host_aware_build.py for cross-compilation (macOS -> Linux)
# Note: host_aware_build.py passes all args to cargo, so --release works
local_resource(
    'build-ip-claim-controller',
    cmd='python3 scripts/host_aware_build.py --release -p ip-claim-controller',
    deps=[
        'controllers/ip-claim/src',
        'controllers/ip-claim/Cargo.toml',
        'crates/crds/src',
        'crates/netbox-client/src',
        'Cargo.toml',
        'Cargo.lock',
        'scripts/host_aware_build.py',
    ],
    resource_deps=[],
    labels=['controllers'],
    allow_parallel=True,
)

# Build Docker image for IP Claim Controller
# Use custom_build to ensure binary exists before Docker build
# This matches the pattern from secret-manager-controller
BINARY_PATH = 'target/x86_64-unknown-linux-musl/release/ip-claim-controller'
IMAGE_NAME = 'ip-claim-controller'

custom_build(
    IMAGE_NAME,
    'docker build -f dockerfiles/Dockerfile.ip-claim-controller.dev -t %s:tilt . && docker tag %s:tilt $EXPECTED_REF && docker push $EXPECTED_REF' % (
        IMAGE_NAME,
        IMAGE_NAME
    ),
    deps=[
        BINARY_PATH,  # File dependency ensures binary exists before Docker build
        'dockerfiles/Dockerfile.ip-claim-controller.dev',
    ],
    resource_deps=['build-ip-claim-controller'],  # Wait for build to complete
    tag='tilt',
    live_update=[
        sync(BINARY_PATH, '/app/ip-claim-controller'),
        run('kill -HUP 1', trigger=[BINARY_PATH]),
    ],
)

# Deploy IP Claim Controller
k8s_yaml(kustomize('%s/config/ip-claim-controller' % DCops_DIR))

k8s_resource(
    'ip-claim-controller',
    labels=['controllers'],
    resource_deps=['build-ip-claim-controller'],  # Wait for binary to be built before deploying
)

# ====================
# Future Controllers
# ====================
# Additional controllers will be added here as they're implemented

