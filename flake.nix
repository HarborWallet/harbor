{
  description = "Harbor Flake.nix";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "nixpkgs/nixos-24.11";
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
            pkgs.diesel-cli
            pkgs.nixfmt-rfc-style
            pkgs.dbus
            pkgs.libsecret
            pkgs.protobuf
          ]
          ++ lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.AppKit
            pkgs.darwin.apple_sdk.frameworks.CoreText
            pkgs.darwin.apple_sdk.frameworks.WebKit
          ]
          ++ lib.optionals pkgs.stdenv.isLinux [
            pkgs.xorg.libX11
            pkgs.xorg.libXcursor
            pkgs.xorg.libXi
            pkgs.xorg.libXrandr
            pkgs.gnome-keyring
            pkgs.libgnome-keyring
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
          packages = inputs ++ [
            pkgs.git
            pkgs.mesa
            pkgs.libglvnd # Adds EGL support
            pkgs.xorg.libX11
            pkgs.xorg.libXcursor
            pkgs.xorg.libXi
            pkgs.xorg.libXrandr
            pkgs.xorg.libxcb
            pkgs.libxkbcommon
            pkgs.wayland
            # Wayland-specific dependencies
            pkgs.wayland-protocols
            pkgs.wayland-scanner
            # For Vulkan fallback (wgpu might need this)
            pkgs.vulkan-loader
          ];

          # Important environment variables for EGL and Wayland
          # EGL_PLATFORM = "wayland"; # Changed from x11 to wayland
          # WAYLAND_DISPLAY = "wayland-0"; # Default Wayland display
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

          shellHook = ''
            export LIBCLANG_PATH=${pkgs.libclang.lib}/lib/
            # Add important Mesa paths
            export LIBGL_DRIVERS_PATH=${if pkgs.mesa ? drivers then pkgs.mesa.drivers else pkgs.mesa}/lib/dri
            export __EGL_VENDOR_LIBRARY_DIRS=${if pkgs.mesa ? drivers then pkgs.mesa.drivers else pkgs.mesa}/share/glvnd/egl_vendor.d/
            # Wayland specific environment variables
            export XDG_RUNTIME_DIR=''${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
            
            # Ensure DBus session is available for keyring
            ${if pkgs.stdenv.isLinux then ''
              if [ -z "$DBUS_SESSION_BUS_ADDRESS" ]; then
                eval $(dbus-launch --sh-syntax)
                export DBUS_SESSION_BUS_ADDRESS
              fi
              
              # Start gnome-keyring-daemon if not running
              if ! pgrep -x "gnome-keyring-d" > /dev/null; then
                eval $(gnome-keyring-daemon --start --components=secrets)
                export GNOME_KEYRING_CONTROL
                export SSH_AUTH_SOCK
              fi
            '' else ""}
          '';
        };

      }
    );
}
