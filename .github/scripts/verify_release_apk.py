#!/usr/bin/env python3
"""
Release APK Security & Integrity Policy Verifier

Verifies built Android APKs against production release security policies:
1. Rejects any APK where `android:debuggable` is set to `true`.
2. Inspects APK signature using `apksigner` (warns or fails on debug keystores).
3. Verifies required architecture native libraries (e.g., `libsnifferlauncher.so`).
4. Audits requested permissions against expected launcher capabilities.

Usage:
    python3 .github/scripts/verify_release_apk.py <path_to_apk_or_dir> [--strict-signing]
"""

from __future__ import annotations

import argparse
import glob
import os
import re
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path
from typing import List, Optional, Tuple

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

# Allowed permissions for SnifferLauncher
ALLOWED_PERMISSIONS = {
    "android.permission.QUERY_ALL_PACKAGES",
    "android.permission.READ_EXTERNAL_STORAGE",
    "android.permission.READ_MEDIA_IMAGES",
    "android.permission.SET_WALLPAPER",
}


def find_android_tool(tool_name: str) -> Optional[str]:
    """Finds an Android SDK build-tool (aapt, apksigner, etc.) in PATH or ANDROID_HOME."""
    in_path = shutil.which(tool_name)
    if in_path:
        return in_path

    search_dirs = []
    for env_var in ("ANDROID_HOME", "ANDROID_SDK_ROOT", "ANDROID_NDK_HOME"):
        val = os.environ.get(env_var)
        if val:
            search_dirs.append(Path(val))

    for base in search_dirs:
        # Check build-tools directory
        build_tools = base / "build-tools"
        if build_tools.is_dir():
            for version_dir in sorted(build_tools.iterdir(), reverse=True):
                candidate = version_dir / tool_name
                if candidate.is_file() and os.access(candidate, os.X_OK):
                    return str(candidate)

    return None


def verify_debuggable(apk_path: Path, aapt_bin: Optional[str]) -> Tuple[bool, str]:
    """
    Checks whether android:debuggable is true.
    Returns (is_secure, detail_message).
    """
    if aapt_bin:
        try:
            res = subprocess.run(
                [aapt_bin, "dump", "xmltree", str(apk_path), "AndroidManifest.xml"],
                capture_output=True,
                text=True,
                check=True,
            )
            output = res.stdout

            # Match android:debuggable attribute
            for line in output.splitlines():
                if "android:debuggable" in line:
                    if "0xffffffff" in line or "=0x1" in line or "=true" in line.lower():
                        return False, f"FAILED: android:debuggable is set to TRUE in manifest:\n  {line.strip()}"
                    elif "0x0" in line or "=false" in line.lower():
                        return True, f"PASSED: android:debuggable is explicitly FALSE:\n  {line.strip()}"

            # If attribute is omitted, Android defaults to debuggable=false
            return True, "PASSED: android:debuggable attribute is omitted (defaults to FALSE in release mode)."
        except Exception as e:
            return False, f"Error running aapt dump xmltree: {e}"

    # Fallback to direct binary XML inspection inside ZIP
    try:
        with zipfile.ZipFile(apk_path, "r") as z:
            manifest_bytes = z.read("AndroidManifest.xml")
            # In Android binary XML, string "debuggable" will be present in the string pool.
            # If debuggable=true, the attribute value entry has boolean flag 0xFFFFFFFF
            if b"debuggable" in manifest_bytes:
                # Basic heuristic check for 0xffffffff boolean value following debuggable string
                idx = manifest_bytes.find(b"debuggable")
                context = manifest_bytes[idx : idx + 64]
                if b"\xff\xff\xff\xff" in context:
                    return False, "FAILED: Binary XML heuristic indicates debuggable=true."
                return True, "PASSED: Binary XML contains debuggable attribute with false/safe value."
            return True, "PASSED: debuggable string not found in manifest (defaults to false)."
    except Exception as e:
        return False, f"Error inspecting APK ZIP directly: {e}"


def verify_signature(
    apk_path: Path, apksigner_bin: Optional[str], strict_signing: bool
) -> Tuple[bool, str]:
    """
    Inspects APK signature and certificate identity.
    Returns (is_valid, detail_message).
    """
    if not apksigner_bin:
        # Fallback: check META-INF for signature files
        with zipfile.ZipFile(apk_path, "r") as z:
            sig_files = [f for f in z.namelist() if f.startswith("META-INF/") and (f.endswith(".RSA") or f.endswith(".EC") or f.endswith(".DSA") or f.endswith(".SF"))]
            if sig_files:
                return True, f"PASSED: Found APK signature files in META-INF ({', '.join(sig_files)})."
            return False, "FAILED: No signature files found in META-INF directory."

    try:
        res = subprocess.run(
            [apksigner_bin, "verify", "--verbose", "--print-certs", str(apk_path)],
            capture_output=True,
            text=True,
        )

        if res.returncode != 0:
            return False, f"FAILED: apksigner verification failed:\n{res.stderr or res.stdout}"

        output = res.stdout
        is_debug_cert = False
        cert_subjects = []

        for line in output.splitlines():
            line_s = line.strip()
            if line_s.startswith("Signer #") or "certificate DN:" in line_s or "Subject:" in line_s:
                cert_subjects.append(line_s)
                if "Android Debug" in line_s or "androiddebugkey" in line_s:
                    is_debug_cert = True

        cert_info = " | ".join(cert_subjects) if cert_subjects else "Verified signature"

        if is_debug_cert:
            msg = f"WARNING: APK is signed with a DEBUG certificate: {cert_info}"
            if strict_signing:
                return False, f"FAILED (strict signing): {msg}"
            return True, msg

        return True, f"PASSED: Valid production/release certificate detected ({cert_info})."
    except Exception as e:
        return False, f"Error running apksigner: {e}"


