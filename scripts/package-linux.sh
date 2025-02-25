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
  local BINARY_PATH="$RELEASE_DIR/$TARGET"

  # Clean up any previous package files
  rm -rf "$PACKAGE_ROOT"

  # Create directory structure
  mkdir -p "$PACKAGE_ROOT/DEBIAN"
  mkdir -p "$INSTALL_ROOT/bin"
  mkdir -p "$INSTALL_ROOT/share/applications"
  mkdir -p "$INSTALL_ROOT/share/metainfo"
  mkdir -p "$INSTALL_ROOT/share/icons/hicolor/512x512/apps"
  mkdir -p "$INSTALL_ROOT/share/icons/hicolor/scalable/apps"
  mkdir -p "$INSTALL_ROOT/lib/harbor"
  mkdir -p "$INSTALL_ROOT/lib/harbor/libs"

  # Verify the binary exists
  if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
  fi

  # Copy the binary
  install -Dm755 "$BINARY_PATH" "$INSTALL_ROOT/lib/harbor/harbor-ui-bin"

  # Bundle required Wayland and graphics libraries
  echo "Bundling libraries for Wayland and graphics support..."
  
  if [ -n "$DEB_LIBRARIES" ]; then
    # Use libraries from Nix environment
    echo "Using libraries from Nix environment (DEB_LIBRARIES)"
    
    IFS=':' read -ra LIB_PATHS <<< "$DEB_LIBRARIES"
    for lib_dir in "${LIB_PATHS[@]}"; do
      if [ -d "$lib_dir" ]; then
        echo "Searching in: $lib_dir"
        # Copy all Wayland, GL, other important libraries, and core C runtime libraries
        for pattern in "libwayland*.so*" "libEGL*.so*" "libGL*.so*" "libvulkan*.so*" "libxkbcommon*.so*" "libX*.so*" "libdbus*.so*" "libffi*.so*" "libgcc_s*.so*" "libc.so*" "ld-linux*.so*" "libm.so*" "librt.so*" "libpthread.so*" "libstdc++*.so*" "libICE.so*" "libSM.so*"; do
          for lib_file in "$lib_dir"/$pattern; do
            if [ -f "$lib_file" ] && [ ! -L "$lib_file" ]; then
              echo "  - Copying: $lib_file"
              cp -L "$lib_file" "$INSTALL_ROOT/lib/harbor/libs/" 2>/dev/null || true
            fi
          done
        done
      fi
    done
  else
    # Fallback to system libraries
    echo "DEB_LIBRARIES not set, falling back to system libraries"
    for lib_dir in /usr/lib/x86_64-linux-gnu /usr/lib /lib/x86_64-linux-gnu /lib; do
      if [ -d "$lib_dir" ]; then
        echo "Searching in: $lib_dir"
        for lib in libwayland-client.so* libwayland-cursor.so* libwayland-egl.so* libEGL.so* libGL.so* libdbus-1.so* libxkbcommon.so* libc.so* ld-linux-x86-64.so* libm.so* librt.so* libpthread.so* libstdc++.so* libICE.so* libSM.so*; do
          if [ -f "$lib_dir/$lib" ]; then
            echo "  - Copying: $lib_dir/$lib"
            cp -L "$lib_dir/$lib" "$INSTALL_ROOT/lib/harbor/libs/" 2>/dev/null || true
          fi
        done
      fi
    done
  fi

  # Make sure library permissions are correct
  chmod 755 "$INSTALL_ROOT/lib/harbor/libs"
  chmod 644 "$INSTALL_ROOT/lib/harbor/libs/"*.so*

  # Create proper symlinks for the libraries
  echo "Creating library symlinks..."
  for lib in "$INSTALL_ROOT/lib/harbor/libs/"*.so.*; do
    if [ -f "$lib" ]; then
      base=$(basename "$lib" | grep -o '^[^.]*\.so')
      if [ -n "$base" ]; then
        target=$(basename "$lib")
        echo "  - Linking: $base -> $target"
        ln -sf "$target" "$INSTALL_ROOT/lib/harbor/libs/$base"
      fi
    fi
  done
  
  # Verify we have the critical Wayland libraries
  if [ ! -f "$INSTALL_ROOT/lib/harbor/libs/libwayland-client.so" ]; then
    echo "WARNING: Failed to bundle libwayland-client.so"
  fi
  
  if [ ! -f "$INSTALL_ROOT/lib/harbor/libs/libwayland-egl.so" ]; then
    echo "WARNING: Failed to bundle libwayland-egl.so"
  fi

  # Manual patching of the binary's RPATH
  if command -v patchelf >/dev/null; then
    echo "Patching RPATH of binary to use bundled libraries..."
    patchelf --set-rpath "/usr/lib/harbor/libs:/usr/lib:/usr/lib/x86_64-linux-gnu:/lib/x86_64-linux-gnu" "$INSTALL_ROOT/lib/harbor/harbor-ui-bin"
    echo "New RPATH: $(patchelf --print-rpath "$INSTALL_ROOT/lib/harbor/harbor-ui-bin")"
  else
    echo "WARNING: patchelf not found, cannot patch binary RPATH!"
    echo "You must install patchelf to create a working .deb package."
    exit 1
  fi

  # Create a wrapper script that sets up the environment
  cat > "$INSTALL_ROOT/bin/harbor-ui" << 'EOF'
