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

# Install runtime dependencies, Python, Node.js, and npm
RUN apt-get update && apt-get install -y \
    libssl1.1 \
    ca-certificates \
    git \
    python3 \
    python3-pip \
    curl \
    && curl -fsSL https://deb.nodesource.com/setup_16.x | bash - \
    && apt-get install -y nodejs \
    && pip3 install pyright

# Install Pyright globally
RUN npm install -g pyright

COPY openapi.yaml .
COPY configs/pyrightconfig.json .
# Copy the binary from the builder stage and the openapi.yaml file
COPY --from=builder /usr/src/app/target/release/github-clone-server .


# Document that the container listens on port 8080
EXPOSE 8080

# Create log directory and set permissions
RUN mkdir -p /var/log && chmod 755 /var/log

# Run a simple check before starting the server
RUN echo '#!/bin/sh\necho "Container is starting..."\nexec ./github-clone-server 2>&1 | tee -a /var/log/app.log' > /usr/src/app/start.sh && chmod +x /usr/src/app/start.sh

# Run the start script
CMD ["/usr/src/app/start.sh"]
