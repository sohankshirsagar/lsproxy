#!/bin/bash

set -e  # Exit immediately if a command exits with a non-zero status

./scripts/build.sh

if ! docker run --rm -v "$(pwd)/lsproxy":/usr/src/app lsproxy-dev cargo llvm-cov; then
    echo "Tests failed. Exiting."
    exit 1
fi
