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

RELEASE_DIR="target/release"
APP_DIR="$RELEASE_DIR/macos"
APP_NAME="Harbor.app"
APP_PATH="$APP_DIR/$APP_NAME"
FRAMEWORKS_DIR="$APP_PATH/Contents/Frameworks"

# Clean up any leftover files from previous runs
rm -f certificate.p12 notarization.zip
security delete-keychain build.keychain 2>/dev/null || true

# Check if app exists
if [ ! -d "$APP_PATH" ]; then
    echo "Error: $APP_PATH does not exist"
    exit 1
fi

# Check required environment variables
environment=(
    "MACOS_CERTIFICATE"
    "MACOS_CERTIFICATE_PWD"
    "MACOS_CI_KEYCHAIN_PWD"
    "MACOS_CERTIFICATE_NAME"
    "MACOS_NOTARIZATION_APPLE_ID"
    "MACOS_NOTARIZATION_TEAM_ID"
    "MACOS_NOTARIZATION_PWD"
)

for var in "${environment[@]}"; do
    if [[ -z "${!var}" ]]; then
        echo "Error: $var is not set"
        exit 1
    fi
done

echo "Decoding certificate..."
echo $MACOS_CERTIFICATE | base64 --decode > certificate.p12 2>/dev/null
if [ ! -f certificate.p12 ]; then
    echo "Error: Failed to decode certificate"
    exit 1
fi
echo "Certificate decoded successfully"

echo "Creating and configuring keychain..."
# Remove existing keychain if it exists
security delete-keychain build.keychain 2>/dev/null || true
security create-keychain -p "$MACOS_CI_KEYCHAIN_PWD" build.keychain 2>/dev/null
echo "Created new keychain"

security default-keychain -s build.keychain
echo "Set as default keychain"

security unlock-keychain -p "$MACOS_CI_KEYCHAIN_PWD" build.keychain 2>/dev/null
echo "Unlocked keychain"

echo "Importing certificate..."
security import certificate.p12 -k build.keychain -P "$MACOS_CERTIFICATE_PWD" -T /usr/bin/codesign 2>/dev/null
if [ $? -ne 0 ]; then
    echo "Error: Failed to import certificate"
    exit 1
fi
echo "Certificate imported"

security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$MACOS_CI_KEYCHAIN_PWD" build.keychain 2>/dev/null
echo "Set partition list"

# Only log number of valid signing identities found
echo "Checking for valid signing identities..."
IDENTITIES=$(security find-identity -v -p codesigning build.keychain | grep -c "valid identities found")
echo "Found $IDENTITIES valid signing identities"

# First sign any bundled libraries in the Frameworks directory
if [ -d "$FRAMEWORKS_DIR" ]; then
    echo "Signing bundled libraries in Frameworks directory..."
    find "$FRAMEWORKS_DIR" -type f -name "*.dylib" | while read -r lib; do
        echo "Signing library: $(basename "$lib")"
        /usr/bin/codesign --force --timestamp --sign "$MACOS_CERTIFICATE_NAME" --options runtime "$lib"
    done
fi

echo "Signing Harbor.app..."
# Sign the app with deep option to handle all bundled components
/usr/bin/codesign --force --deep --timestamp -s "$MACOS_CERTIFICATE_NAME" --options runtime "$APP_PATH" -v

echo "Creating keychain profile for notarization..."
xcrun notarytool store-credentials "harbor-notary-profile" \
    --apple-id "$MACOS_NOTARIZATION_APPLE_ID" \
    --team-id "$MACOS_NOTARIZATION_TEAM_ID" \
    --password "$MACOS_NOTARIZATION_PWD" 2>/dev/null

echo "Creating notarization archive..."
ditto -c -k --keepParent "$APP_PATH" "notarization.zip"

echo "Submitting app for notarization..."
xcrun notarytool submit "notarization.zip" --keychain-profile "harbor-notary-profile" --wait

echo "Stapling notarization ticket to app..."
xcrun stapler staple "$APP_PATH"

echo "Cleaning up..."
rm -f certificate.p12 notarization.zip
security delete-keychain build.keychain 2>/dev/null

echo "✨ App signing and notarization complete!" 