# Contributing to SnifferLauncher

Thank you for your interest in contributing to SnifferLauncher. This project is a modular, high-performance Android and Desktop launcher written in Rust with a QuickJS plugin runtime and a stable C ABI native package architecture.

To maintain code quality, stability, and security, all contributors must adhere to the guidelines outlined below.

---

## Code of Conduct & Style Standards

- **Professional Tone**: Maintain a formal, technical, and objective communication style across issues, pull requests, and commit messages. Do not use emojis in commit messages, pull request titles, or formal project documentation.
- **Code Quality**: Code must compile without warnings on the default toolchain and pass all automated quality gates.
- **Security First**: SnifferLauncher enforces strict sandboxing between UI plugins, native dynamic packages, and host platform APIs. Never introduce unverified host bridges or bypass permission checks.

---

## Development Setup

### Prerequisites

1. **Rust Toolchain**: Rust 1.85+ (supporting Rust 2024 edition).
   ```bash
   rustup update stable
   ```
2. **Desktop Preview**:
   - Linux: `libx11-dev`, `libxcursor-dev`, `libxrandr-dev`, `libxi-dev`, `libgl1-mesa-dev`
   - Windows / macOS: Standard graphics drivers supporting OpenGL 3.3+.
3. **Android Build (Optional for Desktop contributors)**:
   - Android NDK r25b or newer.
   - `cargo-apk` or `cargo-ndk`.

### Running the Desktop Simulator

```bash
# Run the desktop preview environment
cargo run --bin snifferlauncher_desktop
```

---

## Repository Architecture

The workspace is organized into modular crates:

- **`crates/sniffer_core`**: Core layout engine, UI AST representations, math types, and shared event definitions.
- **`crates/sniffer_plugin`**: QuickJS JavaScript runtime, Host Platform Bridge, capability permission manager, and manifest validation.
- **`crates/sniffer_pkg`**: Versioned C ABI dynamic package loader (ADR 001), zero-dependency SHA-256 package verification, and package registry.
- **`crates/sniffer_render`**: Glow-based OpenGL renderer, font atlas caching, culling, and hitbox detection.
- **`crates/sniffer_platform_desktop`**: Desktop preview harness using `winit` and `glutin`.
- **`crates/sniffer_platform_android`**: Android Native Activity lifecycle, JNI bindings, and Android Host Bridge.
- **`.plugins/*`**: Built-in vanilla JavaScript plugins and widgets evaluated inside the sandboxed QuickJS runtime.

---

## Quality Checklist

Before submitting a Pull Request, verify that all three commands pass cleanly:

```bash
# 1. Run all workspace tests
cargo test --locked --workspace --all-features

# 2. Check for linter warnings (must report 0 warnings)
cargo clippy --locked --workspace --all-targets -- -D warnings

# 3. Verify code formatting
cargo fmt --all --check
```

---

## Security Guidelines for Contributions

When modifying or adding features, observe the following constraints:

1. **Host Bridge Authorization**:
   - Every platform method in `HostPlatformBridge` requires a `caller_id: &str`.
   - Sensitive platform actions (e.g. launching apps, requesting permissions) must be gated by `permission_manager::has_launch_permission()`.
   - Administrative launcher operations (e.g. setting default home) must be restricted to `is_master == true`.
2. **Plugin Sandboxing**:
   - JavaScript execution is subject to a 500 ms execution deadline (`DeadlineGuard`) and configurable memory limit (`maxMemoryMb`). Never block the worker thread synchronously.
   - All script and preload files must have their SHA-256 checksums declared in `manifest.json`.
3. **Native Dynamic Packages**:
   - Dynamic `.so` / `.dll` packages must conform to ADR 001: `"SNIF"` magic bytes, `abi_version == 1`, and declare a valid SHA-256 hash in `package.json`.

---

## Submitting Pull Requests

1. Create a descriptive feature or bugfix branch from `master` (e.g., `feat/app-search`, `fix/render-culling`).
2. Fill out the Pull Request template completely, checking off the pre-flight checklist.
3. Reference any related issues (e.g., `Fixes #123`).
