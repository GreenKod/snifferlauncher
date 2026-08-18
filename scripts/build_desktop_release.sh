#!/usr/bin/env bash
# Desktop Release Builder (Linux / macOS)
# Builds anonymous Desktop release binary with compiled executable and remapped paths.

set -e

HOME_DIR="${HOME}"
PROJECT_DIR="$(pwd)"

export RUSTFLAGS="--remap-path-prefix ${PROJECT_DIR}=/src --remap-path-prefix ${HOME_DIR}/.rustup/toolchains=/rust --remap-path-prefix ${HOME_DIR}/.rustup=/rust --remap-path-prefix ${HOME_DIR}/.cargo=/cargo --remap-path-prefix ${HOME_DIR}=/anon --remap-path-prefix .=/src -C strip=symbols"
export CFLAGS="-ffile-prefix-map=${PROJECT_DIR}=/src -ffile-prefix-map=${HOME_DIR}=/anon -ffile-prefix-map=.=/src"
export CXXFLAGS="-ffile-prefix-map=${PROJECT_DIR}=/src -ffile-prefix-map=${HOME_DIR}=/anon -ffile-prefix-map=.=/src"

echo "Building anonymous Desktop release binary..."
cargo build --release --bin snifferlauncher-desktop

OUTPUT_BIN="target/release/snifferlauncher-desktop"
RELEASE_PLUGINS_DIR="target/release/.plugins"

echo "Bundling .plugins directory and plugin manifests..."
if [ -d ".plugins" ]; then
    rm -rf "${RELEASE_PLUGINS_DIR}"
    cp -r ".plugins" "${RELEASE_PLUGINS_DIR}"
    echo "Successfully bundled .plugins into target/release/.plugins!"
fi

echo "=========================================="
echo "Build Complete!"
if [ -f "${OUTPUT_BIN}" ]; then
    echo "Binary:  ${OUTPUT_BIN}"
    echo "Plugins: ${RELEASE_PLUGINS_DIR}"
else
    echo "Binary created in target/release/"
fi
echo "=========================================="
