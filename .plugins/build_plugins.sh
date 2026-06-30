#!/usr/bin/env bash
# =============================================================================
#  .plugins/build_plugins.sh
#  Builds every Wasm plugin that has source code in this directory.
#
#  For each plugin with "has_source": true in plugins.json:
#    1. cargo fmt --check       (enforces formatting)
#    2. cargo clippy (pedantic + nursery + cargo, -D warnings)
#    3. cargo build --release   (compiles to wasm32-unknown-unknown)
#    4. copies .wasm output to the plugin's own folder
#
#  Usage (from repo root inside nix-shell):
#    nix-shell --run ".plugins/build_plugins.sh"
#    .plugins/build_plugins.sh           # if already in nix-shell
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGINS_JSON="${SCRIPT_DIR}/plugins.json"

if ! command -v jq &>/dev/null; then
  echo "❌  'jq' is required but not found. Add it to your nix-shell buildInputs."
  exit 1
fi

# Collect plugins with has_source == true into an array
mapfile -t SOURCE_DIRS < <(jq -r '.[] | select(.has_source == true) | .source_dir' "${PLUGINS_JSON}")
mapfile -t PLUGIN_IDS  < <(jq -r '.[] | select(.has_source == true) | .identifier.id' "${PLUGINS_JSON}")
mapfile -t WASM_PATHS  < <(jq -r '.[] | select(.has_source == true) | .location.path' "${PLUGINS_JSON}")

PLUGIN_COUNT="${#SOURCE_DIRS[@]}"

if [[ "${PLUGIN_COUNT}" -eq 0 ]]; then
  echo "ℹ️  No plugins with has_source=true found in plugins.json. Nothing to build."
  exit 0
fi

echo "🔧  Found ${PLUGIN_COUNT} source plugin(s) to build."
echo ""

FAILED=0

for i in "${!SOURCE_DIRS[@]}"; do
  PLUGIN_ID="${PLUGIN_IDS[$i]}"
  SOURCE_DIR="${SOURCE_DIRS[$i]}"
  WASM_OUTPUT="${WASM_PATHS[$i]}"

  PLUGIN_PATH="${SCRIPT_DIR}/${SOURCE_DIR}"
  WASM_DEST="${SCRIPT_DIR}/${WASM_OUTPUT}"
  WASM_DEST_DIR="$(dirname "${WASM_DEST}")"

  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "📦  Plugin: ${PLUGIN_ID}"
  echo "    Source : ${PLUGIN_PATH}"
  echo "    Output : ${WASM_DEST}"
  echo ""

  if [[ ! -d "${PLUGIN_PATH}" ]]; then
    echo "    ❌  Source directory not found: ${PLUGIN_PATH}"
    FAILED=$((FAILED + 1))
    continue
  fi

  if [[ ! -f "${PLUGIN_PATH}/Cargo.toml" ]]; then
    echo "    ❌  No Cargo.toml in ${PLUGIN_PATH}"
    FAILED=$((FAILED + 1))
    continue
  fi

  pushd "${PLUGIN_PATH}" > /dev/null

  STEP_FAILED=0

  # ── Step 1: Format check ────────────────────────────────────────────────────
  echo "  [1/3] 🖊️   cargo fmt --check"
  if ! cargo fmt --manifest-path Cargo.toml --check; then
    echo "    ❌  Format check failed. Run 'cargo fmt' inside ${SOURCE_DIR} to fix."
    STEP_FAILED=1
  else
    echo "    ✅  Format OK"
  fi

  if [[ "${STEP_FAILED}" -eq 0 ]]; then
    # ── Step 2: Clippy (maximum strictness) ────────────────────────────────────
    echo "  [2/3] 🔍  cargo clippy (pedantic + nursery + cargo, -D warnings)"
    if ! cargo clippy \
      --manifest-path Cargo.toml \
      --target wasm32-unknown-unknown \
      --all-targets \
      --all-features \
      -- \
      -D warnings \
      -D clippy::pedantic \
      -D clippy::nursery \
      -D clippy::cargo \
      -A clippy::multiple-crate-versions; then
      echo "    ❌  Clippy failed."
      STEP_FAILED=1
    else
      echo "    ✅  Clippy OK"
    fi
  fi

  if [[ "${STEP_FAILED}" -eq 0 ]]; then
    # ── Step 3: Release build ──────────────────────────────────────────────────
    echo "  [3/3] 🏗️   cargo build --release --target wasm32-unknown-unknown"
    if ! cargo build --manifest-path Cargo.toml --target wasm32-unknown-unknown --release; then
      echo "    ❌  Build failed."
      STEP_FAILED=1
    else
      # Resolve crate name from Cargo.toml (cdylib target name)
      CRATE_NAME=$(cargo metadata --manifest-path Cargo.toml --no-deps --format-version 1 \
        | jq -r '.packages[0].targets[] | select(.kind[] == "cdylib") | .name' \
        | tr '-' '_')
      BUILT_WASM="target/wasm32-unknown-unknown/release/${CRATE_NAME}.wasm"

      if [[ ! -f "${BUILT_WASM}" ]]; then
        echo "    ❌  Expected output not found: ${BUILT_WASM}"
        STEP_FAILED=1
      else
        mkdir -p "${WASM_DEST_DIR}"
        cp "${BUILT_WASM}" "${WASM_DEST}"
        echo "    ✅  Built → ${WASM_DEST}"
      fi
    fi
  fi

  popd > /dev/null

  if [[ "${STEP_FAILED}" -ne 0 ]]; then
    FAILED=$((FAILED + 1))
  fi

  echo ""
done

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [[ "${FAILED}" -eq 0 ]]; then
  echo "🎉  All ${PLUGIN_COUNT} plugin(s) built successfully!"
else
  echo "💥  ${FAILED}/${PLUGIN_COUNT} plugin(s) failed. See above for details."
  exit 1
fi
