#!/bin/bash

# Check if OpenAPI Generator is installed
if ! command -v openapi-generator &> /dev/null; then
    echo "OpenAPI Generator could not be found. Please install it first."
    exit 1
fi

# Run OpenAPI Generator with hardcoded values
echo "Generating SDK..."
LANGUAGE=$1
openapi-generator generate \
    -i "openapi.json" \
    -g  "$LANGUAGE" \
    -o "SDK/$LANGUAGE" \
    --additional-properties=packageName=lsproxy_sdk

# Check if generation was successful
if [ $? -eq 0 ]; then
    echo "SDK generated successfully in SDK/$LANGUAGE"
else
    echo "Failed to generate SDK. Please check the error messages above."
fi
