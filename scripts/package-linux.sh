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
  # Create lib directory for Wayland libraries
  mkdir -p "$INSTALL_ROOT/lib/harbor/libs"

  # Create control file with proper dependencies for both X11 and Wayland
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
Depends: libc6, libdbus-1-3, libsecret-1-0, libxkbcommon0, libudev1, libfontconfig1, libasound2, libwayland-client0, libwayland-egl1, libwayland-cursor0, libx11-6, mesa-utils, libxkbcommon-x11-0
Recommends: libvulkan1
EOF

  # Create pre-installation script to ensure system compatibility
  cat > "$PACKAGE_ROOT/DEBIAN/preinst" << 'EOF'
#!/bin/sh
set -e

# Check for Wayland compatibility
WAYLAND_FOUND=0
for lib in libwayland-client.so libwayland-cursor.so libwayland-egl.so; do
    if [ -f "/usr/lib/$lib" ] || [ -f "/usr/lib/x86_64-linux-gnu/$lib" ]; then
        WAYLAND_FOUND=1
        break
    fi
done

# Don't require Wayland - we'll fall back to X11 if not available
if [ "$WAYLAND_FOUND" -eq 0 ]; then
    echo "Warning: Wayland libraries not found, X11 will be used as fallback."
fi

exit 0
EOF

  # Make pre-installation script executable
  chmod 755 "$PACKAGE_ROOT/DEBIAN/preinst"

  # Verify the binary exists
  BINARY_PATH="$RELEASE_DIR/$TARGET"
  if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
  fi

  # Copy Wayland libraries using the same Nix environment used for building
  if [ "${CI:-false}" = "true" ]; then
    # In CI, we're already in a Nix shell, so we can get libraries directly
    echo "Copying libraries from Nix environment to .deb package..."
    
    # Search for libraries in the LD_LIBRARY_PATH from the Nix environment
    IFS=':' read -ra LIB_PATHS <<< "$LD_LIBRARY_PATH"
    for lib_dir in "${LIB_PATHS[@]}"; do
      if [ -d "$lib_dir" ]; then
        for lib_file in "$lib_dir"/libwayland-*.so* "$lib_dir"/libEGL.so* "$lib_dir"/libvulkan.so*; do
          if [ -f "$lib_file" ]; then
            echo "Copying: $lib_file"
            cp -L "$lib_file" "$INSTALL_ROOT/lib/harbor/libs/" 2>/dev/null || true
          fi
        done
      fi
    done
  fi
  
  # Always make sure we have critical libraries, if not found in Nix env
  if [ ! -f "$INSTALL_ROOT/lib/harbor/libs/libwayland-client.so" ]; then
    echo "Falling back to system Wayland libraries..."
    # Copy from system locations as fallback
    for path in /usr/lib/x86_64-linux-gnu /usr/lib; do
      if [ -d "$path" ]; then
        # Copy Wayland main libraries
        for lib in libwayland-client.so* libwayland-cursor.so* libwayland-egl.so*; do
          if [ -f "$path/$lib" ]; then
            echo "Copying system library: $path/$lib"
            cp -L "$path/$lib" "$INSTALL_ROOT/lib/harbor/libs/" 2>/dev/null || true
          fi
        done
        
        # Copy additional required libraries that might be needed
        for lib in libffi.so* libdbus-1.so* libEGL.so* libwl_*.so*; do
          if [ -f "$path/$lib" ]; then
            echo "Copying dependency: $path/$lib"
            cp -L "$path/$lib" "$INSTALL_ROOT/lib/harbor/libs/" 2>/dev/null || true
          fi
        done
      fi
    done
  fi

  # Create a wrapper script that sets up the environment
  cat > "$INSTALL_ROOT/bin/harbor-ui" << 'EOF'
#!/bin/bash
# Wrapper script for harbor-ui that sets up the environment

