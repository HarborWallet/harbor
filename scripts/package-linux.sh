#!/bin/bash
set -e  # Exit on any error

TARGET="harbor-ui"
ARCH="amd64"  # Debian uses amd64 instead of x86_64
RELEASE_DIR="target/release"
ASSETS_DIR="harbor-ui/assets"
ARCHIVE_DIR="$RELEASE_DIR/archive"
VERSION=$(grep -m1 '^version = ' harbor-ui/Cargo.toml | cut -d '"' -f2)
ARCHIVE_NAME="$TARGET-$VERSION-$ARCH-linux.tar.gz"
ARCHIVE_PATH="$RELEASE_DIR/$ARCHIVE_NAME"
DEB_NAME="${TARGET}_${VERSION}_${ARCH}.deb"
DEB_PATH="$RELEASE_DIR/$DEB_NAME"

build() {
  # This function is now a no-op when called from CI since the build is done with Nix
  # But kept for backward compatibility or local builds
  if [ "${CI:-false}" != "true" ]; then
    echo "Building harbor-ui locally..."
    cargo build --release --features vendored -p harbor-ui
  else
    echo "Skipping build step (already built via Nix in CI)"
  fi
}

archive_name() {
  echo $ARCHIVE_NAME
}

archive_path() {
  echo $ARCHIVE_PATH
}

deb_path() {
  echo $DEB_PATH
}

create_deb() {
  local PACKAGE_ROOT="$RELEASE_DIR/debian"
  local INSTALL_ROOT="$PACKAGE_ROOT/usr"

  # Clean up any previous package files
  rm -rf "$PACKAGE_ROOT"

  # Create directory structure
  mkdir -p "$PACKAGE_ROOT/DEBIAN"
  mkdir -p "$INSTALL_ROOT/bin"
  mkdir -p "$INSTALL_ROOT/share/applications"
  mkdir -p "$INSTALL_ROOT/share/metainfo"
  mkdir -p "$INSTALL_ROOT/share/icons/hicolor/512x512/apps"
  mkdir -p "$INSTALL_ROOT/share/icons/hicolor/scalable/apps"

  # Create control file
  cat > "$PACKAGE_ROOT/DEBIAN/control" << EOF
Package: harbor
Version: $VERSION
Architecture: $ARCH
Maintainer: benthecarman <benthecarman@live.com> Paul Miller <paul@paul.lol>
Description: Harbor UI Application
 Fedimint ecash desktop wallet for better bitcoin privacy
Section: utils
Priority: optional
Homepage: https://harbor.cash
Depends: libc6
EOF

  # Copy application binary - always use the Nix build path
  BINARY_PATH="$RELEASE_DIR/$TARGET"
  install -Dm755 "$BINARY_PATH" "$INSTALL_ROOT/bin/harbor-ui"

  # Find icons, trying different possible locations
  ICON_PNG=""
  ICON_SVG=""
  
  # Check potential locations for PNG icon
  for potential_png in "harbor-ui/assets/harbor_icon_512x512.png" "harbor-ui/assets/harbor_icon.png"; do
    if [ -f "$potential_png" ]; then
      ICON_PNG="$potential_png"
      break
    fi
  done
  
  # Check potential locations for SVG icon
  for potential_svg in "harbor-ui/assets/harbor_icon.svg" "harbor-ui/assets/linux/harbor_icon.svg"; do
    if [ -f "$potential_svg" ]; then
      ICON_SVG="$potential_svg"
      break
    fi
  done
  
  # Install icons
  if [ -n "$ICON_PNG" ]; then
    install -Dm644 "$ICON_PNG" "$INSTALL_ROOT/share/icons/hicolor/512x512/apps/cash.harbor.harbor.png"
  else
    echo "Warning: PNG icon not found"
  fi
  
  if [ -n "$ICON_SVG" ]; then
    install -Dm644 "$ICON_SVG" "$INSTALL_ROOT/share/icons/hicolor/scalable/apps/cash.harbor.harbor.svg"
  else
    echo "Warning: SVG icon not found"
  fi

  # Find desktop file, trying different possible locations
  DESKTOP_FILE=""
  for potential_desktop in "harbor-ui/assets/linux/cash.harbor.harbor.desktop" "harbor-ui/assets/linux/harbor.desktop"; do
    if [ -f "$potential_desktop" ]; then
      DESKTOP_FILE="$potential_desktop"
      break
    fi
  done
  
  # Install desktop file
  if [ -n "$DESKTOP_FILE" ]; then
    install -Dm644 "$DESKTOP_FILE" "$INSTALL_ROOT/share/applications/cash.harbor.harbor.desktop"
  else
    echo "Warning: Desktop file not found"
  fi

  # Find metadata file, trying different possible locations
  METADATA_FILE=""
  for potential_metadata in "harbor-ui/assets/linux/cash.harbor.harbor.appdata.xml" "harbor-ui/assets/linux/cash.harbor.harbor.metainfo.xml"; do
    if [ -f "$potential_metadata" ]; then
      METADATA_FILE="$potential_metadata"
      break
    fi
  done
  
  # Install AppStream metadata
  if [ -n "$METADATA_FILE" ]; then
    install -Dm644 "$METADATA_FILE" "$INSTALL_ROOT/share/metainfo/cash.harbor.harbor.appdata.xml"
  else
    echo "Warning: Metadata file not found"
  fi

  # Add post-installation script to update icon cache
  cat > "$PACKAGE_ROOT/DEBIAN/postinst" << 'EOF'
#!/bin/sh
set -e
if [ -x "$(command -v update-desktop-database)" ]; then
    update-desktop-database -q
fi
if [ -x "$(command -v gtk-update-icon-cache)" ]; then
    gtk-update-icon-cache -f -t /usr/share/icons/hicolor
fi
EOF

  # Make post-installation script executable
  chmod 755 "$PACKAGE_ROOT/DEBIAN/postinst"

  # Build deb package
  mkdir -p $(dirname "$DEB_PATH")
  dpkg-deb --build "$PACKAGE_ROOT" "$DEB_PATH"
  echo "Created Debian package: $DEB_PATH"
}

