#!/usr/bin/env bash
# Builds the editor into dist/web: ship_core as WebAssembly (ship_web.wasm),
# the page, and the recipes under ships/ as presets.js.
#   scripts/build-web.sh          release build
#   python3 -m http.server -d dist/web 8000   → http://localhost:8000
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -p ship_web --target wasm32-unknown-unknown --release
OUT=dist/web
rm -rf "$OUT"; mkdir -p "$OUT"
cp web/* "$OUT"/
cp target/wasm32-unknown-unknown/release/ship_web.wasm "$OUT"/
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -O2 --strip-debug "$OUT/ship_web.wasm" -o "$OUT/ship_web.wasm" || true
fi
node scripts/presets.mjs ships "$OUT/presets.js"
echo "built $OUT ($(du -h "$OUT/ship_web.wasm" | cut -f1) wasm)"