# Our bundled libraries from Nix should take priority
if [ -d "/usr/lib/harbor/libs" ]; then
  export LD_LIBRARY_PATH="/usr/lib/harbor/libs${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi

# Determine system architecture - only use these if our bundled libs aren't sufficient
if [ "$(uname -m)" = "x86_64" ]; then
  LIB_DIRS=(
    "/usr/lib"
    "/usr/lib/x86_64-linux-gnu"
    "/usr/lib64"
  )
else
  LIB_DIRS=(
    "/usr/lib"
    "/usr/lib/$(uname -m)-linux-gnu"
  )
fi

# Only add system library paths if necessary
if [ ! -f "/usr/lib/harbor/libs/libwayland-client.so" ]; then
  EXTRA_LD_PATHS=""
  for dir in "${LIB_DIRS[@]}"; do
    if [ -d "$dir" ]; then
      EXTRA_LD_PATHS="${EXTRA_LD_PATHS:+$EXTRA_LD_PATHS:}$dir"
    fi
  done
  
  # Append system library paths after our bundled paths
  if [ -n "$EXTRA_LD_PATHS" ]; then
    export LD_LIBRARY_PATH="${LD_LIBRARY_PATH:+$LD_LIBRARY_PATH:}$EXTRA_LD_PATHS"
  fi
fi

# Standard locations for graphics drivers and EGL vendor configs
DRI_PATHS=(
  "/usr/lib/dri"
  "/usr/lib/x86_64-linux-gnu/dri"
  "/usr/lib64/dri"
  "/usr/lib/mesa/dri"  # Added Mesa-specific path
)

EGL_VENDOR_DIRS=(
  "/usr/share/glvnd/egl_vendor.d"
  "/usr/share/egl/egl_vendor.d"
  "/etc/glvnd/egl_vendor.d"
  "/usr/lib/x86_64-linux-gnu/glvnd/egl_vendor.d"  # Added additional vendor path
)

# Find and set DRI path
if [ -z "$LIBGL_DRIVERS_PATH" ]; then
  for path in "${DRI_PATHS[@]}"; do
    if [ -d "$path" ]; then
      export LIBGL_DRIVERS_PATH="$path"
      break
    fi
  done
fi

# Find and set EGL vendor path
if [ -z "$__EGL_VENDOR_LIBRARY_DIRS" ]; then
  for path in "${EGL_VENDOR_DIRS[@]}"; do
    if [ -d "$path" ]; then
      export __EGL_VENDOR_LIBRARY_DIRS="$path"
      break
    fi
  done
fi

# Set XDG_RUNTIME_DIR if not set
if [ -z "$XDG_RUNTIME_DIR" ]; then
  USER_ID=$(id -u)
  XDG_DIR="/run/user/$USER_ID"
  
  # Only set if the directory exists or can be created
  if [ -d "$XDG_DIR" ] || mkdir -p "$XDG_DIR" 2>/dev/null; then
    export XDG_RUNTIME_DIR="$XDG_DIR"
    # Ensure proper permissions
    chmod 700 "$XDG_DIR"
  fi
fi

# Check for Wayland availability and libraries
if [ -f "/usr/lib/harbor/libs/libwayland-client.so" ]; then
  # Use our bundled Wayland libraries from Nix
  export WINIT_UNIX_BACKEND=wayland
  export EGL_PLATFORM=wayland
  export GDK_BACKEND=wayland
  
  # Set WAYLAND_DEBUG for diagnostic information if needed
  # export WAYLAND_DEBUG=1
  
  # Set environment variables similar to what worked in the AppImage
  export LIBGL_DRIVERS_PATH=${LIBGL_DRIVERS_PATH:-/usr/lib/dri}
  export __EGL_VENDOR_LIBRARY_DIRS=${__EGL_VENDOR_LIBRARY_DIRS:-/usr/share/glvnd/egl_vendor.d/}
  
  # Set important environment variables directly
  if [ -z "$WAYLAND_DISPLAY" ] && [ -n "$XDG_RUNTIME_DIR" ]; then
    export WAYLAND_DISPLAY=wayland-0
  fi
  
  # Force direct rendering for better performance
  export LIBGL_ALWAYS_SOFTWARE=0
  
  # Add Vulkan ICD path if it exists
  if [ -d "/usr/share/vulkan/icd.d" ]; then
    export VK_ICD_FILENAMES="/usr/share/vulkan/icd.d/nvidia_icd.json:/usr/share/vulkan/icd.d/intel_icd.json:/usr/share/vulkan/icd.d/radeon_icd.json"
  fi
