# Contributing to GAME

Thank you for your interest in contributing to GAME.

## License

By contributing to this repository, you agree that your contributions will be licensed under the [FSL-1.1-Apache-2.0](LICENSE) license.

## Getting Started

```bash
# Clone
git clone https://github.com/runyourempire/game-engine.git
cd game-engine/game-compiler

# Build
cargo build

# Run tests (567 tests, should all pass)
cargo test --lib

# Compile an example
cargo run -- build examples/001-hello.game -o dist

# Run the dev server with hot reload
cargo run -- dev examples/001-hello.game
```

## Project Structure

```
src/
  main.rs          — CLI (build, dev, validate, new, info)
  lib.rs           — Public API (compile, compile_to_ast)
  lexer.rs         — Logos-based tokenizer
  parser.rs        — Hand-written recursive descent parser
  ast.rs           — Abstract syntax tree types
  error.rs         — Error types (CompileError, Diagnostic)
  builtins.rs      — Builtin function signatures
  codegen/         — Shader code generation
    mod.rs         — Orchestration + validation
    stages.rs      — Pipeline state machine
    wgsl.rs        — WebGPU shader generation
    glsl.rs        — WebGL2 shader generation
    ...            — Feature-specific generators
  runtime/         — JavaScript runtime templates
    component.rs   — Web Component wrapper
    helpers.rs     — GameRenderer / GameRendererGL classes
    html.rs        — HTML output format
  lsp.rs           — Language Server Protocol (behind `lsp` feature)
  wasm.rs          — WASM bindings (behind `wasm` feature)
editors/vscode/    — VS Code extension
wrappers/          — Framework wrappers (React, Vue, Svelte)
examples/          — 78 reference .game files
```

## Development Guidelines

- Run `cargo test --lib` before submitting — all 567 tests must pass
- Run `cargo clippy` and fix any warnings
- Run `cargo fmt` for consistent formatting
- New builtins need: parser support, WGSL codegen, GLSL codegen, pipeline state, and a test
- New features need an example in `examples/`

## Reporting Issues

Open an issue on [GitHub](https://github.com/runyourempire/game-engine/issues) with:
- GAME source code that reproduces the problem
- Expected vs actual output
- `game --version` output
