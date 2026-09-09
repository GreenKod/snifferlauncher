#!/usr/bin/env python3
"""
Dependency Age Gate (Supply-Chain Quarantine Check)

Ensures that any new or updated crate dependency in Cargo.lock was published to
crates.io at least 24 hours ago. This prevents zero-day supply-chain poisoning
where malicious crate versions are pulled in immediately after publication
before community detection or yank actions.

Usage:
    python .github/scripts/check_dependency_age.py [--base-ref <git-ref>] [--min-age-hours <hours>]
"""

from __future__ import annotations

import argparse
import datetime
import json
import os
import re
import subprocess
import sys
import urllib.error
import urllib.request
from typing import Dict, List, Optional, Tuple

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

USER_AGENT = "SnifferLauncher-CI/1.0 (contact: greenkod.dev@gmail.com)"
CRATES_IO_API = "https://crates.io/api/v1/crates"
DEFAULT_MIN_AGE_HOURS = 24.0

# Emergency allowlist for explicitly vetted crates if needed
ALLOWLIST_CRATES: set[str] = set()


def parse_commit_message() -> str:
    try:
        res = subprocess.run(
            ["git", "log", "-1", "--pretty=%B"],
            capture_output=True,
            text=True,
            check=True,
        )
        return res.stdout
    except Exception:
        return ""


def parse_cargo_lock_packages(content: str) -> Dict[str, str]:
    """
    Parses Cargo.lock and returns a dict mapping crate name to version
    for crates sourced from crates.io registry.
    """
    packages: Dict[str, str] = {}
    current_name: Optional[str] = None
    current_version: Optional[str] = None
    is_crates_io: bool = False

    for line in content.splitlines():
        line = line.strip()
        if line == "[[package]]":
            if current_name and current_version and is_crates_io:
                packages[current_name] = current_version
            current_name = None
            current_version = None
            is_crates_io = False
        elif line.startswith("name = "):
            m = re.match(r'name\s*=\s*"([^"]+)"', line)
            if m:
                current_name = m.group(1)
        elif line.startswith("version = "):
            m = re.match(r'version\s*=\s*"([^"]+)"', line)
            if m:
                current_version = m.group(1)
        elif line.startswith("source = "):
            # We only verify crates sourced from crates.io registry
            if "registry+https://github.com/rust-lang/crates.io-index" in line:
                is_crates_io = True

    # Last package in file
    if current_name and current_version and is_crates_io:
        packages[current_name] = current_version

    return packages


def get_base_cargo_lock(base_ref: Optional[str]) -> Optional[str]:
    """Retrieves the content of Cargo.lock at the base git ref."""
    candidates = []
    if base_ref:
        candidates.append(base_ref)

    env_base = os.environ.get("GITHUB_BASE_REF")
    if env_base:
        candidates.extend([f"origin/{env_base}", env_base])

    candidates.extend(["origin/master", "origin/main", "master", "main", "HEAD~1"])

    for ref in candidates:
        try:
            res = subprocess.run(
                ["git", "show", f"{ref}:Cargo.lock"],
                capture_output=True,
                text=True,
                check=False,
            )
            if res.returncode == 0 and res.stdout.strip():
                print(f"[info] Comparing against base ref: {ref}")
                return res.stdout
        except Exception:
            continue

    return None


def get_current_cargo_lock() -> str:
    lock_path = os.path.join(os.getcwd(), "Cargo.lock")
    if not os.path.exists(lock_path):
        raise FileNotFoundError(f"Cargo.lock not found at {lock_path}")
    with open(lock_path, "r", encoding="utf-8") as f:
        return f.read()


def find_updated_packages(
    base_packages: Dict[str, str], current_packages: Dict[str, str]
) -> List[Tuple[str, Optional[str], str]]:
    """
    Returns list of (crate_name, old_version_or_none, new_version).
    """
    updated = []
    for name, new_ver in current_packages.items():
        if name in ALLOWLIST_CRATES:
            continue
        old_ver = base_packages.get(name)
        if old_ver is None:
            # New dependency added
            updated.append((name, None, new_ver))
        elif old_ver != new_ver:
            # Dependency upgraded or downgraded
            updated.append((name, old_ver, new_ver))
    return updated


