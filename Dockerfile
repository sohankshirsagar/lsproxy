# Use the official Rust image as a parent image
FROM rust:1.74-slim-buster as builder

# Set the working directory in the container
WORKDIR /usr/src/app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    git \
    pkg-config \
    libssl-dev

# Copy only the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release

# Remove the dummy main.rs
RUN rm src/main.rs

# Copy the actual source code
COPY src ./src

# Build the application
RUN cargo build --release

# Use a smaller base image for the final container
FROM debian:buster-slim

# Set the working directory in the container
WORKDIR /usr/src/app

# Install runtime dependencies and Python
RUN apt-get update && apt-get install -y \
    libssl1.1 \
    ca-certificates \
    git \
    python3 \
    python3-pip \
    && pip3 install python-lsp-server[all]

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/github-clone-server .

# Copy the index.html file
COPY index.html .

# Document that the container listens on port 8080
EXPOSE 8080

# Run the Rust server with logging
ENV RUST_LOG=debug
CMD ["sh", "-c", "./github-clone-server 2>&1 | tee /var/log/app.log"]
