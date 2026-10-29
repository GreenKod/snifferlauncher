## Description
<!-- Provide a clear description of the problem, relevant context, and what this PR accomplishes. -->

## Type of Change
<!-- Mark the relevant options with an [x] -->
- [ ] Bug fix (non-breaking change fixing a defect)
- [ ] New feature (non-breaking change adding functionality)
- [ ] Security hardening / vulnerability remediation
- [ ] Performance optimization
- [ ] Refactoring (no functional changes)
- [ ] Breaking change (fix or feature that alters existing contracts)
- [ ] Documentation update

## Affected Components
<!-- Mark all launcher components touched by this PR -->
- [ ] `src/` (Launcher host entry points `lib.rs` / `main.rs`)
- [ ] `.plugins/default_ui` (Home screen & App Drawer plugin)
- [ ] `.plugins/dock` (Dock & long-press preview plugin)
- [ ] `.plugins/clock_widget` / `.plugins/devkit_hud` / `.plugins/logger` (Widgets & diagnostics)
- [ ] `.plugins/framework` (`SnifferUI` runtime copy & SHA-256 manifest checksums)
- [ ] `android/` (`AndroidManifest.xml`, resources, icons)
- [ ] `tests/launcher_ui_tests.rs` (Launcher UI, manifest & plugin integration tests)
- [ ] CI/CD & Build scripts (`.github/workflows`, keystores, packaging)

## Related Issues
<!-- Link related issues, e.g. Fixes #123, Relates to #456 -->
Fixes #

## Pre-flight Checklist
<!-- Please ensure all checks pass before submitting -->
- [ ] `cargo test --locked --workspace --all-features` passes locally.
- [ ] `cargo clippy --locked --workspace --all-targets -- -D warnings` reports 0 warnings.
- [ ] `cargo fmt --all --check` produces no diff.
- [ ] If modifying `.plugins/*` or `.plugins/framework/*`, SHA-256 digests in `manifest.json` have been updated.
- [ ] If changing Android release configuration, `debuggable=false` and release keystore signing pass `verify_release_apk.py`.
