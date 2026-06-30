# =============================================================================
#  shell.nix — snifferlauncher development environment (entry point)
# =============================================================================
#
#  Usage:
#    nix-shell                         # enter the dev shell
#    nix-shell --run "cargo build"     # run a single command
#    direnv allow                      # auto-activate with direnv + nix-direnv
#
#  Module layout:
#    nix/android.nix           Android SDK + NDK composition
#    nix/rust.nix              Rust stable toolchain (via rust-overlay)
#    nix/vscode-extensions.nix VS Code extension ID list
#    nix/shell-hook.nix        Shell hook (env vars, .cargo/config.toml, …)
#
#  Prerequisites:
#    • nixpkgs channel must include rust-overlay (fetched automatically below)
#    • android_sdk.accept_license = true  (set in config below)
# =============================================================================

{ pkgs ? import <nixpkgs> {
    config = {
      android_sdk.accept_license = true;
      allowUnfree              = true;
    };
    overlays = [
      (import (builtins.fetchGit {
        url = "https://github.com/oxalica/rust-overlay.git";
        ref = "master";
      }))
    ];
  }
}:

let
  # ── Module imports ───────────────────────────────────────────────────────────
  android         = import ./nix/android.nix          { inherit pkgs; };
  rustToolchain   = import ./nix/rust.nix              { inherit pkgs; };
  vscodeExts      = import ./nix/vscode-extensions.nix;

in pkgs.mkShell {
  name = "snifferlauncher-dev";

  # ── Build inputs ─────────────────────────────────────────────────────────────
  buildInputs = [
    # Rust toolchain (rust-analyzer, clippy, rustfmt, rust-src + cross targets)
    rustToolchain

    # Android SDK + NDK (platform-tools, cmdline-tools, build-tools)
    android.sdk

    # Cross-compilation toolchains
    pkgs.pkgsCross.mingwW64.stdenv.cc   # x86_64-pc-windows-gnu  (MinGW-w64)
    pkgs.pkgsCross.musl64.stdenv.cc     # x86_64-unknown-linux-musl

    # Build system
    pkgs.pkg-config
    pkgs.cmake
    pkgs.ninja
    pkgs.clang
    pkgs.llvmPackages.bintools          # lld, llvm-ar, llvm-objcopy, …
    pkgs.lld

    # System libraries (Linux native targets)
    pkgs.openssl
    pkgs.openssl.dev
    pkgs.zlib
    pkgs.zlib.dev

    # SDL2 (required by the sdl2 Rust crate)
    pkgs.SDL2
    pkgs.SDL2.dev
    pkgs.SDL2_ttf
    pkgs.SDL2_image
    pkgs.SDL2_mixer

    # Java runtime (required by Android build-tools / Gradle)
    pkgs.jdk17
    pkgs.gradle

    # General utilities
    pkgs.git
    pkgs.curl
    pkgs.wget
    pkgs.which
    pkgs.file
  ];

  # ── Shell hook ───────────────────────────────────────────────────────────────
  shellHook = import ./nix/shell-hook.nix {
    inherit pkgs;
    androidSdk       = android.sdk;
    ndkVersion       = android.ndkVersion;
    vscodeExtensions = vscodeExts;
  };
}
