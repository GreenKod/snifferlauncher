# =============================================================================
#  nix/rust.nix — Rust stable toolchain via rust-overlay
# =============================================================================
#
#  Returns the Rust toolchain derivation with:
#    • rust-analyzer, clippy, rustfmt, rust-src
#    • Cross-compilation targets for Android, Windows (MinGW), and Linux musl
#
#  Requires: rust-overlay to be applied to nixpkgs beforehand (see shell.nix).
#
{ pkgs }:

pkgs.rust-bin.stable.latest.default.override {
  extensions = [
    "rust-src"       # required by rust-analyzer for jump-to-definition
    "rust-analyzer"  # LSP server
    "clippy"         # linter
    "rustfmt"        # formatter
  ];

  targets = [
    # Android
    "aarch64-linux-android"
    "armv7-linux-androideabi"
    "i686-linux-android"
    "x86_64-linux-android"
    # Windows cross-compilation via MinGW-w64
    "x86_64-pc-windows-gnu"
    # Linux (native + static musl)
    "x86_64-unknown-linux-gnu"
    "x86_64-unknown-linux-musl"
  ];
}
