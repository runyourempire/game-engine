import * as vscode from "vscode";

export type TunableKind = "number" | "color" | "palette";

export interface TunableToken {
  kind: TunableKind;
  value: string;
  line: number;
  col: number;
  endCol: number;
  context: string;
  range?: { min: number; max: number; step: number };
  palettes?: string[];
}

const KNOWN_PALETTES = [
  "fire", "ocean", "neon", "aurora", "sunset", "ice", "ember", "lava",
  "magma", "inferno", "plasma", "electric", "cyber", "matrix", "forest",
  "moss", "earth", "desert", "blood", "rose", "candy", "royal", "deep_sea",
  "coral", "arctic", "twilight", "vapor", "gold", "silver", "monochrome",
];

const CONTEXT_RANGES: Record<string, { min: number; max: number; step: number }> = {
  glow: { min: 0, max: 10, step: 0.1 },
  radius: { min: 0, max: 1, step: 0.01 },
  opacity: { min: 0, max: 1, step: 0.01 },
  speed: { min: 0, max: 20, step: 0.1 },
  scale: { min: 0, max: 5, step: 0.1 },
  intensity: { min: 0, max: 10, step: 0.1 },
  warp: { min: 0, max: 10, step: 0.1 },
  fbm: { min: 1, max: 8, step: 1 },
  memory: { min: 0, max: 1, step: 0.01 },
  vignette: { min: 0, max: 1, step: 0.01 },
  distort: { min: 0, max: 10, step: 0.1 },
  tint: { min: 0, max: 1, step: 0.01 },
  resonate: { min: 0, max: 5, step: 0.1 },
  arc: { min: 0, max: 6.2832, step: 0.01 },
  canvas: { min: 1, max: 2000, step: 1 },
};

const DEFAULT_RANGE = { min: 0, max: 10, step: 0.1 };

export function detectTunableToken(
  document: vscode.TextDocument,
  position: vscode.Position
): TunableToken | null {
  const line = document.lineAt(position.line).text;
  const cursor = position.character;

  // Check palette(name) — cursor inside the parentheses
  const paletteMatch = /palette\(\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\)/.exec(line);
  if (paletteMatch) {
    const nameStart = line.indexOf(paletteMatch[1], paletteMatch.index);
    const nameEnd = nameStart + paletteMatch[1].length;
    if (cursor >= paletteMatch.index && cursor <= paletteMatch.index + paletteMatch[0].length) {
      return {
        kind: "palette",
        value: paletteMatch[1],
        line: position.line,
        col: nameStart,
        endCol: nameEnd,
        context: "palette",
        palettes: KNOWN_PALETTES,
      };
    }
  }

  // Check tint(r, g, b) — detect if cursor is on one of the three numbers
  const tintMatch = /tint\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*\)/.exec(line);
  if (tintMatch && cursor >= tintMatch.index && cursor <= tintMatch.index + tintMatch[0].length) {
    const r = tintMatch[1], g = tintMatch[2], b = tintMatch[3];
    const rHex = Math.round(parseFloat(r) * 255).toString(16).padStart(2, "0");
    const gHex = Math.round(parseFloat(g) * 255).toString(16).padStart(2, "0");
    const bHex = Math.round(parseFloat(b) * 255).toString(16).padStart(2, "0");
    return {
      kind: "color",
      value: `#${rHex}${gHex}${bHex}`,
      line: position.line,
      col: tintMatch.index,
      endCol: tintMatch.index + tintMatch[0].length,
      context: "tint",
      range: { min: 0, max: 1, step: 0.01 },
    };
  }

  // Check #RRGGBB hex color
  const hexPattern = /#[0-9a-fA-F]{6}/g;
  let hexMatch;
  while ((hexMatch = hexPattern.exec(line)) !== null) {
    if (cursor >= hexMatch.index && cursor <= hexMatch.index + 7) {
      return {
        kind: "color",
        value: hexMatch[0],
        line: position.line,
        col: hexMatch.index,
        endCol: hexMatch.index + 7,
        context: "hex",
      };
    }
  }

  // Check number literals — find the number at cursor
  const numPattern = /\b(\d+\.?\d*)\b/g;
  let numMatch;
  while ((numMatch = numPattern.exec(line)) !== null) {
    const start = numMatch.index;
    const end = start + numMatch[0].length;
    if (cursor >= start && cursor <= end) {
      // Determine context from the function call surrounding this number
      const prefix = line.substring(0, start).trimEnd();
      let context = "unknown";
      const fnCall = /([a-zA-Z_]\w*)\s*\(\s*$/.exec(prefix);
      if (fnCall) {
        context = fnCall[1];
      } else {
        // Check if after a comma inside a function call
        const fnComma = /([a-zA-Z_]\w*)\s*\([^)]*,\s*$/.exec(prefix);
        if (fnComma) {
          context = fnComma[1];
        }
      }

      const range = CONTEXT_RANGES[context] ?? DEFAULT_RANGE;

      return {
        kind: "number",
        value: numMatch[0],
        line: position.line,
        col: start,
        endCol: end,
        context,
        range,
      };
    }
  }

  return null;
}
