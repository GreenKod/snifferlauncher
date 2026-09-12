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
<!-- Mark all crates / components touched by this PR -->
- [ ] `sniffer_core` (Data types, AST, layout engine, math)
- [ ] `sniffer_plugin` (QuickJS runtime, host bridge, permissions, manifest validation)
- [ ] `sniffer_pkg` (C ABI dynamic packages, package manager, integrity)
- [ ] `sniffer_render` (Glow/OpenGL backend, font atlas, hitbox, culling)
- [ ] `sniffer_platform_android` (Android entry point, JNI bridge, APK lifecycle)
- [ ] `sniffer_platform_desktop` (Desktop winit/glutin runner, preview)
- [ ] `.plugins/*` (JavaScript UI plugins, widgets, framework)
- [ ] CI/CD & Build scripts (`.github/workflows`, keystores, packaging)

## Related Issues
<!-- Link related issues, e.g. Fixes #123, Relates to #456 -->
Fixes #

## Pre-flight Checklist
<!-- Please ensure all checks pass before submitting -->
- [ ] `cargo test --locked --workspace --all-features` passes locally.
- [ ] `cargo clippy --locked --workspace --all-targets -- -D warnings` reports 0 warnings.
- [ ] `cargo fmt --all --check` produces no diff.
- [ ] If changing plugin APIs, security gating (permissions / caller ID) has been verified.
- [ ] If changing native dynamic packages, C ABI compatibility (ADR 001) has been preserved.
- [ ] If changing Android release configuration, `debuggable=false` and release keystore signing pass `verify_release_apk.py`.
