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
          ]
          ++ lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.AppKit
            pkgs.darwin.apple_sdk.frameworks.CoreText
            pkgs.darwin.apple_sdk.frameworks.WebKit
          ]
          ++ lib.optionals pkgs.stdenv.isLinux [
            # Added for Linux AppImage builds
            pkgs.appimagekit
            pkgs.imagemagick
            # Linux-specific graphics dependencies
            pkgs.mesa
            pkgs.libglvnd
            pkgs.xorg.libX11
            pkgs.xorg.libXcursor
            pkgs.xorg.libXi
            pkgs.xorg.libXrandr
            pkgs.xorg.libxcb
            pkgs.libxkbcommon
            pkgs.wayland
            pkgs.wayland-protocols
            pkgs.wayland-scanner
            pkgs.vulkan-loader
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
          shellHook = ''
            export LIBCLANG_PATH=${pkgs.libclang.lib}/lib/
            ${lib.optionalString pkgs.stdenv.isLinux ''
              # Add important Mesa paths (Linux only)
              export LIBGL_DRIVERS_PATH=${pkgs.mesa}/lib/dri
              export __EGL_VENDOR_LIBRARY_DIRS=${pkgs.mesa}/share/glvnd/egl_vendor.d/
              # Wayland specific environment variables
              export XDG_RUNTIME_DIR=''${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
            ''}
          '';
        };
      }
    );
}
