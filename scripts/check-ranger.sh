#!/usr/bin/env bash
# Checks that ship_core is still in Ranger's strict Rust subset: rgrc reads
# crates/ship_core/src/lib.rs and writes it as JavaScript (and other targets
# with TARGETS="es6 cpp go …"), then the JavaScript builds the same .glb
# files as the cargo build (tests/parity.cjs).
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=target/ranger
mkdir -p "$OUT"
for t in ${TARGETS:-es6}; do
  log=$(node .deps/Ranger/dist/rgrc.js -l="$t" -nodemodule crates/ship_core/src/lib.rs -d="$OUT" -o="ship_core.$t" 2>&1) || true
  if echo "$log" | grep -q "FAIL\|\[ERROR\]"; then
    echo "$log" | grep -v "api docs" | grep -B6 -A4 "FAIL\|ERROR" | head -80
    echo "ship_core: not in the Ranger subset for $t"
    exit 1
  fi
  echo "ship_core -> $t: ok ($OUT/ship_core.$t)"
done
cp "$OUT/ship_core.es6.js" "$OUT/ship_core.js"
cargo build -q --release -p ship_cli
node tests/parity.cjs