def verify_native_libraries(apk_path: Path) -> Tuple[bool, str]:
    """Verifies that native .so binaries are present."""
    try:
        with zipfile.ZipFile(apk_path, "r") as z:
            so_files = [f for f in z.namelist() if f.startswith("lib/") and f.endswith(".so")]
            if not so_files:
                return False, "FAILED: No native .so libraries found in APK."
            
            has_launcher_lib = any("libsnifferlauncher.so" in f for f in so_files)
            if not has_launcher_lib:
                return False, f"FAILED: libsnifferlauncher.so not found among native libraries: {so_files}"
            
            return True, f"PASSED: Native libraries verified ({', '.join(so_files)})."
    except Exception as e:
        return False, f"Error inspecting native libraries in APK: {e}"


def verify_permissions(apk_path: Path, aapt_bin: Optional[str]) -> Tuple[bool, str]:
    """Verifies permissions requested by the APK."""
    if not aapt_bin:
        return True, "SKIPPED: aapt not found to dump permissions."

    try:
        res = subprocess.run(
            [aapt_bin, "dump", "permissions", str(apk_path)],
            capture_output=True,
            text=True,
            check=True,
        )
        
        found_permissions = set()
        for line in res.stdout.splitlines():
            m = re.search(r"uses-permission:\s*name='([^']+)'", line)
            if m:
                found_permissions.add(m.group(1))

        unexpected = found_permissions - ALLOWED_PERMISSIONS
        if unexpected:
            return False, f"FAILED: APK declares unexpected permissions: {unexpected}"

        return True, f"PASSED: Permissions match policy ({', '.join(sorted(found_permissions))})."
    except Exception as e:
        return False, f"Error checking permissions with aapt: {e}"


def verify_single_apk(apk_path: Path, aapt_bin: Optional[str], apksigner_bin: Optional[str], strict_signing: bool) -> bool:
    print(f"\n{'='*70}")
    print(f"📦 Verifying Release Security Policy: {apk_path.name}")
    print(f"   Path: {apk_path}")
    print(f"   Size: {apk_path.stat().st_size:,} bytes")
    print(f"{'='*70}")

    all_passed = True

    # 1. Debuggable check (P0 Blocker)
    ok, msg = verify_debuggable(apk_path, aapt_bin)
    print(f"\n[1] Debuggable Status Check (P0 Blocker):")
    print(f"    {msg}")
    if not ok:
        all_passed = False
        print("    ::error title=Release Policy Violation::android:debuggable must be FALSE in release APK!")

    # 2. Signature check
    ok, msg = verify_signature(apk_path, apksigner_bin, strict_signing)
    print(f"\n[2] Signature & Certificate Check:")
    print(f"    {msg}")
    if not ok:
        all_passed = False
        print("    ::error title=Release Signature Failure::APK signature verification failed!")

    # 3. Native libraries check
    ok, msg = verify_native_libraries(apk_path)
    print(f"\n[3] Native Architecture Libraries Check:")
    print(f"    {msg}")
    if not ok:
        all_passed = False
        print("    ::error title=Missing Native Libraries::Required .so files missing from APK!")

    # 4. Permissions check
    ok, msg = verify_permissions(apk_path, aapt_bin)
    print(f"\n[4] Android Permissions Audit:")
    print(f"    {msg}")
    if not ok:
        all_passed = False
        print("    ::error title=Permission Violation::Unexpected permissions declared in APK!")

    return all_passed


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify Android Release APK Security Policy")
    parser.add_argument("target", help="Path to APK file or directory containing APKs")
    parser.add_argument("--strict-signing", action="store_true", help="Fail if signed with a debug certificate")
    args = parser.parse_args()

    target_path = Path(args.target)
    apks: List[Path] = []

    if target_path.is_file() and target_path.suffix == ".apk":
        apks.append(target_path)
    elif target_path.is_dir():
        apks.extend(target_path.rglob("*.apk"))
    else:
        print(f"Error: Target path '{target_path}' not found or is not an APK.")
        return 1

    if not apks:
        print(f"No APK files found at '{target_path}'.")
        return 1

    aapt_bin = find_android_tool("aapt")
    apksigner_bin = find_android_tool("apksigner")

    print(f"Resolved Tools:")
    print(f"  - aapt:      {aapt_bin or 'NOT FOUND (using fallback zip parser)'}")
    print(f"  - apksigner: {apksigner_bin or 'NOT FOUND (using fallback cert inspection)'}")

    total_failed = 0
    for apk in apks:
        passed = verify_single_apk(apk, aapt_bin, apksigner_bin, args.strict_signing)
        if not passed:
            total_failed += 1

    print(f"\n{'='*70}")
    if total_failed == 0:
        print(f"✅ All {len(apks)} APK file(s) PASSED release security verification!")
        return 0
    else:
        print(f"❌ {total_failed} of {len(apks)} APK file(s) FAILED release security verification!")
        return 1


if __name__ == "__main__":
    sys.exit(main())
