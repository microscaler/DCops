# DCops Development Container

This directory contains the configuration for a Dev Container that provides a complete development environment for the DCops project with Docker-in-Docker support. Compatible with **VS Code** and **JetBrains IDEs** (IntelliJ IDEA, RustRover).

## Features

- **Rust Development**: Full Rust toolchain with stable version, rustfmt, clippy, and musl target
- **Docker-in-Docker**: Full Docker support for building and running containers
- **Kubernetes Tools**: kubectl, kind, and Tilt pre-installed
- **Python Support**: Python 3 with pip for running project scripts
- **Development Tools**: Just command runner, cargo-nextest, cargo-llvm-cov, cargo-audit

## Prerequisites

**For VS Code:**
- [VS Code](https://code.visualstudio.com/) with the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)

**For JetBrains IDEs (IntelliJ IDEA, RustRover):**
- [IntelliJ IDEA](https://www.jetbrains.com/idea/) or [RustRover](https://www.jetbrains.com/rust/) (2023.2 or later)
- [Docker](https://www.docker.com/products/docker-desktop/) plugin installed in your IDE

**For All IDEs:**
- Docker Desktop (or Docker Engine) running on your host machine

## Usage

### VS Code

1. **Open in Dev Container**:
   - Open VS Code in the project root
   - Press `F1` or `Cmd+Shift+P` (Mac) / `Ctrl+Shift+P` (Windows/Linux)
   - Select "Dev Containers: Reopen in Container"
   - Wait for the container to build and start (first time may take several minutes)

2. **Verify Setup**:
   After the container starts, the post-create script will:
   - Verify all tools are installed
   - Pre-fetch Rust dependencies
   - Check Docker connectivity

### JetBrains IDEs (IntelliJ IDEA, RustRover)

1. **Open in Dev Container**:
   - Open your JetBrains IDE
   - Go to **File → New → Project from Dev Container**
   - Select the project directory (or open the project first, then use **File → Remote Development → Connect to Dev Container**)
   - The IDE will detect the `.devcontainer/devcontainer.json` file
   - Click **Connect** and wait for the container to build and start (first time may take several minutes)

2. **Alternative Method** (if the above doesn't work):
   - Open the project normally
   - Go to **File → Remote Development → Connect to Dev Container**
   - Select the `.devcontainer/devcontainer.json` configuration
   - Click **Connect**

3. **Verify Setup**:
   - Open the terminal in the IDE (View → Tool Windows → Terminal)
   - The post-create script should have run automatically
   - Verify tools: `rustc --version`, `docker --version`, `kubectl version --client`

4. **Configure Rust Toolchain** (if needed):
   - Go to **File → Settings** (or **Preferences** on Mac)
   - Navigate to **Languages & Frameworks → Rust**
   - Ensure the Rust toolchain is detected (should be automatic)
   - Install the Rust plugin if not already installed

### Start Development (All IDEs)

Once the container is running:

```bash
# Start Kind cluster and Tilt
just dev-up

# Or start Tilt only (if cluster is already running)
just tilt-up

# Build the project
just build

# Run tests
just test
```

## Port Forwarding

The following ports are configured for forwarding:
- **8001**: NetBox UI (via Tilt)
- **10350**: Tilt UI
- **5000**: Local Docker registry
- **5432**: PostgreSQL (optional)
- **6379**: Redis (optional)

**VS Code**: Ports are automatically forwarded and can be viewed in the "Ports" panel.

**JetBrains IDEs**: Port forwarding is handled automatically. You can view and manage forwarded ports in **View → Tool Windows → Services → Port Forwarding**.

## Docker-in-Docker

Docker-in-Docker is configured using the official Dev Container feature (`ghcr.io/devcontainers/features/docker-in-docker:2`). The Docker daemon runs inside the container, allowing you to:
- Build Docker images (including multi-platform with `docker buildx`)
- Run containers
- Use `docker compose` (v2)
- Run Kind clusters
- Build and push images to local registry

**Configuration**:
- Docker daemon starts automatically via the `postStartCommand` script
- Docker data is persisted in a named volume (`dind-var-lib-docker`)
- The `vscode` user has full Docker access (no sudo required)
- Docker BuildKit is enabled by default

**Note**: The Docker daemon is started automatically when the container starts. If you encounter Docker issues:
1. Check Docker status: `docker info`
2. Restart the container:
   - **VS Code**: "Dev Containers: Rebuild Container"
   - **JetBrains**: **File → Remote Development → Rebuild Dev Container**
3. Verify the daemon: `sudo service docker status`

## Troubleshooting

### Docker daemon not running
If Docker commands fail:
1. Check if the daemon is running: `docker info`
2. If not, restart it: `sudo service docker start`
3. Verify Docker is working: `docker run --rm hello-world`
4. If issues persist, rebuild the container:
   - **VS Code**: "Dev Containers: Rebuild Container"
   - **JetBrains**: **File → Remote Development → Rebuild Dev Container**

### Rust dependencies not found
Run:
```bash
cargo fetch
```

### Port conflicts
If ports are already in use, modify the `forwardPorts` section in `devcontainer.json`.

### Container build fails
- Ensure Docker Desktop is running
- Check Docker has enough resources allocated (recommended: 4GB RAM, 2 CPUs)
- Try rebuilding the container:
  - **VS Code**: "Dev Containers: Rebuild Container"
  - **JetBrains**: **File → Remote Development → Rebuild Dev Container**

## Customization

### Adding IDE Extensions

**VS Code**: Edit the `extensions` array in `devcontainer.json` under `customizations.vscode.extensions`.

**JetBrains**: Extensions are managed through the IDE's plugin system. Install plugins as you normally would - they will persist in the container.

### Modifying Tools
Edit the `Dockerfile` to add or modify installed tools.

### Environment Variables
Add to the `remoteEnv` section in `devcontainer.json`.

## Volume Mounts

- `.cargo`: Cached Rust cargo directory for faster builds
- Docker data: Persistent Docker-in-Docker volume

## Notes

- The container runs as the `vscode` user (non-root)
- Docker commands may require `sudo` in some cases (though the setup tries to avoid this)
- The Rust toolchain matches the project's `rust-toolchain.toml` configuration

## IDE-Specific Tips

### VS Code
- Use the integrated terminal for all commands
- Rust Analyzer extension is pre-configured
- Docker extension provides container management UI

### JetBrains IDEs (IntelliJ IDEA, RustRover)
- **Rust Plugin**: Install the Rust plugin if not already installed (usually auto-detected)
- **Terminal**: Use the built-in terminal (View → Tool Windows → Terminal)
- **Run Configurations**: Create run configurations for your Rust binaries as needed
- **Cargo Integration**: The IDE should automatically detect Cargo.toml and configure the project
- **Debugging**: Use the built-in debugger with Rust support
- **Code Completion**: Rust Analyzer runs automatically in the background

