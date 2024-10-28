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
    curl \
    && curl -fsSL https://deb.nodesource.com/setup_16.x | bash - \
    && apt-get install -y nodejs \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install rust-analyzer
RUN rustup component add rust-analyzer

# Install global npm packages
RUN npm install -g pyright typescript-language-server typescript

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

CMD /bin/bash -c "touch Cargo.toml && cargo build --release"
