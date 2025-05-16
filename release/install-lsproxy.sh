#!/bin/bash
set -e

LSPROXY_VERSION="0.1.16"

# Initialize variables
TARGET_USER=""
TARGET_UID=""

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --user)
            if [ -z "$2" ] || [[ "$2" == --* ]]; then
                echo "Error: --user requires a value"
                exit 1
            fi
            TARGET_USER="$2"
            shift 2
            ;;
        --uid)
            if [ -z "$2" ] || [[ "$2" == --* ]]; then
                echo "Error: --uid requires a value"
                exit 1
            fi
            TARGET_UID="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [--user USERNAME] [--uid UID]"
            echo "Install LSProxy and its dependencies"
            echo ""
            echo "Options:"
            echo "  --user USERNAME    Specify an existing user, or prepare directories for a future user when used with --uid"
            echo "  --uid UID         Specify the UID for directory preparation (must be used with --user)"
            echo ""
            echo "Note: When both --user and --uid are provided, this script will prepare directories"
            echo "      for that user/uid combination but will NOT create the user account itself."
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Validate and setup user configuration
setup_user() {
    local user_home

    # Case 1: No user or UID specified - use root
    if [ -z "$TARGET_USER" ] && [ -z "$TARGET_UID" ]; then
        TARGET_USER="root"
        user_home="/root"
        echo "No user specified, running as root"

    # Case 2: Only UID specified - error
    elif [ -z "$TARGET_USER" ] && [ -n "$TARGET_UID" ]; then
        echo "Error: --uid must be used with --user"
        exit 1

    # Case 3: Only username specified - must exist
    elif [ -n "$TARGET_USER" ] && [ -z "$TARGET_UID" ]; then
        if ! id "$TARGET_USER" >/dev/null 2>&1; then
            echo "Error: User $TARGET_USER does not exist"
            exit 1
        fi
        user_home=$(eval echo ~$TARGET_USER)
        echo "Using existing user: $TARGET_USER"

    # Case 4: Both username and UID specified
    else
        # Check if user exists
        if id "$TARGET_USER" >/dev/null 2>&1; then
            # User exists - verify UID matches
            existing_uid=$(id -u "$TARGET_USER")
            if [ "$existing_uid" != "$TARGET_UID" ]; then
                echo "Error: User $TARGET_USER exists but has UID $existing_uid (not $TARGET_UID)"
                exit 1
            fi
            user_home=$(eval echo ~$TARGET_USER)
            echo "Using existing user: $TARGET_USER with UID: $TARGET_UID"
        else
            # User doesn't exist - we'll create the directories expecting it
            user_home="/home/$TARGET_USER"
            echo "Preparing for user: $TARGET_USER with UID: $TARGET_UID"
        fi
    fi

    # Create and set up directories
    mkdir -p "$user_home"/{.local,.cargo,go}

    # If we have a UID, set ownership
    if [ -n "$TARGET_UID" ]; then
        chown -R "${TARGET_UID}:${TARGET_UID}" "$user_home"
    elif [ "$TARGET_USER" != "root" ]; then
        chown -R "${TARGET_USER}:${TARGET_USER}" "$user_home"
    fi

    # Export variables for use in other functions
    export LSPROXY_USER_HOME="$user_home"
    export LSPROXY_TARGET_USER="$TARGET_USER"
    export LSPROXY_TARGET_UID="$TARGET_UID"
}


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
        curl \
        clangd \
        build-essential \
        gcc \
        g++
}

# Function to install python
install_python() {
   echo "Installing python and packages..."
   DEBIAN_FRONTEND=noninteractive apt-get install -y \
       python3 \
       python3-pip

   ln -sf /usr/bin/python3 /usr/bin/python

   PY_VERSION=$(python --version | cut -d' ' -f2 | cut -d'.' -f1,2)
   MANAGED_FILE="/usr/lib/python${PY_VERSION}/EXTERNALLY-MANAGED"

   if [ -f "$MANAGED_FILE" ]; then
       rm "$MANAGED_FILE"
   else
       echo "Warning: EXTERNALLY-MANAGED file not found at $MANAGED_FILE"
   fi

   pip3 install jedi-language-server ast-grep-cli
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
        "https://www.eclipse.org/downloads/download.php?file=/jdtls/snapshots/jdt-language-server-latest.tar.gz"
    mkdir -p /opt/jdtls
    tar -xzf /tmp/jdt-language-server.tar.gz -C /opt/jdtls --no-same-owner
    rm /tmp/jdt-language-server.tar.gz

    # Add jdtls to PATH
    echo 'export PATH="/opt/jdtls/bin:${PATH}"' >> /etc/profile.d/jdtls.sh
}

# Function to install PHP and related tools
install_php() {
    echo "Installing PHP and related tools..."
    DEBIAN_FRONTEND=noninteractive apt-get install -y \
        php \
        php-xml \
        php-mbstring \
        php-curl \
        php-zip \
        unzip

    # Install Composer
    curl -sS https://getcomposer.org/installer | php -- --install-dir=/usr/local/bin --filename=composer

    # Install PHPActor from source
    cd /usr/src && \
    git clone https://github.com/phpactor/phpactor.git && \
    cd /usr/src/phpactor && \
    composer install

    # Add phpactor to PATH
    echo 'export PATH="/usr/src/phpactor/bin:${PATH}"' >> /etc/profile.d/phpactor.sh
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
    . $HOME/.cargo/env
    rustup component add rust-analyzer
    rustup component add rustfmt
}

