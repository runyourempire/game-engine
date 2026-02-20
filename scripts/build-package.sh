#!/usr/bin/env bash
# Build the game-components npm package from presets
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$SCRIPT_DIR")"
COMPILER="$ROOT/game-compiler/target/release/game"
OUTDIR="$ROOT/package/dist"

# Build compiler in release mode if needed
if [ ! -f "$COMPILER" ] && [ ! -f "$COMPILER.exe" ]; then
  echo "Building GAME compiler (release)..."
  cd "$ROOT/game-compiler" && cargo build --release
fi

# Use .exe on Windows
if [ -f "$COMPILER.exe" ]; then
  COMPILER="$COMPILER.exe"
fi

echo "Compiling presets → package/dist/"
"$COMPILER" build "$ROOT/presets/" --outdir "$OUTDIR"

echo ""
echo "Package ready at $ROOT/package/"
echo "Publish with: cd package && npm publish"
