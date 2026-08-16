#!/usr/bin/env bash
# Run Desktop Preview (Linux / macOS)
# Builds and launches the Sniffer Launcher desktop preview window.

set -e

echo "=========================================="
echo "Starting Sniffer Launcher Desktop Preview..."
echo "=========================================="

DEBUG_PLUGINS_DIR="target/debug/.plugins"
if [ -d ".plugins" ]; then
    mkdir -p "target/debug"
    rm -rf "${DEBUG_PLUGINS_DIR}"
    cp -r ".plugins" "${DEBUG_PLUGINS_DIR}"
fi

cargo run --bin snifferlauncher-desktop
