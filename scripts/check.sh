#!/usr/bin/env bash
# ==============================================================================
# SnifferLauncher Local CI Pre-flight Quality Verification Script (Bash)
# ==============================================================================
set -euo pipefail

echo -e "\n\033[1;36m========================================================\033[0m"
echo -e "\033[1;36m 🚀 SnifferLauncher Local CI Pre-flight Quality Check\033[0m"
echo -e "\033[1;36m========================================================\033[0m\n"

echo -e "\033[1;34m[1/5] Checking Code Formatting (cargo fmt)...\033[0m"
cargo fmt --all --check

echo -e "\033[1;34m[2/5] Checking Workspace Compilation (cargo check)...\033[0m"
cargo check --locked --workspace --all-targets --all-features
cargo check --locked --workspace --no-default-features

echo -e "\033[1;34m[3/5] Running Strict Linter (cargo clippy)...\033[0m"
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings

echo -e "\033[1;34m[4/5] Running Unit & Integration Tests (cargo test)...\033[0m"
cargo test --locked --workspace --all-features

echo -e "\033[1;34m[5/5] Checking for Unused Dependencies (cargo machete)...\033[0m"
if command -v cargo-machete &> /dev/null; then
    cargo machete
else
    echo -e "\033[0;33mℹ cargo-machete not found, skipping.\033[0m"
fi

echo -e "\n\033[1;32m🎉 ALL QUALITY GATES PASSED! Ready to push safely. 🎉\033[0m\n"
