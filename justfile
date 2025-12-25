#!/usr/bin/env just --justfile
# DCops Development Justfile

# Set shell for recipes
set shell := ["bash", "-uc"]

# Default recipe to display help
default:
    @just --list --unsorted

# ============================================================================
# Development Environment
# ============================================================================

# Start development environment (Kind + Tilt)
dev-up:
    python3 scripts/dev_up.py

# Stop development environment (Kind + Tilt)
dev-down:
    python3 scripts/dev_down.py

# Start Tilt only (assumes cluster is already running)
tilt-up:
    @echo "🎯 Starting Tilt..."
    @echo "   Tilt UI: http://localhost:10350"
    @echo "   NetBox UI: http://localhost:8001 (via Tilt port forward)"
    @tilt up

# Stop Tilt only
tilt-down:
    @echo "🛑 Stopping Tilt..."
    @tilt down

# ============================================================================
# Building
# ============================================================================

# Build all (Rust binary + Docker image)
build: build-rust build-docker

# Build Rust binary (debug)
build-rust:
    @echo "🔨 Building Rust binary..."
    @cargo build --workspace

# Build Rust binary (release)
build-release:
    @echo "🔨 Building Rust binary (release)..."
    @cargo build --workspace --release

# Build Rust binary for musl target (cross-compilation)
build-musl:
    @echo "🔨 Building Rust binary for musl target..."
    @python3 scripts/host_aware_build.py --release

# Build Docker image (development)
build-docker:
    @echo "🐳 Building Docker images (development)..."
    @docker build -f dockerfiles/Dockerfile.pxe-intent-controller.dev -t localhost:5000/dcops-pxe-intent-controller:dev .
    @docker build -f dockerfiles/Dockerfile.ip-claim-controller.dev -t localhost:5000/dcops-ip-claim-controller:dev .

# Build Docker image (production)
build-docker-prod:
    @echo "🐳 Building Docker images (production)..."
    @docker buildx build -f dockerfiles/Dockerfile.pxe-intent-controller -t localhost:5000/dcops-pxe-intent-controller:latest .
    @docker buildx build -f dockerfiles/Dockerfile.ip-claim-controller -t localhost:5000/dcops-ip-claim-controller:latest .

# Build base images
build-base:
    @echo "🐳 Building base Docker images..."
    @docker buildx build -f dockerfiles/Dockerfile.base.rust-builder -t localhost:5000/dcops-rust-builder-base-image:latest .
    @docker buildx build -f dockerfiles/Dockerfile.base.controller -t localhost:5000/dcops-controller-base-image:latest .

# ============================================================================
# Testing
# ============================================================================

# Run all tests
test: test-unit

# Run unit tests
test-unit:
    @echo "🧪 Running unit tests..."
    @cargo test --workspace --lib --no-fail-fast

# Run unit tests with output
test-unit-verbose:
    @echo "🧪 Running unit tests (verbose)..."
    @cargo test --workspace --lib -- --nocapture --no-fail-fast

# Run tests with LLVM coverage
test-coverage:
    @echo "🧪 Running tests with LLVM coverage..."
    @echo "📦 Installing cargo-llvm-cov if needed..."
    @cargo install cargo-llvm-cov --locked 2>/dev/null || true
    @echo "🔍 Generating coverage report..."
    @cargo llvm-cov --workspace --lib --lcov --output-path lcov.info
    @cargo llvm-cov --workspace --lib --html --output-dir target/llvm-cov/html
    @echo "✅ Coverage report generated:"
    @echo "   📊 HTML: target/llvm-cov/html/index.html"
    @echo "   📄 LCOV: lcov.info"
    @cargo llvm-cov --workspace --lib --summary-only

# Open coverage report in browser
test-coverage-open:
    @echo "🌐 Opening coverage report..."
    @open target/llvm-cov/html/index.html || xdg-open target/llvm-cov/html/index.html || echo "Please open target/llvm-cov/html/index.html manually"

# ============================================================================
# Code Quality
# ============================================================================

# Format code
fmt:
    @echo "🎨 Formatting code..."
    @cargo fmt

# Check formatting
fmt-check:
    @echo "🎨 Checking code formatting..."
    @cargo fmt -- --check

# Lint code
lint:
    @echo "🔍 Linting code..."
    @cargo clippy --workspace -- -D warnings

# Lint and fix
lint-fix:
    @echo "🔍 Linting and fixing code..."
    @cargo clippy --fix --allow-dirty --allow-staged

# Audit dependencies
audit:
    @echo "🔒 Auditing dependencies..."
    @cargo audit

# Check code (compile without building)
check:
    @echo "✅ Checking code..."
    @cargo check --workspace --all-targets

# Validate all (format, lint, check, tests, coverage)
validate: fmt-check lint check test-unit test-coverage-check
    @echo "✅ All validations passed!"