elif [ -f "/usr/lib/libwayland-client.so" ] || [ -f "/usr/lib/x86_64-linux-gnu/libwayland-client.so" ]; then
  # Fall back to system Wayland libraries if available
  export WINIT_UNIX_BACKEND=wayland
  export EGL_PLATFORM=wayland
  export GDK_BACKEND=wayland
  
  # Set important environment variables directly
  if [ -z "$WAYLAND_DISPLAY" ] && [ -n "$XDG_RUNTIME_DIR" ]; then
    export WAYLAND_DISPLAY=wayland-0
  fi
elif [ "$XDG_SESSION_TYPE" = "wayland" ]; then
  # If we're in a Wayland session but don't have direct library access
  export WINIT_UNIX_BACKEND=wayland
  export WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-wayland-0}
  export GDK_BACKEND=wayland
else
  # Fallback to X11
  export WINIT_UNIX_BACKEND=x11
  export GDK_BACKEND=x11
fi

# Add debugging info to a log file
echo "Environment variables:" > /tmp/harbor-env-debug.log
echo "WINIT_UNIX_BACKEND=$WINIT_UNIX_BACKEND" >> /tmp/harbor-env-debug.log
echo "EGL_PLATFORM=$EGL_PLATFORM" >> /tmp/harbor-env-debug.log
echo "WAYLAND_DISPLAY=$WAYLAND_DISPLAY" >> /tmp/harbor-env-debug.log
echo "XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR" >> /tmp/harbor-env-debug.log
echo "LD_LIBRARY_PATH=$LD_LIBRARY_PATH" >> /tmp/harbor-env-debug.log
echo "LIBGL_DRIVERS_PATH=$LIBGL_DRIVERS_PATH" >> /tmp/harbor-env-debug.log
echo "__EGL_VENDOR_LIBRARY_DIRS=$__EGL_VENDOR_LIBRARY_DIRS" >> /tmp/harbor-env-debug.log
ls -la /usr/lib/harbor/libs >> /tmp/harbor-env-debug.log 2>&1 || true

# Run the actual binary
exec "/usr/lib/harbor/harbor-ui-bin" "$@"
EOF

  # Make the wrapper script executable
  chmod 755 "$INSTALL_ROOT/bin/harbor-ui"

  # Create directory for the actual binary
  mkdir -p "$INSTALL_ROOT/lib/harbor"

  # Copy the actual binary with a different name
  install -Dm755 "$BINARY_PATH" "$INSTALL_ROOT/lib/harbor/harbor-ui-bin"

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

# Fix permissions for bundled libraries
if [ -d "/usr/lib/harbor/libs" ]; then
    chmod 755 /usr/lib/harbor/libs
    chmod 644 /usr/lib/harbor/libs/*.so* 2>/dev/null || true
    
    # Make sure our libraries are properly linked
    for lib in /usr/lib/harbor/libs/*.so.*; do
        if [ -f "$lib" ]; then
            base=$(basename "$lib" | cut -d. -f1)
            ln -sf "$lib" "/usr/lib/harbor/libs/$base.so" 2>/dev/null || true
        fi
    done
fi

# Unlike previous versions, we DO NOT create symlinks to system libraries
# as this might override our carefully bundled libraries

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