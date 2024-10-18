#!/bin/bash

set -e  # Exit immediately if a command exits with a non-zero status

# Run the build
./scripts/build.sh

# Run the application
docker run --rm -p 4444:4444 -v $1:/mnt/workspace -v "$(pwd)/lsproxy/target/debug":/usr/src/app lsproxy-dev ./lsproxy
