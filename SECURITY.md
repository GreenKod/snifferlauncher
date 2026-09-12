# Security Policy

## Supported Versions

Security updates and vulnerability remediation are actively maintained for the following versions:

| Version / Branch | Supported          |
| ---------------- | ------------------ |
| `master`         | Yes                |
| Latest Release   | Yes                |
| Older Versions   | No                 |

---

## Reporting a Vulnerability

If you discover a security vulnerability in SnifferLauncher, please report it responsibly so it can be addressed safely.

### Private Disclosure Process

**Do not report potential security vulnerabilities through public GitHub issues or discussions.**

If you discover a vulnerability—especially one involving:
- QuickJS sandbox escape or arbitrary native code execution,
- Host Bridge authorization or permission gating bypass,
- Unchecked native dynamic package loading (C ABI boundary bypass),
- Denial of service or worker thread deadlock via malformed plugins,

Please submit a private report via GitHub Security Advisories:
[Submit a Security Advisory](https://github.com/GreenKod/snifferlauncher/security/advisories/new)

### What to Include in Your Report

To help reproduce and fix the issue quickly, please provide:
1. A clear description of the vulnerability and its potential impact.
2. Affected components (e.g., `sniffer_plugin`, `sniffer_pkg`, `HostPlatformBridge`).
3. Step-by-step reproduction instructions or a minimal proof-of-concept plugin/package.
4. Any suggested fix, if available.

### Review Process

Reports are reviewed as soon as possible. Once verified, a fix will be released and you will be credited in the release notes if desired.

---

## Security Architecture & Threat Model

SnifferLauncher enforces defense-in-depth across multiple system boundaries:

1. **JavaScript Plugin Sandbox**:
   - QuickJS runtimes execute with a hard per-plugin memory quota (`maxMemoryMb`, clamped to `2..=64` MiB) and an atomic watchdog deadline (`JS_TICK_DEADLINE_MS = 500ms`, `init = 2000ms`).
   - Plugin scripts must match their manifest-declared SHA-256 digests prior to execution.
2. **Host Bridge Capability Gating**:
   - Host platform operations (`launch_app`, `request_permissions`, `open_default_home_picker`) enforce caller identification (`caller_id`) and verify manifest permission grants (`has_launch_permission()`).
   - Administrative launcher operations are restricted strictly to master plugins (`is_master == true`).
3. **Native Dynamic Packages (ADR 001)**:
   - Shared libraries (`.so` / `.dll`) must adhere to the versioned C ABI contract (`"SNIF"` magic, `abi_version == 1`).
   - Binaries are validated against cryptographic SHA-256 hashes before dynamic linking (`dlopen` / `LoadLibrary`).
4. **Android Release Hardening**:
   - Release APK artifacts require `android:debuggable="false"` and verified release keystore signatures, enforced by automated CI policy gates (`verify_release_apk.py`).
