# Use the official Rust image as a parent image
FROM rust:1.74-slim-buster as builder

# Set the working directory in the container
WORKDIR /usr/src/app


# Install build dependencies
RUN apt-get update && apt-get install -y \
    git \
    pkg-config \
    libssl-dev

# Copy the current directory contents into the container
COPY . .

# Build the application
RUN cargo build --release

# Use a smaller base image for the final container
FROM debian:buster-slim

# Set the working directory in the container
WORKDIR /usr/src/app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl1.1 \
    ca-certificates \
    git

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/github-clone-server .

# Copy the index.html file
COPY index.html .

# Document that the container listens on port 8080
EXPOSE 8080

# Run the binary
CMD ["./github-clone-server"]
