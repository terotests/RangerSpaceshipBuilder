#!/usr/bin/env bash
# Puts Ranger in .deps/Ranger at the revision RangerGames uses: the `ranger`
# prelude crate ship_core builds against, and rgrc (dist/rgrc.js), which
# checks that ship_core stays in Ranger's strict Rust subset.
# With RANGER_DIR set to an existing checkout it links that instead.
set -euo pipefail
cd "$(dirname "$0")/.."
RANGER_REV=e4e275e9d3d33da249572cca14774011a210c670
mkdir -p .deps
if [[ -n "${RANGER_DIR:-}" ]]; then
  ln -sfn "$(cd "$RANGER_DIR" && pwd)" .deps/Ranger
elif [[ ! -L .deps/Ranger ]]; then
  if [[ ! -d .deps/Ranger/.git ]]; then
    git clone --filter=blob:none --no-checkout https://github.com/terotests/Ranger.git .deps/Ranger
  fi
  git -C .deps/Ranger fetch --filter=blob:none origin "$RANGER_REV" 2>/dev/null || git -C .deps/Ranger fetch origin
  git -C .deps/Ranger checkout -q "$RANGER_REV"
fi
echo "ready: .deps/Ranger ($(git -C .deps/Ranger rev-parse --short HEAD))"
