#!/usr/bin/env bash
# ==============================================================================
# SnifferLauncher Development Mode Toggle Script (scripts/onDevelop.sh)
# ==============================================================================
# Toggles sniffer_engine dependency in Cargo.toml (and syncs Cargo.lock) between:
#   - Git mode (default):  git = "https://github.com/GreenKod/snifferengine.git"
#   - Local Dev mode:      path = "../snifferengine/crates/sniffer_engine"
# Also ensures .githooks (pre-commit & pre-push) are active.
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LAUNCHER_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

if [ ! -f "$LAUNCHER_DIR/Cargo.toml" ]; then
    echo "❌ Cargo.toml bulunamadı: $LAUNCHER_DIR/Cargo.toml"
    exit 1
fi

# Ensure git hooks are executable and configured
if [ -d "$LAUNCHER_DIR/.githooks" ]; then
    chmod +x "$LAUNCHER_DIR/.githooks/"* 2>/dev/null || true
    git -C "$LAUNCHER_DIR" config core.hooksPath .githooks
fi

CARGO_TOML="$LAUNCHER_DIR/Cargo.toml"

GIT_ENGINE='sniffer_engine = { git = "https://github.com/GreenKod/snifferengine.git" }'
LOCAL_ENGINE='sniffer_engine = { path = "../snifferengine/crates/sniffer_engine" }'

GIT_PATCH='android-activity = { git = "https://github.com/GreenKod/snifferengine.git" }'
LOCAL_PATCH='android-activity = { path = "../snifferengine/patches/android-activity" }'

if grep -qF "$GIT_ENGINE" "$CARGO_TOML"; then
    sed -i "s|$GIT_ENGINE|$LOCAL_ENGINE|g" "$CARGO_TOML"
    sed -i "s|$GIT_PATCH|$LOCAL_PATCH|g" "$CARGO_TOML"
    (cd "$LAUNCHER_DIR" && cargo update -w)
    echo -e "\033[1;32m🔧 [DEV MODU AKTİF]\033[0m Cargo.toml ve Cargo.lock yerel '../snifferengine' klasörüne bağlandı."
    echo -e "\033[0;33mℹ️  Commit veya push yaptığında .githooks otomatik olarak GitHub moduna geri çevirecektir.\033[0m"
elif grep -qF "$LOCAL_ENGINE" "$CARGO_TOML"; then
    sed -i "s|$LOCAL_ENGINE|$GIT_ENGINE|g" "$CARGO_TOML"
    sed -i "s|$LOCAL_PATCH|$GIT_PATCH|g" "$CARGO_TOML"
    (cd "$LAUNCHER_DIR" && cargo update -p sniffer_engine -p android-activity)
    echo -e "\033[1;36m🌐 [GIT MODU AKTİF]\033[0m Cargo.toml ve Cargo.lock varsayılan GitHub ('GreenKod/snifferengine') adresine döndürüldü."
else
    echo "❌ Cargo.toml içinde beklenen sniffer_engine satırı bulunamadı!"
    exit 1
fi

