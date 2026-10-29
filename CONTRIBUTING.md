# Contributing to SnifferLauncher

Thank you for your interest in contributing to SnifferLauncher. SnifferLauncher is a native Android and Desktop launcher built on top of [SnifferEngine](https://github.com/GreenKod/snifferengine) using declarative JavaScript plugins (`SnifferUI`).

---

## Code of Conduct & Style Standards

- **Professional Tone**: Maintain a formal, technical, and objective communication style across issues, pull requests, and commit messages.
- **Engine vs. Launcher Separation**:
  - Changes to core layout, rendering, 120Hz scroll/spring physics, QuickJS sandboxing, or `SnifferUI` framework internals belong in the sibling **`snifferengine`** repository (`../snifferengine`).
  - Changes to launcher screens (`default_ui`, `dock`, `clock_widget`), Android launcher manifest/resources (`android/`), and launcher UI tests (`tests/launcher_ui_tests.rs`) belong in **`snifferlauncher`**.

---

## Development Setup

### Workspace Layout

Ensure `snifferengine` is checked out as a sibling directory (`../snifferengine`):

```text
sniffers/
├── snifferengine/
└── snifferlauncher/
```

### Running the Desktop Simulator

```bash
cargo run --bin snifferlauncher-desktop
```

---

## Quality Checklist

Before submitting a Pull Request, verify that all checks pass cleanly:

```bash
# 1. Run all launcher UI and manifest tests
cargo test --locked --workspace --all-features

# 2. Check for linter warnings (must report 0 warnings)
cargo clippy --locked --workspace --all-targets -- -D warnings

# 3. Verify code formatting
cargo fmt --all --check
```

If you modify any JavaScript file in `.plugins/*` or sync `.plugins/framework/*`, update the corresponding SHA-256 checksums in the plugin `manifest.json` files and verify with `cargo test --locked --test launcher_ui_tests`.
