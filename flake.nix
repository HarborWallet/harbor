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

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs { inherit system overlays; };
        lib = pkgs.lib;
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        inputs = [
          rust
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
        ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
          pkgs.darwin.apple_sdk.frameworks.AppKit 
          pkgs.darwin.apple_sdk.frameworks.CoreText
          pkgs.darwin.apple_sdk.frameworks.WebKit
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
          buildInputs = with pkgs; [
            libxkbcommon
            libGL
            # WINIT_UNIX_BACKEND=wayland
            wayland
          ];
          LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
          shellHook = ''
            export LIBCLANG_PATH=${pkgs.libclang.lib}/lib/
            export LD_LIBRARY_PATH=${pkgs.openssl}/lib:$LD_LIBRARY_PATH
          '';
        };
      }
    );
}