#!/bin/bash
# Wrapper script for harbor-ui

# Ensure bundled libraries are used
export LD_LIBRARY_PATH="/usr/lib/harbor/libs:${LD_LIBRARY_PATH}"

# Detect Wayland and set proper environment
if [ "$XDG_SESSION_TYPE" = "wayland" ] || [ -n "$WAYLAND_DISPLAY" ]; then
  export WINIT_UNIX_BACKEND=wayland
  export GDK_BACKEND=wayland
  export EGL_PLATFORM=wayland
  
  # Set WAYLAND_DISPLAY if not already set
  if [ -z "$WAYLAND_DISPLAY" ] && [ -n "$XDG_RUNTIME_DIR" ]; then
    export WAYLAND_DISPLAY=wayland-0
  fi
else
  # Fallback to X11
  export WINIT_UNIX_BACKEND=x11
  export GDK_BACKEND=x11
fi

# Set standard paths for graphics drivers
export LIBGL_DRIVERS_PATH=${LIBGL_DRIVERS_PATH:-/usr/lib/dri}
export __EGL_VENDOR_LIBRARY_DIRS=${__EGL_VENDOR_LIBRARY_DIRS:-/usr/share/glvnd/egl_vendor.d/}

# Try software rendering if hardware fails
export LIBGL_ALWAYS_SOFTWARE=1

# For debugging - uncomment if needed
# export WAYLAND_DEBUG=1

# List loaded libraries for debugging
if [ "${HARBOR_DEBUG:-0}" = "1" ]; then
  echo "Environment variables:" > /tmp/harbor-env-debug.log
  env | grep -E "WAYLAND|XDG|LD_LIBRARY|LIBGL|EGL|WINIT|GDK" >> /tmp/harbor-env-debug.log
  echo "Library path contents:" >> /tmp/harbor-env-debug.log
  ls -la /usr/lib/harbor/libs >> /tmp/harbor-env-debug.log 2>&1
fi

# Run the actual binary
exec "/usr/lib/harbor/harbor-ui-bin" "$@"
EOF

  # Make the wrapper script executable
  chmod 755 "$INSTALL_ROOT/bin/harbor-ui"

  # Create control file with dependencies
  cat > "$PACKAGE_ROOT/DEBIAN/control" << EOF
Package: harbor
Version: $VERSION
Architecture: $ARCH
Maintainer: benthecarman <benthecarman@live.com>, Paul Miller <paul@paul.lol>
Description: Harbor UI Application
 Fedimint ecash desktop wallet for better bitcoin privacy
Section: utils
Priority: optional
Homepage: https://harbor.cash
Depends: libc6, libx11-6
Recommends: libwayland-client0, libwayland-egl1, libwayland-cursor0, libxkbcommon0, libgl1
EOF

  # Install icons - we know exactly where they are
  install -Dm644 "harbor-ui/assets/harbor_icon.png" "$INSTALL_ROOT/share/icons/hicolor/512x512/apps/cash.harbor.harbor.png"
  
  # Install desktop file
  install -Dm644 "harbor-ui/assets/linux/cash.harbor.harbor.desktop" "$INSTALL_ROOT/share/applications/cash.harbor.harbor.desktop"

  # Install AppStream metadata
  install -Dm644 "harbor-ui/assets/linux/cash.harbor.harbor.appdata.xml" "$INSTALL_ROOT/share/metainfo/cash.harbor.harbor.appdata.xml"

  # Add post-installation script to update icon cache
  cat > "$PACKAGE_ROOT/DEBIAN/postinst" << 'EOF'
#!/bin/sh
set -e

# Update desktop database and icon cache
if [ -x "$(command -v update-desktop-database)" ]; then
    update-desktop-database -q
fi
if [ -x "$(command -v gtk-update-icon-cache)" ]; then
    gtk-update-icon-cache -f -t /usr/share/icons/hicolor
fi

exit 0
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

  # Copy files for archive
  install -Dm755 "$BINARY_PATH" -t $ARCHIVE_DIR/bin
  install -Dm644 "harbor-ui/assets/linux/cash.harbor.harbor.appdata.xml" -t $ARCHIVE_DIR/share/metainfo
  install -Dm644 "harbor-ui/assets/linux/cash.harbor.harbor.desktop" -t $ARCHIVE_DIR/share/applications
  
  # Copy icons
  if [ -d "$ASSETS_DIR/linux/icons" ]; then
    cp -r $ASSETS_DIR/linux/icons $ARCHIVE_DIR/share/
  fi
  
  # Copy the binary with the name 'harbor' for backward compatibility
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