# Check coverage meets minimum (65%)
test-coverage-check:
    @echo "📊 Checking test coverage (minimum 65%)..."
    @cargo install cargo-llvm-cov --locked 2>/dev/null || true
    @cargo llvm-cov --workspace --lib --summary-only | grep -E "^\s*Total\s+\|\s+[0-9]+\s+\|\s+[0-9]+\s+\|\s+([0-9]+)%" || echo "⚠️  Could not parse coverage, run 'just test-coverage' for full report"

# ============================================================================
# Deployment
# ============================================================================

# Deploy to Kubernetes (using kustomize)
deploy:
    @echo "🚀 Deploying to Kubernetes..."
    @kubectl apply -k config/
    @echo "✅ Deployed to microscaler-system namespace"

# Deploy CRDs only
deploy-crd:
    @echo "📝 Deploying CRDs..."
    @kubectl apply -f config/crd/
    @echo "✅ CRDs deployed"

# Undeploy from Kubernetes
undeploy:
    python3 scripts/undeploy.py

# ============================================================================
# Utilities
# ============================================================================

# Clean build artifacts
clean:
    @echo "🧹 Cleaning build artifacts..."
    @cargo clean
    @echo "✅ Cleaned"

# Show cluster and controller status
status:
    python3 scripts/status.py

# Show PXE Intent Controller logs
logs-pxe:
    @echo "📜 PXE Intent Controller logs..."
    @kubectl logs -n microscaler-system -l app=pxe-intent-controller --tail=100 -f

# Show IP Claim Controller logs
logs-ip:
    @echo "📜 IP Claim Controller logs..."
    @kubectl logs -n microscaler-system -l app=ip-claim-controller --tail=100 -f

# Show all controller logs
logs-all:
    @echo "📜 All controller logs..."
    @kubectl logs -n microscaler-system -l app.kubernetes.io/part-of=dcops --tail=100 -f --all-containers=true

# Port forward to PXE Intent Controller metrics
port-forward-pxe:
    @echo "🔌 Port forwarding to PXE Intent Controller metrics (5000)..."
    @kubectl port-forward -n microscaler-system svc/pxe-intent-controller-metrics 5000:5000

# Port forward to IP Claim Controller metrics
port-forward-ip:
    @echo "🔌 Port forwarding to IP Claim Controller metrics (5001)..."
    @kubectl port-forward -n microscaler-system svc/ip-claim-controller-metrics 5001:5000

# ============================================================================
# NetBox Management
# ============================================================================

# Deploy NetBox to Kind cluster
deploy-netbox:
    @echo "🚀 Deploying NetBox..."
    @python3 scripts/deploy_netbox.py

# Undeploy NetBox from Kind cluster
undeploy-netbox:
    @echo "🛑 Undeploying NetBox..."
    @python3 scripts/undeploy_netbox.py

# Show NetBox deployment status
netbox-status:
    @echo "📊 NetBox Status..."
    @python3 scripts/netbox_status.py

# Port forward to NetBox
port-forward-netbox:
    @echo "🔌 Port forwarding to NetBox (8000)..."
    @kubectl port-forward -n netbox svc/netbox 8000:80

# Manage NetBox API token for IP Claim Controller
# Usage: just netbox-token
#   Or with token: NETBOX_TOKEN=abc123 just netbox-token
#   Or with URL: NETBOX_URL=http://localhost:8001 just netbox-token
netbox-token:
    @echo "🔑 Managing NetBox API token..."
    @python3 scripts/manage_netbox_token.py

# Create a NetBox prefix
# Usage: NETBOX_TOKEN=<token> just netbox-create-prefix --prefix 192.168.1.0/24
netbox-create-prefix prefix:
    @echo "📝 Creating NetBox prefix..."
    @python3 scripts/create_netbox_prefix.py --token $${NETBOX_TOKEN} --prefix $(prefix)

# Verify NetBox CR reconciliation
# Usage: just verify-netbox-crs
#   Or verify specific CRD: just verify-netbox-crs --crd netboxprefixes
#   Or verify specific CR: just verify-netbox-crs --crd netboxprefixes --name control-plane-prefix
verify-netbox-crs:
    @echo "🔍 Verifying NetBox CR reconciliation..."
    @python3 scripts/verify_netbox_crs.py --all

# ============================================================================
# Dependencies & Tools
# ============================================================================

# Check prerequisites
check-deps:
    python3 scripts/check_deps.py

# ============================================================================
# Documentation
# ============================================================================

# Generate documentation
docs:
    @echo "📚 Generating documentation..."
    @cargo doc --no-deps --open

# Generate documentation (without opening)
docs-build:
    @echo "📚 Building documentation..."
    @cargo doc --no-deps
    @echo "✅ Documentation built: target/doc/"

