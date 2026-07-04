#!/bin/bash

# SnifferLauncher Release Builder
# This script compiles the project and prepares a ready-to-distribute ZIP package.

set -e

echo "🔨 Compiling SnifferLauncher desktop version (release mode)..."
cargo build --release

echo "📦 Preparing the dist folder for packaging..."
rm -rf dist/
mkdir -p dist/snifferlauncher-bundle

echo "📂 Copying compiled binary and plugins..."
cp target/release/snifferlauncher dist/snifferlauncher-bundle/

# Include .plugins directory if it exists
if [ -d ".plugins" ]; then
    cp -r .plugins dist/snifferlauncher-bundle/
fi

# Include assets directory if it exists (e.g., test.png)
if [ -d "assets" ]; then
    # No need to include Android-specific plugins if they are copied
    mkdir -p dist/snifferlauncher-bundle/assets
    cp -r assets/* dist/snifferlauncher-bundle/assets/ || true
    rm -rf dist/snifferlauncher-bundle/assets/plugins || true
    rm -rf dist/snifferlauncher-bundle/assets/ui || true
fi

echo "🗜️ Creating ZIP file..."
cd dist
zip -r snifferlauncher-linux-amd64.zip snifferlauncher-bundle/
cd ..

echo "✅ Distribution package is ready: dist/snifferlauncher-linux-amd64.zip"
