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

# Bundle libintl.8.dylib with the app
echo "Bundling and fixing libintl.8.dylib..."
FRAMEWORKS_DIR="$APP_DIR/$APP_NAME/Contents/Frameworks"
# Ensure directory exists with proper permissions
mkdir -p "$FRAMEWORKS_DIR"
chmod 755 "$FRAMEWORKS_DIR"

# Find libintl.8.dylib in nix store
LIBINTL_PATH=$(find /nix/store -name "libintl.8.dylib" -type f | head -n 1)
if [ -n "$LIBINTL_PATH" ]; then
    echo "Found libintl at: $LIBINTL_PATH"
    
    # Copy to Frameworks directory with sudo if needed
    echo "Copying library to Frameworks..."
    if ! cp "$LIBINTL_PATH" "$FRAMEWORKS_DIR/"; then
        echo "Standard copy failed, trying with elevated permissions..."
        # Try with sudo if available
        if command -v sudo >/dev/null 2>&1; then
            sudo cp "$LIBINTL_PATH" "$FRAMEWORKS_DIR/"
            sudo chown $(whoami) "$FRAMEWORKS_DIR/libintl.8.dylib"
            sudo chmod 644 "$FRAMEWORKS_DIR/libintl.8.dylib"
        else
            echo "ERROR: Could not copy libintl.8.dylib to Frameworks directory"
            exit 1
        fi
    fi
    
    # Update binary to reference the bundled library using @rpath
    echo "Fixing reference in binary..."
    install_name_tool -add_rpath "@executable_path/../Frameworks" "$APP_BINARY_DIR/harbor"
    install_name_tool -change "$LIBINTL_PATH" "@rpath/libintl.8.dylib" "$APP_BINARY_DIR/harbor"
    
    # Check if there are any other dependencies of libintl.8.dylib
    SUB_DEPS=$(otool -L "$FRAMEWORKS_DIR/libintl.8.dylib" | grep -v "/System/" | grep -v "@rpath" | grep -v "@executable_path" | grep -v "/usr/lib/" | awk -F' ' '{print $1}')
    if [ -n "$SUB_DEPS" ]; then
        echo "Processing dependencies of libintl.8.dylib..."
        for SUB_DEP_PATH in $SUB_DEPS; do
            # Skip if it's referring to itself
            if [[ "$SUB_DEP_PATH" == *"libintl.8.dylib"* ]]; then
                continue
            fi
            
            SUB_DEP_NAME=$(basename "$SUB_DEP_PATH")
            echo "Processing dependency: $SUB_DEP_NAME"
            
            # Copy to Frameworks directory with sudo if needed
            if ! cp "$SUB_DEP_PATH" "$FRAMEWORKS_DIR/"; then
                echo "Standard copy failed for $SUB_DEP_NAME, trying with elevated permissions..."
                if command -v sudo >/dev/null 2>&1; then
                    sudo cp "$SUB_DEP_PATH" "$FRAMEWORKS_DIR/"
                    sudo chown $(whoami) "$FRAMEWORKS_DIR/$SUB_DEP_NAME"
                    sudo chmod 644 "$FRAMEWORKS_DIR/$SUB_DEP_NAME"
                else
                    echo "WARNING: Could not copy $SUB_DEP_NAME to Frameworks directory"
                    continue
                fi
            fi
            
            # Fix the reference in libintl
            install_name_tool -change "$SUB_DEP_PATH" "@rpath/$SUB_DEP_NAME" "$FRAMEWORKS_DIR/libintl.8.dylib"
            
            # Set the ID
            install_name_tool -id "@rpath/$SUB_DEP_NAME" "$FRAMEWORKS_DIR/$SUB_DEP_NAME"
        done
    fi
    
    # Fix the ID of libintl itself
    install_name_tool -id "@rpath/libintl.8.dylib" "$FRAMEWORKS_DIR/libintl.8.dylib"
else
    echo "Warning: Could not find libintl.8.dylib in nix store!"
fi

echo "✨ Created '$APP_NAME' in '$APP_DIR'"
