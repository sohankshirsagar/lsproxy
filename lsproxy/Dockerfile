FROM rust:1.81.0-slim-bookworm

# Set the working directory in the container
WORKDIR /usr/src/app

# Install runtime and development dependencies, Python, Node.js, PHP, and npm
RUN apt-get update && apt-get install \
    -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libssl3 \
    ca-certificates \
    git \
    procps \
    python3 \
    python3-pip \
    python3-venv \
    curl \
    wget \
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

# Install rust-analyzer and rustfmt
RUN rustup component add rust-analyzer
RUN rustup component add rustfmt

# Install coverage tools
RUN cargo install cargo-llvm-cov
RUN rustup component add llvm-tools-preview

# Install jedi python language server, ast grep for tree sitter
RUN pip install jedi-language-server ast-grep-cli

# Install global npm packages
RUN npm install -g typescript-language-server typescript

# Install go and Gopls
ARG TARGETARCH
RUN curl -O -L "https://go.dev/dl/go1.23.5.linux-${TARGETARCH}.tar.gz" && \
    tar -C /usr/local -xzf go1.23.5.linux-${TARGETARCH}.tar.gz && \
    rm go1.23.5.linux-${TARGETARCH}.tar.gz && \
    /usr/local/go/bin/go install golang.org/x/tools/gopls@latest && \
    cp ~/go/bin/gopls /usr/bin/gopls

ENV GOROOT=/usr/local/go
ENV GOPATH=/home/user/go
ENV PATH=$GOPATH/bin:$GOROOT/bin:$PATH

# Install ruby and ruby-lsp
RUN apt update && apt install -y ruby-full \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN gem install ruby-lsp

ENV HOME=/home/user

# Download and run dotnet install script
RUN curl -fsSL https://builds.dotnet.microsoft.com/dotnet/scripts/v1/dotnet-install.sh -o dotnet-install.sh \
    && chmod +x dotnet-install.sh \
    && ./dotnet-install.sh --channel 8.0 --install-dir /home/user/.dotnet \
    && ./dotnet-install.sh --channel 9.0 --install-dir /home/user/.dotnet \
    && rm dotnet-install.sh

# Add .NET to PATH
ENV PATH="${PATH}:/home/user/.dotnet"
ENV DOTNET_ROOT=/home/user/.dotnet

# Install csharp-ls globally
RUN dotnet tool install --global csharp-ls

# Add .NET tools to PATH
ENV PATH="${PATH}:/home/user/.dotnet/tools"


COPY Cargo.toml .

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN touch src/lib.rs
RUN cargo build --release

RUN cargo install ast-grep --locked
COPY ./src/ast_grep /usr/src/ast_grep

# Document that the container listens on port 4444
EXPOSE 4444

# Set environment variables for logging and backtrace
ENV RUST_LOG=debug,glob=info,ignore=info
ENV RUST_BACKTRACE=1
ENV RA_LOG="/tmp/rust-analyzer.log"

CMD /bin/bash -c "touch Cargo.toml && cargo build --release"
