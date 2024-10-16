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

# Run the application to generate the OpenAPI spec
docker run --name temp_lsp_box lsp-box-runner -w

# Copy the generated OpenAPI spec from the container to the host
docker cp temp_lsp_box:/usr/src/app/openapi.json ./openapi.json

# Remove the temporary container
docker rm temp_lsp_box

echo "OpenAPI specification has been generated and saved as openapi.json"
