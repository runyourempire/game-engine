# mcp-game-server

MCP (Model Context Protocol) server for the **GAME compiler** (Generative Animation Matrix Engine). Exposes the GAME compiler to AI agents -- compile `.game` DSL source into WebGPU shaders, HTML pages, and Web Components directly from Claude Code, Cursor, or any MCP-compatible host.

## Tools

| Tool | Description |
|------|-------------|
| `compile` | Compile `.game` source to WGSL shader, self-contained HTML, or ES module Web Component |
| `validate` | Check `.game` source for syntax/semantic errors without full compilation output |
| `lint` | Validate + surface structured warnings, error line numbers, and helpful suggestions |
| `list_primitives` | List all 37 GAME builtins organized by type-state transition, with params and defaults |
| `list_stdlib` | List all stdlib modules and their exported functions (11 modules) |
| `list_presets` | List all preset names grouped by category (7 categories) |

## Resources

| URI | Description |
|-----|-------------|
| `game://language-reference` | The `.game` language specification |
| `game://primitives` | All built-in primitives and functions |
| `game://examples` | Example `.game` files |

## Prompts

| Name | Description |
|------|-------------|
| `generate-component` | Generate `.game` source from a natural language description |
| `iterate-component` | Refine existing `.game` source based on feedback |
| `describe-component` | Describe what a `.game` visual effect does in plain English |
| `generate-4da-component` | Generate a `.game` component tuned for the 4DA desktop app |
| `generate-achievement-visual` | Generate achievement/progression UI visuals |
| `generate-game-indicator` | Generate status indicators, health bars, and XP gauges |

## Setup

### Prerequisites

- Node.js >= 18
- The GAME compiler binary (`game.exe`) built via `cargo build --release` in the `game-compiler/` directory

### Install

```bash
cd D:\GAME\mcp-game-server
npm install
npm run build
```

### Configure for Claude Code

Add to your Claude Code MCP settings:

```json
{
  "mcpServers": {
    "game": {
      "command": "node",
      "args": ["D:/GAME/mcp-game-server/dist/index.js"],
      "env": {
        "GAME_COMPILER_PATH": "D:/GAME/game-compiler/target/release/game.exe",
        "GAME_ROOT": "D:/GAME"
      }
    }
  }
}
```

### Configure for Claude Desktop

Add to `%APPDATA%\Claude\claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "game": {
      "command": "node",
      "args": ["D:/GAME/mcp-game-server/dist/index.js"],
      "env": {
        "GAME_COMPILER_PATH": "D:/GAME/game-compiler/target/release/game.exe",
        "GAME_ROOT": "D:/GAME"
      }
    }
  }
}
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `GAME_COMPILER_PATH` | `../game-compiler/target/release/game.exe` (relative to dist/) | Absolute path to the GAME compiler binary |
| `GAME_ROOT` | Parent of `mcp-game-server/` | Path to the GAME project root (where LANGUAGE.md, PRIMITIVES.md, examples/ live) |

## Usage Examples

### Compile a .game file to Web Component

```
Tool: compile
Input: {
  "source": "cinematic \"Hello\" {\n  layer {\n    fn: circle(0.3 + sin(time) * 0.05) | glow(2.0)\n  }\n}",
  "format": "component",
  "tag": "hello-shader"
}
```

### Lint with structured warnings

```
Tool: lint
Input: {
  "source": "cinematic \"Test\" {\n  layer {\n    fn: glow(2.0) | circle(0.3)\n  }\n}"
}
```

### List builtins by category

```
Tool: list_primitives
Input: { "category": "sdf_generators" }
```

### List stdlib modules

```
Tool: list_stdlib
Input: { "module": "patterns" }
```

### List presets

```
Tool: list_presets
Input: { "category": "ui" }
```

### Generate from description (prompt)

```
Prompt: generate-component
Arguments: {
  "description": "A pulsing nebula that reacts to mouse movement"
}
```

## Development

```bash
npm run dev          # Watch mode (recompile on change)
npm run build        # Build once
npm run inspect      # Launch MCP inspector
```
