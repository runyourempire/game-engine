#!/usr/bin/env bash
# Build WASM and package the VSCode extension as a .vsix file.
#
# Prerequisites:
#   - wasm-pack (cargo install wasm-pack)
#   - vsce (npm i -g @vscode/vsce)
#
# Usage:
#   ./scripts/build-vscode.sh

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Building WASM ==="
cd "$ROOT/game-compiler"
wasm-pack build --target web --release

echo "=== Copying WASM to VSCode extension ==="
DEST="$ROOT/vscode-game/pkg"
rm -rf "$DEST"
cp -r "$ROOT/game-compiler/pkg" "$DEST"

echo "=== Packaging VSIX ==="
cd "$ROOT/vscode-game"
vsce package --no-dependencies

echo "=== Done ==="
ls -la "$ROOT/vscode-game/"*.vsix 2>/dev/null || echo "(no .vsix found — check vsce output above)"
