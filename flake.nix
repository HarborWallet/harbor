{
  description = "Harbor Flake.nix";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs { inherit system overlays; };
        lib = pkgs.lib;
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        inputs =
          [
            rust
            pkgs.cargo-watch
            pkgs.rust-analyzer
            pkgs.openssl
            pkgs.zlib
            pkgs.sqlcipher
            pkgs.gcc
            pkgs.pkg-config
            pkgs.just
            pkgs.binaryen
            pkgs.clang
            pkgs.expat
            pkgs.llvmPackages.libcxxClang
            pkgs.fontconfig
            pkgs.freetype
            pkgs.freetype.dev
            pkgs.libGL
            pkgs.pkg-config
            pkgs.xorg.libX11
            pkgs.xorg.libXcursor
            pkgs.xorg.libXi
            pkgs.xorg.libXrandr
            pkgs.diesel-cli
            pkgs.nixfmt-rfc-style
            # Make sure patchelf is included for direct binary patching
            pkgs.patchelf

          ]
          ++ lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.AppKit
            pkgs.darwin.apple_sdk.frameworks.CoreText
            pkgs.darwin.apple_sdk.frameworks.WebKit
          ]
          ++ lib.optionals pkgs.stdenv.isLinux [
            # Linux dependencies for building and packaging
            pkgs.sqlcipher
            pkgs.udev
            pkgs.libxkbcommon
            pkgs.alsa-lib
            pkgs.dpkg
            pkgs.fakeroot
            # Linux-specific graphics dependencies
            pkgs.mesa
            pkgs.libglvnd
            pkgs.xorg.libX11
            pkgs.xorg.libXcursor
            pkgs.xorg.libXi
            pkgs.xorg.libXrandr
            pkgs.xorg.libxcb
            pkgs.libxkbcommon
            # Wayland dependencies
            pkgs.wayland
            pkgs.wayland-protocols
            pkgs.wayland-scanner
            # For Vulkan fallback
            pkgs.vulkan-loader
            pkgs.vulkan-headers
            # Keyring stuff
            pkgs.dbus
            pkgs.libsecret
            pkgs.gnome-keyring
            pkgs.libgnome-keyring
          ];
        
        # Define libraries that should be bundled in the .deb package
        debLibraries = with pkgs; [
          # Core libraries
          stdenv.cc.cc.lib
          zlib
          libsecret
          # Graphics libraries
          libGL
          mesa.drivers
          wayland
          libxkbcommon
          vulkan-loader
          # X11 libraries
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          # Sound
          alsa-lib
          # Other dependencies
          dbus
          udev
        ];
      in
      {
        defaultPackage = pkgs.rustPlatform.buildRustPackage {
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = inputs;
        };

        devShell = pkgs.mkShell rec {
          packages = inputs;

          # Important environment variables for EGL and Wayland
          LD_LIBRARY_PATH = lib.makeLibraryPath ([
            pkgs.mesa
            pkgs.libglvnd
            pkgs.xorg.libX11
            pkgs.libxkbcommon
            pkgs.wayland
            # Added libraries to LD_LIBRARY_PATH for keyring
            pkgs.dbus.lib
            pkgs.libsecret
          ]);
          
          # Export list of libraries that should be bundled
          DEB_LIBRARIES = lib.makeLibraryPath debLibraries;

          shellHook = ''
            export LIBCLANG_PATH=${pkgs.libclang.lib}/lib/
            ${lib.optionalString pkgs.stdenv.isLinux ''
              # Add important Mesa paths (Linux only)
              export LIBGL_DRIVERS_PATH=${pkgs.mesa.drivers}/lib/dri
              export __EGL_VENDOR_LIBRARY_DIRS=${pkgs.mesa.drivers}/share/glvnd/egl_vendor.d/
              
              # Wayland specific environment variables - only set if directory exists
              if [ -d "/run/user/$(id -u)" ]; then
                export XDG_RUNTIME_DIR="/run/user/$(id -u)"
              fi
              
              # Set Wayland display if in a Wayland session
              if [ "$XDG_SESSION_TYPE" = "wayland" ]; then
                export WAYLAND_DISPLAY=wayland-0
              fi
              
              # Only try to start DBus and keyring if not in CI
              if [ "${builtins.getEnv "CI"}" != "true" ]; then
                # Ensure DBus session is available for keyring
                if [ -z "$DBUS_SESSION_BUS_ADDRESS" ] && command -v dbus-daemon >/dev/null; then
                  dbus_output=$(dbus-launch --sh-syntax 2>/dev/null || true)
                  if [ -n "$dbus_output" ]; then
                    eval "$dbus_output"
                    export DBUS_SESSION_BUS_ADDRESS
                  fi
                fi
                
                # Start gnome-keyring-daemon if not running and command exists
                if command -v gnome-keyring-daemon >/dev/null && ! pgrep -x "gnome-keyring-d" > /dev/null; then
                  keyring_output=$(gnome-keyring-daemon --start --components=secrets 2>/dev/null || true)
                  if [ -n "$keyring_output" ]; then
                    eval "$keyring_output"
                    export GNOME_KEYRING_CONTROL
                    export SSH_AUTH_SOCK
                  fi
                fi
              fi
            ''}
          '';
        };
      }
    );
}
