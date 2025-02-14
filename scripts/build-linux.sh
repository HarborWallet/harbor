#!/bin/bash

set -e # Exit on error

TARGET="harbor-ui"
RELEASE_DIR="target/release"
APP_NAME="Harbor"
LINUX_DIR="$RELEASE_DIR/linux"

# Get version from Cargo.toml
VERSION=$(grep -m1 '^version = ' harbor-ui/Cargo.toml | cut -d '"' -f2)
BUILD=$(git describe --always --dirty --exclude='*')

# Build binary
echo "Building Harbor for Linux..."
cargo build --release --features vendored

# Generate Linux icons
./scripts/generate-linux-icons.sh

# Create AppDir structure
APPDIR="$LINUX_DIR/$APP_NAME.AppDir"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons"
mkdir -p "$APPDIR/usr/share/metainfo"

# Copy binary
cp "$RELEASE_DIR/$TARGET" "$APPDIR/usr/bin/harbor"

# Copy desktop file and rename it
cp "harbor-ui/assets/linux/harbor.desktop" "$APPDIR/usr/share/applications/cash.harbor.harbor.desktop"

# Copy metadata file
cp "harbor-ui/assets/linux/cash.harbor.harbor.metainfo.xml" "$APPDIR/usr/share/metainfo/"

# Copy icons
cp -r "harbor-ui/assets/linux/icons" "$APPDIR/usr/share/"

# Create AppRun script
cat > "$APPDIR/AppRun" << 'EOF'
#!/bin/bash
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
export PATH="${HERE}/usr/bin/:${PATH}"
exec "${HERE}/usr/bin/harbor" "$@"
EOF

chmod +x "$APPDIR/AppRun"

# Create symlinks
cd "$APPDIR"
ln -sf usr/share/applications/cash.harbor.harbor.desktop .
ln -sf usr/share/icons/256x256/harbor.png cash.harbor.harbor.png
cd -

# Generate AppImage
echo "Generating AppImage..."
# Get architecture and normalize to common format
ARCH=$(uname -m)
if [ "$ARCH" = "aarch64" ]; then
    ARCH="arm64"
elif [ "$ARCH" = "x86_64" ]; then
    ARCH="x86_64"
fi

# Set architecture explicitly for appimagetool
export ARCH
appimagetool --no-appstream "$APPDIR" "$LINUX_DIR/$APP_NAME-$VERSION-$ARCH.AppImage"

echo "✨ Created AppImage in $LINUX_DIR" 