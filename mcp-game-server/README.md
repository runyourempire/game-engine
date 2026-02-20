# mcp-game-server

MCP (Model Context Protocol) server for the **GAME compiler** (Generative Animation Matrix Engine). Exposes the GAME compiler to AI agents — compile `.game` DSL source into WebGPU shaders, HTML pages, and Web Components directly from Claude Code, Cursor, or any MCP-compatible host.

## Tools

| Tool | Description |
|------|-------------|
| `compile` | Compile `.game` source to WGSL shader, self-contained HTML, or ES module Web Component |
| `validate` | Check `.game` source for syntax/semantic errors without full compilation output |
| `list_primitives` | List all available GAME language primitives grouped by category |

## Resources

| URI | Description |
|-----|-------------|
| `game://language-reference` | The `.game` language specification |
| `game://primitives` | All built-in primitives and functions |
| `game://examples` | Example `.game` files |

## Prompts

| Name | Description |
|------|-------------|
| `generate-component` | Guide an LLM to produce valid `.game` source from a natural language description |

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

### Validate syntax

```
Tool: validate
Input: {
  "source": "cinematic { layer { fn: sphere(0.5) | glow(1.0) } }"
}
```

### List available primitives

```
Tool: list_primitives
Input: {}
```

### Generate from description (prompt)

```
Prompt: generate-component
Arguments: {
  "description": "A pulsing nebula that reacts to mouse movement with iridescent colors"
}
```

## Development

```bash
npm run dev          # Watch mode (recompile on change)
npm run build        # Build once
npm run inspect      # Launch MCP inspector
```
