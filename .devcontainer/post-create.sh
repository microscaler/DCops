#!/bin/bash
set -e

echo "🚀 Setting up DCops development environment..."

# Ensure cargo is in PATH
export PATH="/home/vscode/.cargo/bin:${PATH}"

# Verify Rust installation
echo "📦 Verifying Rust installation..."
rustc --version
cargo --version

# Verify tools
echo "🔧 Verifying tools..."
kubectl version --client --short
kind --version
tilt version
just --version
python3 --version

# Pre-fetch Rust dependencies (optional, speeds up first build)
echo "📥 Pre-fetching Rust dependencies..."
if [ -f "Cargo.toml" ]; then
    cargo fetch || echo "⚠️  Could not fetch dependencies (this is OK if Cargo.toml is not ready)"
fi

# Set up Python virtual environment if needed
if [ -f "requirements.txt" ]; then
    echo "🐍 Setting up Python virtual environment..."
    python3 -m venv .venv || echo "⚠️  Could not create venv (this is OK if not needed)"
fi

# Ensure Docker is accessible (Docker-in-Docker)
echo "🐳 Verifying Docker access..."
if command -v docker &> /dev/null; then
    # Wait for Docker daemon to be ready
    timeout=30
    while [ $timeout -gt 0 ]; do
        if docker info &> /dev/null; then
            echo "✅ Docker is ready"
            docker --version
            break
        fi
        echo "⏳ Waiting for Docker daemon... ($timeout seconds remaining)"
        sleep 1
        timeout=$((timeout - 1))
    done
    
    if [ $timeout -eq 0 ]; then
        echo "⚠️  Docker daemon not ready yet. You may need to restart the container."
    fi
else
    echo "⚠️  Docker command not found"
fi

echo "✅ Development environment setup complete!"
echo ""
echo "📚 Useful commands:"
echo "   just dev-up          - Start Kind cluster and Tilt"
echo "   just build           - Build Rust binaries and Docker images"
echo "   just test            - Run tests"
echo "   just lint            - Run linters"
echo "   tilt up              - Start Tilt UI"
echo ""

