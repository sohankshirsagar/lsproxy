FROM --platform=linux/$BUILDARCH rust:1.81.0-slim-bookworm AS builder
ARG BUILDARCH
ARG TARGETPLATFORM

WORKDIR /usr/src/app

RUN apt-get update && apt install -y --no-install-recommends curl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Set up cross-compilation tools and target based on build/target platform
RUN apt-get update && \
    case "$TARGETPLATFORM" in \
        "linux/amd64") \
            if [ "$BUILDARCH" = "arm64" ]; then \
                rustup target add x86_64-unknown-linux-gnu && \
                apt-get install -y gcc-x86-64-linux-gnu && \
                echo '[target.x86_64-unknown-linux-gnu]' > /usr/local/cargo/config.toml && \
                echo 'linker = "x86_64-linux-gnu-gcc"' >> /usr/local/cargo/config.toml; \
            elif [ "$BUILDARCH" != "amd64" ]; then \
                echo "Unsupported build architecture for linux/amd64: $BUILDARCH" && exit 1; \
            fi \
            ;; \
        "linux/arm64") \
            if [ "$BUILDARCH" != "amd64" ]; then \
                rustup target add aarch64-unknown-linux-gnu && \
                apt-get install -y gcc-aarch64-linux-gnu && \
                echo '[target.aarch64-unknown-linux-gnu]' > /usr/local/cargo/config.toml && \
                echo 'linker = "aarch64-linux-gnu-gcc"' >> /usr/local/cargo/config.toml; \
            elif [ "$BUILDARCH" != "arm64" ]; then \
                echo "Unsupported build architecture for linux/arm64: $BUILDARCH" && exit 1; \
            fi \
            ;; \
        *) \
            echo "Unsupported target platform: $TARGETPLATFORM (BUILDARCH: $BUILDARCH)" && exit 1 \
            ;; \
    esac && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY . .

# Build with appropriate target
RUN case "$TARGETPLATFORM" in \
        "linux/amd64") \
            if [ "$BUILDARCH" = "arm64" ]; then \
                cargo build --release --target x86_64-unknown-linux-gnu; \
            elif [ "$BUILDARCH" = "amd64" ]; then \
                cargo build --release; \
            else \
                echo "Unsupported build architecture for linux/amd64: $BUILDARCH" && exit 1; \
            fi \
            ;; \
        "linux/arm64") \
            if [ "$BUILDARCH" != "amd64" ]; then \
                cargo build --release --target aarch64-unknown-linux-gnu; \
            elif [ "$BUILDARCH" = "arm64" ]; then \
                cargo build --release; \
            else \
                echo "Unsupported build architecture for linux/arm64: $BUILDARCH" && exit 1; \
            fi \
            ;; \
        *) \
            echo "Unsupported target platform: $TARGETPLATFORM (BUILDARCH: $BUILDARCH)" && exit 1 \
            ;; \
    esac

FROM rust:1.81.0-slim-bookworm

# Install runtime dependencies, Python, Node.js, PHP, and npm
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
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
    g++ \
    php \
    php-xml \
    php-mbstring \
    php-curl \
    php-zip \
    unzip \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install Composer
RUN curl -sS https://getcomposer.org/installer | php -- --install-dir=/usr/local/bin --filename=composer

# Install PHPActor
RUN cd /usr/src && \
    git clone https://github.com/phpactor/phpactor.git && \
    cd /usr/src/phpactor && \
    composer install

ENV PATH="/usr/src/phpactor/bin:${PATH}"

# Config Python
RUN ln -sf /usr/bin/python3 /usr/bin/python
RUN rm /usr/lib/python3.11/EXTERNALLY-MANAGED

# Install Java
RUN apt-get update && \
    apt-get install \
    -y --no-install-recommends \
    openjdk-17-jdk \
    gradle \
    maven \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* \
    && \
    curl -L -o /tmp/jdt-language-server.tar.gz https://www.eclipse.org/downloads/download.php?file=/jdtls/snapshots/jdt-language-server-latest.tar.gz && \
    mkdir -p /opt/jdtls && \
    tar -xzf /tmp/jdt-language-server.tar.gz -C /opt/jdtls --no-same-owner && \
    rm /tmp/jdt-language-server.tar.gz

# Add jdtls to PATH
ENV PATH="/opt/jdtls/bin:${PATH}"

# Install rust-analyzer
RUN rustup component add rust-analyzer
RUN rustup component add rustfmt

# Install jedi python language server, ast grep for tree sitter
RUN pip install jedi-language-server ast-grep-cli

# Install global npm packages
RUN npm install -g typescript-language-server typescript

# Install go and Gopls
ARG TARGETARCH
RUN curl -O -L "https://go.dev/dl/go1.21.4.linux-${TARGETARCH}.tar.gz" && \
    tar -C /usr/local -xzf go1.21.4.linux-${TARGETARCH}.tar.gz && \
    rm go1.21.4.linux-${TARGETARCH}.tar.gz && \
    /usr/local/go/bin/go install golang.org/x/tools/gopls@latest && \
    cp ~/go/bin/gopls /usr/local/bin/gopls

# Install ruby and ruby-lsp
RUN apt update && apt install -y ruby-full \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN gem install ruby-lsp

ENV HOME=/home/user
ENV GOROOT=/usr/local/go
ENV GOPATH=/home/user/go
ENV PATH=$GOPATH/bin:$GOROOT/bin:$PATH

# Copy binary with -bin suffix
COPY --from=builder /usr/src/app/target/release/lsproxy /usr/local/bin/lsproxy-bin

# Copy ast-grep rules
COPY ./src/ast_grep /usr/src/ast_grep

EXPOSE 4444
ENV RUST_LOG=info
ENV RA_LOG="/tmp/rust-analyzer.log"

RUN mkdir -p /mnt/workspace /usr/local/cargo /usr/src/app /usr/local/go /home/user/go \
    && chown -R 1000:1000 /mnt/workspace /usr/local/cargo /usr/src/app /usr/local/go /home/user /usr/local/bin

# Create the wrapper script with all current env vars
RUN echo '#!/bin/bash' > /usr/local/bin/lsproxy \
    && env | while read -r line; do \
    echo "export $line" >> /usr/local/bin/lsproxy; \
    done \
    && echo 'exec /usr/local/bin/lsproxy-bin "$@"' >> /usr/local/bin/lsproxy \
    && chmod +x /usr/local/bin/lsproxy

CMD ["lsproxy"]
