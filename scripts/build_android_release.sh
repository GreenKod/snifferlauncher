#!/usr/bin/env bash
# Android Release Builder (Linux / macOS)
# Builds anonymous Android release APK with compiled native .so and packed assets.

set -e

HOME_DIR="${HOME}"
PROJECT_DIR="$(pwd)"

export RUSTFLAGS="--remap-path-prefix ${PROJECT_DIR}=/src --remap-path-prefix ${HOME_DIR}/.rustup/toolchains=/rust --remap-path-prefix ${HOME_DIR}/.rustup=/rust --remap-path-prefix ${HOME_DIR}/.cargo=/cargo --remap-path-prefix ${HOME_DIR}=/anon --remap-path-prefix .=/src -C strip=symbols"
export CFLAGS="-ffile-prefix-map=${PROJECT_DIR}=/src -ffile-prefix-map=${HOME_DIR}=/anon -ffile-prefix-map=.=/src"
export CXXFLAGS="-ffile-prefix-map=${PROJECT_DIR}=/src -ffile-prefix-map=${HOME_DIR}=/anon -ffile-prefix-map=.=/src"

if [ -z "$SYSROOT" ]; then
    NDK_PATH="${ANDROID_NDK_HOME:-/opt/android-ndk}"
    if [ -d "$NDK_PATH/toolchains/llvm/prebuilt/linux-x86_64/sysroot" ]; then
        export SYSROOT="$NDK_PATH/toolchains/llvm/prebuilt/linux-x86_64/sysroot"
    fi
fi
if [ -n "$SYSROOT" ] && [ -z "$BINDGEN_EXTRA_CLANG_ARGS" ]; then
    export BINDGEN_EXTRA_CLANG_ARGS="--target=aarch64-linux-android30 --sysroot=$SYSROOT"
fi

echo "Building anonymous Android ARM64 release binary..."
cargo apk build --target aarch64-linux-android --lib --release

if command -v pwsh >/dev/null 2>&1 && [ -f scratch/pack_apk.ps1 ]; then
    echo "Injecting .so library and packing assets into APK..."
    pwsh -File scratch/pack_apk.ps1
fi

UNSIGNED_APK="target/release/apk/snifferlauncher-android-arm64-release-unsigned.apk"
SIGNED_APK="target/release/apk/snifferlauncher-android-arm64-release-signed.apk"

echo "=========================================="
echo "Build Complete!"
echo "Unsigned APK: ${UNSIGNED_APK}"
if [ -f "${SIGNED_APK}" ]; then
    echo "Signed APK:   ${SIGNED_APK}"
fi
echo "=========================================="
