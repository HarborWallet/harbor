#!/bin/bash
set -e  # Exit on any error

TARGET="harbor-ui"
ARCH="x86_64"
RELEASE_DIR="target/release"
ASSETS_DIR="harbor-ui/assets"
ARCHIVE_DIR="$RELEASE_DIR/archive"
VERSION=$(grep -m1 '^version = ' harbor-ui/Cargo.toml | cut -d '"' -f2)
ARCHIVE_NAME="$TARGET-$VERSION-$ARCH-linux.tar.gz"
ARCHIVE_PATH="$RELEASE_DIR/$ARCHIVE_NAME"

build() {
  cargo build --release --target=x86_64-unknown-linux-gnu --features vendored -p harbor-ui
}

archive_name() {
  echo $ARCHIVE_NAME
}

archive_path() {
  echo $ARCHIVE_PATH
}

package() {
  build || { echo "Build failed"; exit 1; }  # Explicit error handling for build

  install -Dm755 target/x86_64-unknown-linux-gnu/release/$TARGET -t $ARCHIVE_DIR/bin
  install -Dm644 $ASSETS_DIR/linux/cash.harbor.harbor.appdata.xml -t $ARCHIVE_DIR/share/metainfo
  install -Dm644 $ASSETS_DIR/linux/cash.harbor.harbor.desktop -t $ARCHIVE_DIR/share/applications
  cp -r $ASSETS_DIR/icons $ARCHIVE_DIR/share/
  cp -fp "target/x86_64-unknown-linux-gnu/release/$TARGET" "$ARCHIVE_DIR/harbor"

  tar czvf $ARCHIVE_PATH -C $ARCHIVE_DIR .

  echo "Packaged archive: $ARCHIVE_PATH"
}

case "$1" in
  "package") package;;
  "archive_name") archive_name;;
  "archive_path") archive_path;;
  *)
    echo "available commands: package, archive_name, archive_path"
    ;;
esac