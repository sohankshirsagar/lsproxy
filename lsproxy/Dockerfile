FROM rust:1.81.0-slim-bookworm

# Set the working directory in the container
WORKDIR /usr/src/app

# Install runtime dependencies, Python, Node.js, and npm
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libssl3 \
    ca-certificates \
    git \
    python3 \
    python3-pip \
    pipx \
    curl \
    build-essential \
    gcc \
    g++ \
    && curl -fsSL https://deb.nodesource.com/setup_16.x | bash - \
    && apt-get install -y nodejs \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install rust-analyzer
RUN rustup component add rust-analyzer
RUN rustup component add rustfmt

# Install global npm packages
RUN npm install -g pyright typescript-language-server typescript

#Install Gopls
WORKDIR /tmp
RUN curl -O -L "https://go.dev/dl/go1.23.3.linux-arm64.tar.gz"
RUN tar -C /usr/local -xzf go1.23.3.linux-arm64.tar.gz
RUN rm go1.23.3.linux-amd64.tar.gz
RUN /usr/local/go/bin/go install golang.org/x/tools/gopls@latest
RUN cp ~/go/bin/gopls /usr/local/bin/gopls

#Install Clangd
RUN apt update && apt install -y clangd

WORKDIR /usr/src/app

# Install compiledb
RUN pipx install compiledb && pipx ensurepath

RUN cargo install cargo-llvm-cov
RUN rustup component add llvm-tools-preview

COPY Cargo.toml .

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN touch src/lib.rs
RUN cargo build --release


RUN cargo install ast-grep --locked
COPY ./src/ast_grep/rules /usr/src/.lsproxy/ast_grep_rules
# Create sgconfig.yml file pointing to rules directory
RUN echo "ruleDirs:\n  - /usr/src/.lsproxy/ast_grep_rules" > /usr/src/sgconfig.yml

# Document that the container listens on port 4444
EXPOSE 4444

# Set environment variables for logging and backtrace
ENV RUST_LOG=debug
ENV RUST_BACKTRACE=1
ENV RA_LOG="/tmp/rust-analyzer.log"

CMD /bin/bash -c "touch Cargo.toml && cargo build --release"
