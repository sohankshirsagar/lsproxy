#!/bin/bash

set -e  # Exit immediately if a command exits with a non-zero status

# Build the application using the build Dockerfile
docker build -t lsp-box-builder -f dockerfiles/build lsproxy

# Run the builder container to create the binary
if ! docker run --rm -v "$(pwd)/lsproxy":/usr/src/app lsp-box-builder; then
    echo "Build failed. Exiting."
    exit 1
fi

# Build the runner image
docker build -t lsp-box-runner -f dockerfiles/run lsproxy

# Run the application
docker run --rm -p 8080:8080 -v $1:/mnt/repo lsp-box-runner
