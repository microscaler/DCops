# DCops Development Container
# Based on Ubuntu with Rust, Kubernetes tools, and Docker-in-Docker support

FROM mcr.microsoft.com/devcontainers/base:1-ubuntu

# Avoid warnings by switching to noninteractive
ENV DEBIAN_FRONTEND=noninteractive

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    musl-tools \
    ca-certificates \
    gnupg \
    lsb-release \
    python3 \
    python3-pip \
    python3-venv \
    && rm -rf /var/lib/apt/lists/*

# Install Rust using rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && . "$HOME/.cargo/env" \
    && rustup toolchain install stable \
    && rustup default stable \
    && rustup component add rustfmt clippy \
    && rustup target add x86_64-unknown-linux-musl

# Install kubectl
RUN curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl" \
    && chmod +x kubectl \
    && mv kubectl /usr/local/bin/

# Install Kind
RUN curl -Lo ./kind https://kind.sigs.k8s.io/dl/latest/kind-linux-amd64 \
    && chmod +x ./kind \
    && mv ./kind /usr/local/bin/kind

# Install Tilt
RUN curl -fsSL https://raw.githubusercontent.com/tilt-dev/tilt/master/scripts/install.sh | bash

# Install Just command runner
RUN curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to /usr/local/bin

# Install cargo tools
RUN . "$HOME/.cargo/env" && cargo install --locked \
    cargo-nextest \
    cargo-llvm-cov \
    cargo-audit

# Configure git
RUN git config --global --add safe.directory '*' \
    && git config --global init.defaultBranch main

# Set up environment variables
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
ENV RUST_BACKTRACE=1
ENV DOCKER_BUILDKIT=1
ENV PATH="/home/vscode/.cargo/bin:/home/vscode/.local/bin:${PATH}"

# Switch back to dialog for any ad-hoc use of apt-get
ENV DEBIAN_FRONTEND=dialog

# Note: Docker-in-Docker will be set up by the devcontainer feature
# The docker daemon will be started via postStartCommand

