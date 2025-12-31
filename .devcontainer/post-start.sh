#!/bin/bash
set -e

echo "🐳 Starting Docker daemon..."

# The Docker-in-Docker feature should handle this, but we ensure it's running
# Check if dockerd is already running
if ! pgrep -x dockerd > /dev/null; then
    echo "Starting Docker daemon..."
    sudo service docker start || true
fi

# Wait for Docker daemon to be ready
timeout=30
while [ $timeout -gt 0 ]; do
    if docker info &> /dev/null; then
        echo "✅ Docker daemon is ready"
        docker --version
        docker info --format "Docker version: {{.ServerVersion}}"
        break
    fi
    echo "⏳ Waiting for Docker daemon... ($timeout seconds remaining)"
    sleep 1
    timeout=$((timeout - 1))
done

if [ $timeout -eq 0 ]; then
    echo "⚠️  Docker daemon did not start. You may need to restart the container."
    exit 1
fi

# Verify Docker is working
echo "🧪 Testing Docker..."
docker run --rm hello-world > /dev/null 2>&1 && echo "✅ Docker is working correctly" || echo "⚠️  Docker test failed"

