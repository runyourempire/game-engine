#!/usr/bin/env bash
# Build the GAME playground — compiles WASM and copies to pkg/
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
COMPILER_DIR="$SCRIPT_DIR/../game-compiler"

echo "Building WASM compiler..."
cd "$COMPILER_DIR"
wasm-pack build --target web --features wasm

echo "Copying pkg to playground..."
rm -rf "$SCRIPT_DIR/pkg"
cp -r "$COMPILER_DIR/pkg" "$SCRIPT_DIR/pkg"

echo "Done. Open playground/index.html in a browser or run:"
echo "  python3 -m http.server 8080 -d $SCRIPT_DIR"
echo "  open http://localhost:8080"
