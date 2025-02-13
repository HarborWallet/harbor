#!/bin/bash

set -e # Exit on error

# Source the environment variables if .env exists
if [ -f ".env" ]; then
    set -a  # automatically export all variables
    source .env
    set +a
fi

# Debug: Print shell information
echo "Shell: $SHELL"
echo "Bash Version: $BASH_VERSION"

# Script to test access to GitHub secrets
echo "Testing access to GitHub secrets..."

# Check required environment variables
environment=(
    "MACOS_CERTIFICATE"
    "MACOS_CERTIFICATE_PWD"
    "MACOS_CERTIFICATE_NAME"
)

for var in "${environment[@]}"; do
    if [[ -z "${!var}" ]]; then
        echo "❌ Error: $var is not set"
        exit 1
    else
        echo "✅ $var is set"
        # Print first character of the secret if it exists (for safety)
        echo "$var starts with: ${!var:0:1}"
    fi
done

echo "✨ Secret test complete!" 