# Function to install Go and Gopls
install_go() {
    echo "Installing Go and Gopls..."
    local arch=$(detect_arch)
    curl -O -L "https://go.dev/dl/go1.21.4.linux-${arch}.tar.gz"
    tar -C /usr/local -xzf go1.21.4.linux-${arch}.tar.gz
    rm go1.21.4.linux-${arch}.tar.gz

    # Install Gopls
    /usr/local/go/bin/go install golang.org/x/tools/gopls@latest
    cp ~/go/bin/gopls /usr/local/bin/gopls

    # Set up Go environment
    export GOROOT=/usr/local/go
    export GOPATH=/home/user/go
    export PATH=$GOPATH/bin:$GOROOT/bin:$PATH

    # Add Go environment to profile
    echo 'export GOROOT=/usr/local/go' >> /etc/profile.d/go.sh
    echo 'export GOPATH=/home/user/go' >> /etc/profile.d/go.sh
    echo 'export PATH=$GOPATH/bin:$GOROOT/bin:$PATH' >> /etc/profile.d/go.sh
}

# Function to install Ruby and Ruby LSP
install_ruby() {
    echo "Installing Ruby and Ruby LSP..."
    DEBIAN_FRONTEND=noninteractive apt-get install -y ruby-full
    gem install ruby-lsp
}

# Function to install .NET and C# language server
install_dotnet() {
    echo "Installing .NET and C# language server..."
    local dotnet_dir="$LSPROXY_USER_HOME/.dotnet"

    curl -fsSL https://builds.dotnet.microsoft.com/dotnet/scripts/v1/dotnet-install.sh -o dotnet-install.sh
    chmod +x dotnet-install.sh

    if [ "$LSPROXY_TARGET_USER" != "root" ]; then
        # Install as target user if it exists
        if id "$LSPROXY_TARGET_USER" >/dev/null 2>&1; then
            su - "$LSPROXY_TARGET_USER" -c "
                ./dotnet-install.sh --channel 8.0 --install-dir $dotnet_dir
                ./dotnet-install.sh --channel 9.0 --install-dir $dotnet_dir
                export DOTNET_ROOT=$dotnet_dir
                export PATH=\$PATH:$dotnet_dir:$dotnet_dir/tools
                dotnet tool install --global csharp-ls
            "
        else
            # Just install the files, user will be created later
            ./dotnet-install.sh --channel 8.0 --install-dir "$dotnet_dir"
            ./dotnet-install.sh --channel 9.0 --install-dir "$dotnet_dir"
        fi
    else
        # Install as root
        ./dotnet-install.sh --channel 8.0 --install-dir "$dotnet_dir"
        ./dotnet-install.sh --channel 9.0 --install-dir "$dotnet_dir"
        export DOTNET_ROOT="$dotnet_dir"
        export PATH="$PATH:$dotnet_dir:$dotnet_dir/tools"
        dotnet tool install --global csharp-ls
    fi

    rm dotnet-install.sh
}

# Function to download and install LSProxy binary
install_lsproxy() {
    local arch=$(detect_arch)
    local binary_url="https://github.com/agentic-labs/lsproxy/releases/download/${LSPROXY_VERSION}/lsproxy-${LSPROXY_VERSION}-linux-${arch}"

    echo "Downloading LSProxy binary for Linux ${arch}..."
    curl -L -o /usr/local/bin/lsproxy "${binary_url}"
    chmod +x /usr/local/bin/lsproxy
}

# Function to install ast_grep configuration
install_ast_grep_config() {
    local config_url="https://github.com/agentic-labs/lsproxy/releases/download/${LSPROXY_VERSION}/lsproxy-${LSPROXY_VERSION}-ast-grep-rules.tar.gz"
    local dest_dir="/usr/src"

    echo "Downloading ast_grep configuration and rules..."
    mkdir -p "$dest_dir"
    curl -L -o /tmp/ast_grep.tar.gz "$config_url"

    echo "Extracting ast_grep configuration and rules..."
    tar -xzf /tmp/ast_grep.tar.gz -C "$dest_dir" --no-same-owner
    rm /tmp/ast_grep.tar.gz
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
    # Capture the initial environment before any changes
    OLD_ENV=$(env)
    check_root
    setup_user
    install_system_deps
    install_python
    install_nodejs
    install_java
    install_php
    install_node_deps
    install_rust_tools
    install_go
    install_ruby
    install_dotnet
    install_lsproxy
    install_ast_grep_config
    cleanup

    # Capture the new environment and write out only differences dynamically
    NEW_ENV=$(env)
    ENV_FILE="/etc/profile.d/lsproxy-env.sh"
    comm -13 <(echo "$OLD_ENV" | sort) <(echo "$NEW_ENV" | sort) | sed 's/^/export /' > "$ENV_FILE"
    chmod 644 "$ENV_FILE"

    echo "LSProxy installation complete!"
    echo ""
    echo "To apply the new environment settings immediately, run:"
    echo "  source /etc/profile.d/lsproxy-env.sh"
    echo "Alternatively, log out and log back in."
}

main
