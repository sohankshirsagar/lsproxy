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
RUN rustup component add rustfmt

# Install global npm packages
RUN npm install -g pyright typescript-language-server typescript

#Install Gopls
WORKDIR /tmp
RUN curl -O -L "https://go.dev/dl/go1.23.2.linux-amd64.tar.gz"
RUN tar -C /usr/local -xzf go1.23.2.linux-amd64.tar.gz
RUN rm go1.23.2.linux-amd64.tar.gz
RUN /usr/local/go/bin/go install golang.org/x/tools/gopls@latest
RUN cp ~/go/bin/gopls /usr/local/bin/gopls

#Install Clangd
RUN apt-get install unzip
RUN curl -O -L https://github.com/clangd/clangd/releases/download/18.1.3/clangd-linux-18.1.3.zip
RUN unzip clangd-linux-18.1.3.zip
RUN mv clangd_18.1.3 /usr/local/
RUN ln -s /usr/local/clangd_18.1.3/bin/clangd /usr/local/bin/clangd
RUN rm clangd-linux-18.1.3.zip

WORKDIR /usr/src/app

#Install compiledb
RUN python3 -m pip install compiledb

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
