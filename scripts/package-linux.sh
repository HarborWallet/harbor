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
  cargo build --release --target=x86_64-unknown-linux-gnu --features vendored -p harbor-ui
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

  # Copy application binary
  install -Dm755 "target/x86_64-unknown-linux-gnu/release/$TARGET" "$INSTALL_ROOT/bin/harbor-ui"

  # Install PNG icon
  install -Dm644 "harbor-ui/assets/harbor_icon_512x512.png" \
    "$INSTALL_ROOT/share/icons/hicolor/512x512/apps/cash.harbor.harbor.png"

  # Install SVG icon
  install -Dm644 "harbor-ui/assets/harbor_icon.svg" \
    "$INSTALL_ROOT/share/icons/hicolor/scalable/apps/cash.harbor.harbor.svg"

  # Install .desktop file
  install -Dm644 "harbor-ui/assets/linux/cash.harbor.harbor.desktop" \
    "$INSTALL_ROOT/share/applications/cash.harbor.harbor.desktop"

  # Install AppStream metadata
  install -Dm644 "harbor-ui/assets/linux/cash.harbor.harbor.appdata.xml" \
    "$INSTALL_ROOT/share/metainfo/cash.harbor.harbor.appdata.xml"

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
  dpkg-deb --build "$PACKAGE_ROOT" "$DEB_PATH"
  echo "Created Debian package: $DEB_PATH"
}

package() {
  build || { echo "Build failed"; exit 1; }

  # Create tar.gz archive
  install -Dm755 target/x86_64-unknown-linux-gnu/release/$TARGET -t $ARCHIVE_DIR/bin
  install -Dm644 $ASSETS_DIR/linux/cash.harbor.harbor.appdata.xml -t $ARCHIVE_DIR/share/metainfo
  install -Dm644 $ASSETS_DIR/linux/cash.harbor.harbor.desktop -t $ARCHIVE_DIR/share/applications
  cp -r $ASSETS_DIR/icons $ARCHIVE_DIR/share/
  cp -fp "target/x86_64-unknown-linux-gnu/release/$TARGET" "$ARCHIVE_DIR/harbor"
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