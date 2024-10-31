FROM rust:1.81.0-slim-bookworm as builder

WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y curl

COPY . .
RUN cargo build --release

FROM rust:1.81.0-slim-bookworm

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    universal-ctags \
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

# Add rust-analyzer
RUN rustup component add rust-analyzer

# Install language servers
RUN npm install -g pyright typescript-language-server typescript

# Install ast grep
RUN cargo install ast-grep --locked
COPY ./src/ast_grep/rules /usr/src/.lsproxy/ast_grep_rules

# Create sgconfig.yml file pointing to rules directory
RUN echo "ruleDirs:\n  - /usr/src/.lsproxy/ast_grep_rules" > /usr/src/sgconfig.yml

# Copy the binary
COPY --from=builder /usr/src/app/target/release/lsproxy .

EXPOSE 4444
ENV RUST_LOG=info
ENV RA_LOG="/tmp/rust-analyzer.log"
CMD ["./lsproxy"]
