# GAME Language for VS Code

Syntax highlighting, code snippets, and language server support for the [GAME](https://github.com/runyourempire/game-engine) shader DSL (Generative Animation Matrix Engine).

## Features

- **Live Preview** -- Open a side panel that renders your `.game` file in real-time as you type (`Ctrl+Shift+G`)
- **Visual Parameter Tuner** -- Click any number, color, or palette in your code to get sliders, color pickers, and palette selectors. Changes flow bidirectionally between code and preview.
- **Component Gallery** -- Browse 32 built-in components across 6 categories. Open, preview, and fork into your workspace (`Ctrl+Shift+L`)
- **AI Generation** -- Describe a visual in natural language and get working GAME code via Claude API with auto-retry on compilation failures (`Ctrl+Shift+A`)
- **One-Click Export** -- Export compiled components as JS, HTML embed, React usage, standalone HTML, or save to file
- **Syntax Highlighting** -- Full TextMate grammar for `.game` files covering keywords, builtins, operators, literals, and more
- **Language Server** -- Connects to `game lsp` for diagnostics, hover info, completions, and go-to-definition
- **Snippets** -- 17 snippets for common patterns (cinematics, layers, scenes, effects)
- **Language Configuration** -- Bracket matching, auto-closing pairs, comment toggling, code folding

## Requirements

- VS Code 1.85.0 or later
- The `game` compiler binary on your PATH (or configured via `game.serverPath`)
- For AI generation: an Anthropic API key (stored securely via VS Code SecretStorage)

## Keyboard Shortcuts

| Shortcut | Command |
|----------|---------|
| `Ctrl+Shift+G` | Open Live Preview |
| `Ctrl+Shift+L` | Open Component Gallery |
| `Ctrl+Shift+A` | Generate with AI |

## Extension Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `game.serverPath` | `"game"` | Path to the game compiler binary |
| `game.trace.server` | `"off"` | Trace communication with the language server |
| `game.ai.model` | `"claude-sonnet-4-20250514"` | Claude model for AI generation |

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
