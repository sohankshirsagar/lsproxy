#!/bin/bash
set -e

# Build the Docker image
echo "Building Docker image..."
docker build -t lsproxy-builder -f release/Dockerfile lsproxy

# Create a temporary container
CONTAINER_ID=$(docker create lsproxy-builder)

# Copy the binary from the container to /tmp
echo "Extracting binary to /tmp..."
docker cp $CONTAINER_ID:/usr/local/bin/lsproxy-bin /tmp/lsproxy

# Cleanup
echo "Cleaning up..."
docker rm $CONTAINER_ID

echo "Binary extracted to /tmp/lsproxy"
