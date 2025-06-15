#!/bin/bash

set -e # Exit on error

RELEASE_DIR="target/release"
APP_DIR="$RELEASE_DIR/macos"
APP_NAME="Harbor.app"
DMG_DIR="$RELEASE_DIR/macos"

# package dmg
echo "Packing disk image..."
ln -sf /Applications "$DMG_DIR/Applications"
hdiutil create "$DMG_DIR/$DMG_NAME" -volname "Harbor" -fs HFS+ -srcfolder "$APP_DIR" -ov -format UDZO
echo "✨ Packed '$APP_NAME' in '$DMG_DIR/$DMG_NAME'" 