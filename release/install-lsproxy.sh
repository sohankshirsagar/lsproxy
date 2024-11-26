#!/bin/bash
set -e

LSPROXY_VERSION="0.1.7"

# Function to detect architecture
detect_arch() {
    local arch=$(uname -m)
    case $arch in
        x86_64)
            echo "amd64"
            ;;
        aarch64|arm64)
            echo "arm64"
            ;;
        *)
            echo "Unsupported architecture: $arch"
            exit 1
            ;;
    esac
}

# Check if running as root
check_root() {
    if [ "$(id -u)" -ne 0 ]; then
        echo "Error: This script must be run as root"
        exit 1
    fi
}

# Function to install system dependencies
install_system_deps() {
    echo "Installing system dependencies..."
    apt-get update
    DEBIAN_FRONTEND=noninteractive apt-get install -y \
        pkg-config \
        libssl3 \
        ca-certificates \
        git \
        python3 \
        python3-pip \
        python3-venv \
        curl \
        clangd \
        build-essential \
        gcc \
        g++
}

# Function to install Node.js
install_nodejs() {
    echo "Installing Node.js..."
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
    apt-get install -y nodejs
}

# Function to install Java and JDT
install_java() {
    echo "Installing Java and JDT..."
    DEBIAN_FRONTEND=noninteractive apt-get install -y openjdk-17-jdk gradle maven
    curl -L -o /tmp/jdt-language-server.tar.gz \
        "https://www.eclipse.org/downloads/download.php?file=/jdtls/snapshots/jdt-language-server-1.42.0-202410312059.tar.gz"
    mkdir -p /opt/jdtls
    tar -xzf /tmp/jdt-language-server.tar.gz -C /opt/jdtls
    rm /tmp/jdt-language-server.tar.gz
    
    # Add jdtls to PATH
    echo 'export PATH="/opt/jdtls/bin:${PATH}"' >> /etc/profile.d/jdtls.sh
}

# Function to install Python dependencies
install_python_deps() {
    echo "Installing Python dependencies..."
    python3 -m venv /app/venv
    # Add venv to PATH
    echo 'export PATH="/app/venv/bin:${PATH}"' >> /etc/profile.d/venv.sh
    source /app/venv/bin/activate
    pip install jedi-language-server ast-grep-cli
}

# Function to install Node.js dependencies
install_node_deps() {
    echo "Installing Node.js dependencies..."
    npm install -g typescript-language-server typescript
}

# Function to install Rust tooling
install_rust_tools() {
    echo "Installing Rust analysis tools..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
    rustup component add rust-analyzer
    rustup component add rustfmt
}

# Function to download and install LSProxy binary
install_lsproxy() {
    local arch=$(detect_arch)
    local binary_url="https://github.com/agentic-labs/lsproxy/releases/download/${LSPROXY_VERSION}/lsproxy-${LSPROXY_VERSION}-linux-${arch}"
    echo $binary_url
    
    echo "Downloading LSProxy binary for Linux ${arch}..."
    curl -L -o /usr/local/bin/lsproxy "${binary_url}"
    chmod +x /usr/local/bin/lsproxy
}

# Function to clean up
cleanup() {
    echo "Cleaning up..."
    apt-get clean
    rm -rf /var/lib/apt/lists/*
}

# Main installation
main() {
    echo "Installing LSProxy version ${LSPROXY_VERSION} for Linux..."
    check_root
    
    install_system_deps
    install_nodejs
    install_java
    install_python_deps
    install_node_deps
    install_rust_tools
    install_lsproxy
    cleanup
    
    echo "LSProxy installation complete!"
    echo "Please restart your shell or source your profile to use the updated PATH"
}

main
