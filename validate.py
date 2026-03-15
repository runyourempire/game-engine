#!/usr/bin/env python3
"""GAME Validate-and-Generate Pipeline.

Takes a natural language description, generates .game code via LLM,
validates it compiles, and optionally builds the Web Component.

Usage:
    python validate.py "a breathing cosmic nebula with organic tendrils"
    python validate.py --file description.txt
    python validate.py --validate source.game
"""

import subprocess
import sys
import json
import os
from pathlib import Path

COMPILER_DIR = Path(__file__).parent
COMPILER_BIN = COMPILER_DIR / 'target' / 'release' / 'game'
if not COMPILER_BIN.exists():
    COMPILER_BIN = COMPILER_DIR / 'target' / 'debug' / 'game'

def validate_game(source: str) -> tuple[bool, str]:
    """Validate .game source code compiles without errors.
    
    Returns (success, message).
    """
    # Write to temp file
    tmp = COMPILER_DIR / '.tmp_validate.game'
    tmp.write_text(source, encoding='utf-8')
    
    try:
        result = subprocess.run(
            [str(COMPILER_BIN), 'build', str(tmp), '-o', str(COMPILER_DIR / '.tmp_out')],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            return True, 'Compilation successful'
        else:
            error = result.stderr.strip() or result.stdout.strip()
            return False, f'Compilation error: {error}'
    except FileNotFoundError:
        return False, f'Compiler not found at {COMPILER_BIN}. Run: cargo build --release'
    except subprocess.TimeoutExpired:
        return False, 'Compilation timed out (>10s)'
    finally:
        tmp.unlink(missing_ok=True)
        # Clean up output
        out_dir = COMPILER_DIR / '.tmp_out'
        if out_dir.exists():
            for f in out_dir.iterdir():
                f.unlink()
            out_dir.rmdir()


def validate_game_syntax(source: str) -> list[str]:
    """Quick syntax validation without full compilation.
    
    Returns list of issues found.
    """
    issues = []
    
    # Must have at least one cinematic block
    if 'cinematic' not in source:
        issues.append('Missing cinematic block')
    
    # Check balanced braces
    opens = source.count('{')
    closes = source.count('}')
    if opens != closes:
        issues.append(f'Unbalanced braces: {opens} open, {closes} close')
    
    # Check cinematic names are quoted
    import re
    cinematics = re.findall(r'cinematic\s+(\S+)', source)
    for name in cinematics:
        if not (name.startswith(') and name.endswith(')):
            issues.append(f'Cinematic name must be quoted: {name}')
    
    # Check layers have names
    layers = re.findall(r'layer\s+(\S+)', source)
    for name in layers:
        if name == '{':
            issues.append('Layer missing name')
    
    # Check pipelines end in Color state (basic check)
    # Look for layer bodies that have SDF generators but no bridge
    sdf_generators = ['circle', 'ring', 'star', 'box', 'hex', 'fbm', 'simplex', 
                      'voronoi', 'line', 'capsule', 'triangle', 'arc_sdf', 'cross',
                      'heart', 'egg', 'spiral', 'grid', 'radial_fade']
    bridges = ['glow', 'shade', 'emissive', 'palette']
    
    # Simple heuristic: if any SDF generator appears, a bridge should too
    for gen in sdf_generators:
        if gen + '(' in source:
            has_bridge = any(b + '(' in source or b + ' ' in source for b in bridges)
            if not has_bridge:
                issues.append(f'SDF generator {gen}() found but no bridge (glow/shade/palette) to reach Color state')
                break
    
    return issues


def build_game(source: str, output_dir: str = None) -> tuple[bool, str, list[str]]:
    """Compile .game source to Web Components.
    
    Returns (success, message, output_files).
    """
    if output_dir is None:
        output_dir = str(COMPILER_DIR / 'dist')
    
    tmp = COMPILER_DIR / '.tmp_build.game'
    tmp.write_text(source, encoding='utf-8')
    
    try:
        result = subprocess.run(
            [str(COMPILER_BIN), 'build', str(tmp), '-o', output_dir],
            capture_output=True,
            text=True,
            timeout=30,
        )
        
        if result.returncode == 0:
            output_path = Path(output_dir)
            files = [str(f) for f in output_path.iterdir()] if output_path.exists() else []
            return True, 'Build successful', files
        else:
            error = result.stderr.strip() or result.stdout.strip()
            return False, f'Build error: {error}', []
    except FileNotFoundError:
        return False, f'Compiler not found at {COMPILER_BIN}', []
    finally:
        tmp.unlink(missing_ok=True)


def main():
    import argparse
    parser = argparse.ArgumentParser(description='GAME validate-and-generate pipeline')
    parser.add_argument('input', nargs='?', help='Natural language description or .game file path')
    parser.add_argument('--validate', metavar='FILE', help='Validate a .game file')
    parser.add_argument('--build', metavar='FILE', help='Build a .game file to Web Components')
    parser.add_argument('-o', '--output', default=None, help='Output directory')
    parser.add_argument('--syntax-only', action='store_true', help='Quick syntax check only')
    args = parser.parse_args()
    
    if args.validate:
        source = Path(args.validate).read_text(encoding='utf-8')
        if args.syntax_only:
            issues = validate_game_syntax(source)
            if issues:
                print('Syntax issues:')
                for issue in issues:
                    print(f'  - {issue}')
                sys.exit(1)
            else:
                print('Syntax OK')
        else:
            ok, msg = validate_game(source)
            print(msg)
            sys.exit(0 if ok else 1)
    
    elif args.build:
        source = Path(args.build).read_text(encoding='utf-8')
        ok, msg, files = build_game(source, args.output)
        print(msg)
        if files:
            print('Output files:')
            for f in files:
                print(f'  {f}')
        sys.exit(0 if ok else 1)
    
    elif args.input:
        # If it is a .game file, validate it
        if args.input.endswith('.game') and Path(args.input).exists():
            source = Path(args.input).read_text(encoding='utf-8')
            ok, msg = validate_game(source)
            print(msg)
            sys.exit(0 if ok else 1)
        else:
            # Natural language input -- for now just print the prompt
            print(f'Description: {args.input}')
            print('LLM generation not yet wired -- use the system prompt in prompts/generate-visual.md')
            print(f'Then validate with: python {sys.argv[0]} --validate output.game')
    
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
