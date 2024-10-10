#!/bin/bash

# Build the application using the build Dockerfile
docker build -t lsp-box-builder -f dockerfiles/build .

# Run the builder container to create the binary
docker run --rm -v `pwd`:/usr/src/app lsp-box-builder

# Build the runner image
docker build -t lsp-box-runner -f dockerfiles/run .

# Run the application
docker run -p 8080:8080 lsp-box-runner
