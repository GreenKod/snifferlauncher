#!/usr/bin/env bash
# ==============================================================================
# SnifferLauncher Development Mode Toggle Script (onDevelop.sh)
# ==============================================================================
# Toggles sniffer_engine dependency in Cargo.toml between:
#   - Git mode (default):  git = "https://github.com/GreenKod/snifferengine.git"
#   - Local Dev mode:      path = "../snifferengine/crates/sniffer_engine"
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -f "$SCRIPT_DIR/Cargo.toml" ]; then
    CARGO_TOML="$SCRIPT_DIR/Cargo.toml"
elif [ -f "$SCRIPT_DIR/snifferlauncher/Cargo.toml" ]; then
    CARGO_TOML="$SCRIPT_DIR/snifferlauncher/Cargo.toml"
else
    echo "❌ Cargo.toml bulunamadı!"
    exit 1
fi

GIT_ENGINE='sniffer_engine = { git = "https://github.com/GreenKod/snifferengine.git" }'
LOCAL_ENGINE='sniffer_engine = { path = "../snifferengine/crates/sniffer_engine" }'

GIT_PATCH='android-activity = { git = "https://github.com/GreenKod/snifferengine.git" }'
LOCAL_PATCH='android-activity = { path = "../snifferengine/patches/android-activity" }'

if grep -qF "$GIT_ENGINE" "$CARGO_TOML"; then
    sed -i "s|$GIT_ENGINE|$LOCAL_ENGINE|g" "$CARGO_TOML"
    sed -i "s|$GIT_PATCH|$LOCAL_PATCH|g" "$CARGO_TOML"
    echo -e "\033[1;32m🔧 [DEV MODU AKTİF]\033[0m Cargo.toml yerel '../snifferengine' klasörüne bağlandı."
elif grep -qF "$LOCAL_ENGINE" "$CARGO_TOML"; then
    sed -i "s|$LOCAL_ENGINE|$GIT_ENGINE|g" "$CARGO_TOML"
    sed -i "s|$LOCAL_PATCH|$GIT_PATCH|g" "$CARGO_TOML"
    echo -e "\033[1;36m🌐 [GIT MODU AKTİF]\033[0m Cargo.toml varsayılan GitHub ('GreenKod/snifferengine') adresine döndürüldü."
else
    echo "❌ Cargo.toml içinde beklenen sniffer_engine satırı bulunamadı!"
    exit 1
fi