package() {
  build || { echo "Build failed"; exit 1; }

  # Make sure archive directory exists
  mkdir -p $ARCHIVE_DIR/bin
  mkdir -p $ARCHIVE_DIR/share/metainfo
  mkdir -p $ARCHIVE_DIR/share/applications

  # Always use the Nix build path
  BINARY_PATH="$RELEASE_DIR/$TARGET"

  # Find metadata file, trying different possible locations
  METADATA_FILE=""
  for potential_metadata in "$ASSETS_DIR/linux/cash.harbor.harbor.appdata.xml" "$ASSETS_DIR/linux/cash.harbor.harbor.metainfo.xml"; do
    if [ -f "$potential_metadata" ]; then
      METADATA_FILE="$potential_metadata"
      break
    fi
  done

  # Find desktop file, trying different possible locations
  DESKTOP_FILE=""
  for potential_desktop in "$ASSETS_DIR/linux/cash.harbor.harbor.desktop" "$ASSETS_DIR/linux/harbor.desktop"; do
    if [ -f "$potential_desktop" ]; then
      DESKTOP_FILE="$potential_desktop"
      break
    fi
  done

  # Copy files for archive
  install -Dm755 "$BINARY_PATH" -t $ARCHIVE_DIR/bin
  if [ -n "$METADATA_FILE" ]; then
    install -Dm644 "$METADATA_FILE" -t $ARCHIVE_DIR/share/metainfo
  fi
  if [ -n "$DESKTOP_FILE" ]; then
    install -Dm644 "$DESKTOP_FILE" -t $ARCHIVE_DIR/share/applications
  fi
  if [ -d "$ASSETS_DIR/icons" ]; then
    cp -r $ASSETS_DIR/icons $ARCHIVE_DIR/share/
  elif [ -d "$ASSETS_DIR/linux/icons" ]; then
    cp -r $ASSETS_DIR/linux/icons $ARCHIVE_DIR/share/
  fi
  cp -fp "$BINARY_PATH" "$ARCHIVE_DIR/harbor"

  # Create archive
  mkdir -p $(dirname "$ARCHIVE_PATH")
  tar czvf $ARCHIVE_PATH -C $ARCHIVE_DIR .

  # Create deb package
  create_deb
}

case "$1" in
  "package") package;;
  "archive_name") archive_name;;
  "archive_path") archive_path;;
  "deb_path") deb_path;;
  *)
    echo "available commands: package, archive_name, archive_path, deb_path"
    ;;
esac