# GAME Language for VS Code

Syntax highlighting, code snippets, and language server support for the [GAME](https://github.com/runyourempire/game-engine) shader DSL (Generative Animation Matrix Engine).

## Features

- **Syntax Highlighting** -- Full TextMate grammar for `.game` files covering keywords, builtins, operators, literals, and more
- **Language Server** -- Connects to `game lsp` for diagnostics, hover info, completions, and go-to-definition
- **Snippets** -- 17 snippets for common patterns (cinematics, layers, scenes, effects)
- **Language Configuration** -- Bracket matching, auto-closing pairs, comment toggling, code folding

## Requirements

- VS Code 1.85.0 or later
- For LSP features: the `game` compiler binary on your PATH (or configured via `game.serverPath`)

## Extension Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `game.serverPath` | `"game"` | Path to the game compiler binary |
| `game.trace.server` | `"off"` | Trace communication with the language server (`off`, `messages`, `verbose`) |

## Snippets

| Prefix | Description |
|--------|-------------|
| `cinematic` | Cinematic block with layer |
| `layer` | Layer with pipeline |
| `layer-blend` | Layer with blend mode |
| `layer-memory` | Layer with persistence memory |
| `fn` | Function definition |
| `listen` | Audio listener block |
| `arc` | Animation arc block |
| `resonate` | Parameter coupling block |
| `glow-orb` | Glowing orb pattern |
| `neon-ring` | Neon ring with palette |
| `loading` | Loading spinner |
| `organic` | Organic texture (warp + fbm + palette) |
| `scene` | Scene with transitions |
| `import` | Import standard library |
| `use` | Use standard library |
| `matrix` | Coupling matrix |
| `game-full` | Full component with all blocks |

## Development

```bash
cd editors/vscode
npm install
npm run compile
```

To test locally, press F5 in VS Code to launch an Extension Development Host.

To package:

```bash
npm run package
```

## License

FSL-1.1-Apache-2.0
