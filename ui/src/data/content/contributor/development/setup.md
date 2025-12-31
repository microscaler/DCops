# Development Setup

Get started developing DCops with a complete development environment.

## Quick Start: Dev Container

The easiest way to get started is using the Dev Container, which provides a complete development environment with all dependencies pre-installed.

### Prerequisites

- **VS Code** with Dev Containers extension, or
- **JetBrains IDEs** (IntelliJ IDEA, RustRover) with Remote Development support
- **Docker Desktop** running

### Setup Steps

1. **Open Project**
   - Open the DCops project in your IDE
   - VS Code: Click "Reopen in Container" when prompted
   - JetBrains: Use Remote Development → Open in Container

2. **Wait for Container Build**
   - First time: Container will build (takes 5-10 minutes)
   - Subsequent: Container starts quickly

3. **Verify Setup**
   ```bash
   rustc --version
   cargo --version
   docker --version
   kubectl version --client
   ```

### What's Included

The Dev Container includes:
- ✅ Full Rust toolchain (stable, rustfmt, clippy, musl target)
- ✅ Docker-in-Docker for building images and running Kind clusters
- ✅ Kubernetes tools (kubectl, kind, Tilt)
- ✅ Python 3 for project scripts
- ✅ Development tools (Just, cargo-nextest, cargo-llvm-cov, cargo-audit)

📖 **See [`.devcontainer/README.md`](../../../../.devcontainer/README.md) for detailed setup instructions and usage guide.**

## Local Setup (Alternative)

If you prefer a local development environment:

### Prerequisites

- **Rust toolchain** - See `rust-toolchain.toml` for version
- **Docker Desktop** - For building images and running Kind clusters
- **kubectl** - Kubernetes CLI
- **kind** - Kubernetes in Docker
- **Tilt** - Development workflow tool
- **Python 3** - For running project scripts

### Installation

1. **Install Rust:**
   ```bash
   rustup install stable
   rustup target add x86_64-unknown-linux-musl
   ```

2. **Install Docker Desktop:**
   - macOS: Download from [docker.com](https://www.docker.com/products/docker-desktop)
   - Linux: Follow [Docker installation guide](https://docs.docker.com/engine/install/)

3. **Install Kubernetes Tools:**
   ```bash
   # kubectl
   curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/darwin/amd64/kubectl"
   sudo install -o root -g wheel -m 0755 kubectl /usr/local/bin/kubectl
   
   # kind
   curl -Lo ./kind https://kind.sigs.k8s.io/dl/v0.20.0/kind-darwin-amd64
   chmod +x ./kind
   sudo mv ./kind /usr/local/bin/kind
   
   # Tilt
   curl -fsSL https://raw.githubusercontent.com/tilt-dev/tilt/master/scripts/install.sh | bash
   ```

4. **Set Up Kind Cluster:**
   ```bash
   python3 scripts/setup_kind.py
   ```

## Development Workflow

### Start Development Environment

```bash
# Using Tilt (recommended)
tilt up

# Or manually
kubectl apply -f config/crd/all-crds.yaml
kubectl apply -k config/netbox-controller
```

### Run Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_netbox_site_reconciliation

# With coverage
cargo llvm-cov --html
```

### Build Controllers

```bash
# Debug build
cargo build -p netbox-controller

# Release build
cargo build --release -p netbox-controller

# Cross-compile for Linux
cargo build --target x86_64-unknown-linux-musl --release -p netbox-controller
```

### Generate CRDs

```bash
# Generate CRD YAML
cargo run -p crds --bin crdgen > config/crd/all-crds.yaml

# Or use script
python3 scripts/generate_crds.py
```

## Project Structure

```
DCops/
├── controllers/          # Kubernetes controllers
│   ├── netbox/          # NetBox controller
│   ├── pxe-intent/      # PXE controller (stub)
│   └── routeros/        # RouterOS controller (stub)
├── crates/              # Shared libraries
│   ├── crds/            # CRD definitions
│   ├── netbox-client/   # NetBox API client
│   └── ...
├── config/              # Kubernetes manifests
│   ├── crd/             # Generated CRDs
│   ├── examples/        # Example CRs
│   └── netbox-controller/ # Controller deployment
├── scripts/             # Python scripts
└── ui/                  # Documentation site
```

## Common Tasks

### Add a New CRD

1. Define CRD in `crates/crds/src/`
2. Generate CRD YAML: `python3 scripts/generate_crds.py`
3. Add reconciler in `controllers/netbox/src/reconciler/`
4. Add watcher in `controllers/netbox/src/main.rs`
5. Write tests
6. Add example CR in `config/examples/`

### Debug Controller

```bash
# View logs
kubectl logs -n dcops-system deployment/netbox-controller -f

# Check events
kubectl get events -n default --field-selector involvedObject.name=datacenter-tenant

# Describe resource
kubectl describe netboxsite datacenter-1
```

### Update Dependencies

```bash
# Update Rust dependencies
cargo update

# Audit for vulnerabilities
cargo audit

# Check for outdated crates
cargo outdated
```

## Next Steps

- [Architecture](./architecture.md) - Understand the system design
- [Testing](./testing.md) - Learn about testing practices
- [Contributing Guide](../contributing/contributing-guide.md) - Contribution guidelines