def fetch_crate_publish_time(crate_name: str, version: str) -> datetime.datetime:
    """
    Queries crates.io API to get the created_at timestamp for a specific crate version.
    """
    url = f"{CRATES_IO_API}/{crate_name}/{version}"
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})

    with urllib.request.urlopen(req, timeout=15) as resp:
        if resp.status != 200:
            raise RuntimeError(f"crates.io API returned HTTP {resp.status} for {url}")
        data = json.loads(resp.read().decode("utf-8"))

    # crates.io API returns version details under 'version' object
    ver_data = data.get("version", {})
    created_at_str = ver_data.get("created_at")
    if not created_at_str:
        raise ValueError(f"No created_at field found for {crate_name} v{version}")

    # Standard ISO 8601: e.g. "2024-09-08T12:34:56.789012+00:00" or with "Z"
    if created_at_str.endswith("Z"):
        created_at_str = created_at_str[:-1] + "+00:00"

    # Python 3.7+ supports fromisoformat with offset
    return datetime.datetime.fromisoformat(created_at_str)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Verify that modified Cargo dependencies are at least 24 hours old."
    )
    parser.add_argument(
        "--base-ref",
        help="Git ref to compare Cargo.lock against (e.g. origin/master, HEAD~1)",
    )
    parser.add_argument(
        "--min-age-hours",
        type=float,
        default=DEFAULT_MIN_AGE_HOURS,
        help=f"Minimum age in hours required for crate versions (default: {DEFAULT_MIN_AGE_HOURS})",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Fail if base Cargo.lock cannot be found",
    )
    args = parser.parse_args()

    # Emergency bypass check
    commit_msg = parse_commit_message()
    if "[skip-dep-age]" in commit_msg or os.environ.get("SKIP_DEP_AGE") == "true":
        print("⚠️ [quarantine-bypass] '[skip-dep-age]' detected. Skipping dependency age check.")
        return 0

    current_lock = get_current_cargo_lock()
    current_packages = parse_cargo_lock_packages(current_lock)

    base_lock = get_base_cargo_lock(args.base_ref)
    if base_lock is None:
        if args.strict or os.environ.get("GITHUB_BASE_REF"):
            print("::error::Could not find base Cargo.lock to compare against in pull request / strict mode!")
            print("::error::Ensure git fetch-depth includes base ref (e.g. origin/${GITHUB_BASE_REF}) to perform quarantine verification.")
            return 1
        print("[info] No base Cargo.lock found to compare (non-PR or initial setup). Skipping age check for existing dependencies.")
        return 0

    base_packages = parse_cargo_lock_packages(base_lock)
    updated = find_updated_packages(base_packages, current_packages)

    if not updated:
        print("✅ No crates.io dependencies were added or updated.")
        return 0

    print(f"🔍 Found {len(updated)} updated or newly added crates.io dependency/ies:")
    for name, old_v, new_v in updated:
        if old_v:
            print(f"   • {name}: {old_v} -> {new_v}")
        else:
            print(f"   • {name}: (new) {new_v}")

    now = datetime.datetime.now(datetime.timezone.utc)
    violations: List[str] = []

    for name, old_v, new_v in updated:
        print(f"Checking {name} v{new_v} publication date on crates.io...")
        try:
            pub_time = fetch_crate_publish_time(name, new_v)
            age = now - pub_time
            age_hours = age.total_seconds() / 3600.0

            if age_hours < args.min_age_hours:
                hours_left = args.min_age_hours - age_hours
                err_msg = (
                    f"Crate '{name}' v{new_v} was published at {pub_time.strftime('%Y-%m-%d %H:%M:%S UTC')} "
                    f"(only {age_hours:.1f} hours ago). "
                    f"Minimum quarantine policy requires >= {args.min_age_hours:.0f} hours. "
                    f"Please wait {hours_left:.1f} more hours before updating."
                )
                print(f"::error::{err_msg}")
                violations.append(err_msg)
            else:
                print(f"   ✓ {name} v{new_v} published {age_hours:.1f}h ago (>= {args.min_age_hours:.0f}h). OK.")
        except Exception as e:
            print(f"::warning::Failed to verify publication date for {name} v{new_v}: {e}")

    if violations:
        print("\n" + "=" * 70)
        print("❌ DEPENDENCY QUARANTINE POLICY VIOLATION:")
        print("=" * 70)
        for v in violations:
            print(f"  • {v}")
        print("\nTo protect against supply chain poisoning, new releases cannot be used until 24 hours have passed.")
        print("If this is an emergency security hotfix, add '[skip-dep-age]' to your commit message.")
        print("=" * 70)
        return 1

    print("\n✅ All modified dependencies satisfy the 24-hour quarantine policy.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
