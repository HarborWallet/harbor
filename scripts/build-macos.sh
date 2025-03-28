#!/bin/bash

set -e # Exit on error

TARGET="harbor-ui"
ASSETS_DIR="harbor-ui/assets"
RELEASE_DIR="target/release"
APP_NAME="Harbor.app"
APP_TEMPLATE="$ASSETS_DIR/macos/$APP_NAME"
APP_TEMPLATE_PLIST="$APP_TEMPLATE/Contents/Info.plist"
APP_DIR="$RELEASE_DIR/macos"
APP_BINARY="$RELEASE_DIR/$TARGET"
APP_BINARY_DIR="$APP_DIR/$APP_NAME/Contents/MacOS"
APP_EXTRAS_DIR="$APP_DIR/$APP_NAME/Contents/Resources"

DMG_NAME="harbor.dmg"
DMG_DIR="$RELEASE_DIR/macos"

# Get version from Cargo.toml
VERSION=$(grep -m1 '^version = ' harbor-ui/Cargo.toml | cut -d '"' -f2)
BUILD=$(git describe --always --dirty --exclude='*')

# Create a temporary copy of the Info.plist
cp "$APP_TEMPLATE_PLIST" "$APP_TEMPLATE_PLIST.tmp"

# Update version and build in the temporary file
sed -i.bak "s/{{ VERSION }}/$VERSION/g" "$APP_TEMPLATE_PLIST.tmp"
sed -i.bak "s/{{ BUILD }}/$BUILD/g" "$APP_TEMPLATE_PLIST.tmp"

# Move the temporary file back
mv "$APP_TEMPLATE_PLIST.tmp" "$APP_TEMPLATE_PLIST"
rm -f "$APP_TEMPLATE_PLIST.tmp.bak"

# build binary
export MACOSX_DEPLOYMENT_TARGET="11.0"

echo "Building Harbor for Apple Silicon..."
# Build from the root directory and explicitly specify the package
cargo build --release --target=aarch64-apple-darwin --features vendored -p harbor-ui

echo "Creating app bundle..."
# build app
mkdir -p "$APP_BINARY_DIR"
mkdir -p "$APP_EXTRAS_DIR"
cp -fRp "$APP_TEMPLATE" "$APP_DIR"
cp -fp "target/aarch64-apple-darwin/release/$TARGET" "$APP_BINARY_DIR/harbor"
touch -r "target/aarch64-apple-darwin/release/$TARGET" "$APP_DIR/$APP_NAME"

# Fix dynamic library paths
# echo "Fixing dynamic library paths..."
# ./scripts/fix-dylib-paths.sh "$APP_DIR/$APP_NAME"

echo "✨ Created '$APP_NAME' in '$APP_DIR'"
