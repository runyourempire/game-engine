#!/usr/bin/env node
/**
 * GAME MCP Server
 *
 * Exposes the GAME compiler (Generative Animation Matrix Engine) to AI agents
 * via the Model Context Protocol. Compiles .game DSL source into WebGPU
 * shaders (WGSL), self-contained HTML pages, and ES module Web Components.
 *
 * Tools:
 *   - compile: Compile .game source to WGSL, HTML, or Web Component output
 *   - validate: Check .game source for syntax/semantic errors
 *   - lint: Validate + surface structured warnings with suggestions
 *   - list_primitives: List all 37 GAME language builtins with type states
 *   - list_stdlib: List all stdlib modules and their exported functions
 *   - list_presets: List all available preset names grouped by category
 *
 * Resources:
 *   - game://language-reference: The .game language specification
 *   - game://primitives: All available primitives and built-in functions
 *   - game://examples: Example .game files
 *
 * Prompts:
 *   - generate-component: Guide an LLM to produce .game source from a description
 *   - iterate-component: Refine existing .game source based on natural language feedback
 *   - describe-component: Describe what a .game visual effect does in plain English
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
  ListPromptsRequestSchema,
  GetPromptRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import { execFile } from "node:child_process";
import { writeFile, unlink, readFile, readdir } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { randomBytes } from "node:crypto";

// =============================================================================
// Configuration
// =============================================================================

/**
 * Resolve the path to the GAME compiler binary.
 *
 * Priority:
 *   1. GAME_COMPILER_PATH environment variable (absolute path)
 *   2. Default: ../game-compiler/target/release/game.exe relative to project root
 *
 * On Windows the binary is game.exe; the default path assumes a standard
 * Cargo release build layout adjacent to this MCP server directory.
 */
function resolveCompilerPath(): string {
  if (process.env.GAME_COMPILER_PATH) {
    return process.env.GAME_COMPILER_PATH;
  }
  // Default: sibling directory relative to where the server package lives.
  // When installed from dist/, __dirname is mcp-game-server/dist so we go
  // up two levels to reach the GAME project root.
  // However, for maximum clarity we use an absolute fallback that matches the
  // documented location.
  const defaultPath = join(__dirname, "..", "..", "game-compiler", "target", "release", "game.exe");
  return defaultPath;
}

// Resolve __dirname for ESM
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * Resolve the GAME project root (one level above mcp-game-server/).
 */
function resolveGameRoot(): string {
  if (process.env.GAME_ROOT) {
    return process.env.GAME_ROOT;
  }
  return join(__dirname, "..", "..");
}

const COMPILER_PATH = resolveCompilerPath();
const GAME_ROOT = resolveGameRoot();

// =============================================================================
// Compiler Execution Helpers
// =============================================================================

/**
 * Create a temporary .game file, returning its absolute path.
 * The caller is responsible for cleanup via cleanupTempFile().
 */
async function writeTempGameFile(source: string): Promise<string> {
  const id = randomBytes(8).toString("hex");
  const tempPath = join(tmpdir(), `game_mcp_${id}.game`);
  await writeFile(tempPath, source, "utf-8");
  return tempPath;
}

/**
 * Delete a temporary file. Swallows errors silently.
 */
async function cleanupTempFile(filePath: string): Promise<void> {
  try {
    await unlink(filePath);
  } catch {
    // Ignore — temp files will be cleaned by the OS eventually
  }
}

/**
 * Run the GAME compiler with the given arguments.
 * Returns { stdout, stderr, exitCode }.
 */
function runCompiler(args: string[]): Promise<{ stdout: string; stderr: string; exitCode: number }> {
  return new Promise((resolve) => {
    execFile(COMPILER_PATH, args, { timeout: 30_000, maxBuffer: 10 * 1024 * 1024 }, (error, stdout, stderr) => {
      const exitCode = error && "code" in error ? (error.code as number) : error ? 1 : 0;
      resolve({ stdout: stdout ?? "", stderr: stderr ?? "", exitCode });
    });
  });
}

// =============================================================================
// Primitives Data (embedded — avoids file I/O at runtime for this tool)
// =============================================================================

/**
 * All 37 builtins from the GAME compiler (builtins.rs), organized by type state transition.
 *
 * Type state pipeline: Position -> Sdf -> Color
 * - Position -> Position: domain transforms (before SDF evaluation)
 * - Position -> Sdf: SDF generators (shapes + noise)
 * - Sdf -> Sdf: SDF modifiers (post-shape adjustments)
 * - Sdf -> Color: bridge functions (glow, shade, emissive)
 * - Position -> Color: full-screen generators (gradient, spectrum)
 * - Color -> Color: post-processing and color adjustments
 */
const PRIMITIVES_DATA = {
  _meta: {
    total_builtins: 37,
    type_states: ["Position", "Sdf", "Color"],
    pipeline_explanation:
      "GAME uses a type-state pipeline: Position -> Sdf -> Color. " +
      "Each builtin consumes one state and produces another. " +
      "The pipe operator | chains them left-to-right. " +
      "Domain ops (Position->Position) go first, then SDF generators (Position->Sdf), " +
      "then SDF modifiers (Sdf->Sdf), then bridges (Sdf->Color), then color processors (Color->Color). " +
      "The compiler enforces valid transitions at compile time.",
  },
  sdf_generators: {
    description: "Position -> Sdf: Generate signed distance fields from position. Place after domain ops in pipe chain.",
    entries: [
      { name: "circle", syntax: "circle(radius)", params: [{ name: "radius", default: 0.2 }], input: "Position", output: "Sdf" },
      { name: "ring", syntax: "ring(radius, width)", params: [{ name: "radius", default: 0.3 }, { name: "width", default: 0.02 }], input: "Position", output: "Sdf" },
      { name: "star", syntax: "star(points, radius, inner)", params: [{ name: "points", default: 5 }, { name: "radius", default: 0.3 }, { name: "inner", default: 0.15 }], input: "Position", output: "Sdf" },
      { name: "box", syntax: "box(w, h)", params: [{ name: "w", default: 0.2 }, { name: "h", default: 0.2 }], input: "Position", output: "Sdf" },
      { name: "polygon", syntax: "polygon(sides, radius)", params: [{ name: "sides", default: 6 }, { name: "radius", default: 0.3 }], input: "Position", output: "Sdf" },
      { name: "fbm", syntax: "fbm(scale, octaves, persistence, lacunarity)", params: [{ name: "scale", default: 1.0 }, { name: "octaves", default: 4 }, { name: "persistence", default: 0.5 }, { name: "lacunarity", default: 2.0 }], input: "Position", output: "Sdf" },
      { name: "simplex", syntax: "simplex(scale)", params: [{ name: "scale", default: 1.0 }], input: "Position", output: "Sdf" },
      { name: "voronoi", syntax: "voronoi(scale)", params: [{ name: "scale", default: 5.0 }], input: "Position", output: "Sdf" },
      { name: "concentric_waves", syntax: "concentric_waves(amplitude, width, frequency)", params: [{ name: "amplitude", default: 1.0 }, { name: "width", default: 0.5 }, { name: "frequency", default: 3.0 }], input: "Position", output: "Sdf" },
    ],
  },
  sdf_to_color: {
    description: "Sdf -> Color: Bridge SDF distance to color output. Place after SDF generators/modifiers.",
    entries: [
      { name: "glow", syntax: "glow(intensity)", params: [{ name: "intensity", default: 1.5 }], input: "Sdf", output: "Color" },
      { name: "shade", syntax: "shade(r, g, b)", params: [{ name: "r", default: 1.0 }, { name: "g", default: 1.0 }, { name: "b", default: 1.0 }], input: "Sdf", output: "Color" },
      { name: "emissive", syntax: "emissive(intensity)", params: [{ name: "intensity", default: 1.0 }], input: "Sdf", output: "Color" },
    ],
  },
  color_processors: {
    description: "Color -> Color: Modify color output. Post-processing and color adjustments.",
    entries: [
      { name: "tint", syntax: "tint(r, g, b)", params: [{ name: "r", default: 1.0 }, { name: "g", default: 1.0 }, { name: "b", default: 1.0 }], input: "Color", output: "Color", note: "Accepts named colors: tint(gold), tint(cyan), etc." },
      { name: "bloom", syntax: "bloom(threshold, strength)", params: [{ name: "threshold", default: 0.3 }, { name: "strength", default: 2.0 }], input: "Color", output: "Color" },
      { name: "grain", syntax: "grain(amount)", params: [{ name: "amount", default: 0.1 }], input: "Color", output: "Color" },
      { name: "blend", syntax: "blend(factor)", params: [{ name: "factor", default: 0.5 }], input: "Color", output: "Color" },
      { name: "vignette", syntax: "vignette(strength, radius)", params: [{ name: "strength", default: 0.5 }, { name: "radius", default: 0.8 }], input: "Color", output: "Color" },
      { name: "tonemap", syntax: "tonemap(exposure)", params: [{ name: "exposure", default: 1.0 }], input: "Color", output: "Color" },
      { name: "scanlines", syntax: "scanlines(frequency, intensity)", params: [{ name: "frequency", default: 200 }, { name: "intensity", default: 0.3 }], input: "Color", output: "Color" },
      { name: "chromatic", syntax: "chromatic(offset)", params: [{ name: "offset", default: 0.005 }], input: "Color", output: "Color" },
      { name: "saturate_color", syntax: "saturate_color(amount)", params: [{ name: "amount", default: 1.0 }], input: "Color", output: "Color" },
      { name: "glitch", syntax: "glitch(intensity)", params: [{ name: "intensity", default: 0.5 }], input: "Color", output: "Color" },
    ],
  },
  position_transforms: {
    description: "Position -> Position: Transform coordinates before SDF evaluation. Place at the start of pipe chains.",
    entries: [
      { name: "translate", syntax: "translate(x, y)", params: [{ name: "x", default: 0.0 }, { name: "y", default: 0.0 }], input: "Position", output: "Position" },
      { name: "rotate", syntax: "rotate(angle)", params: [{ name: "angle", default: 0.0 }], input: "Position", output: "Position", note: "Use time expressions for animation: rotate(time * 0.5)" },
      { name: "scale", syntax: "scale(s)", params: [{ name: "s", default: 1.0 }], input: "Position", output: "Position" },
      { name: "twist", syntax: "twist(amount)", params: [{ name: "amount", default: 0.0 }], input: "Position", output: "Position" },
      { name: "mirror", syntax: "mirror(axis)", params: [{ name: "axis", default: 0.0 }], input: "Position", output: "Position", note: "0=X axis, 1=Y axis" },
      { name: "repeat", syntax: "repeat(count)", params: [{ name: "count", default: 4.0 }], input: "Position", output: "Position" },
      { name: "domain_warp", syntax: "domain_warp(amount, freq)", params: [{ name: "amount", default: 0.1 }, { name: "freq", default: 3.0 }], input: "Position", output: "Position" },
      { name: "curl_noise", syntax: "curl_noise(frequency, amplitude)", params: [{ name: "frequency", default: 1.0 }, { name: "amplitude", default: 0.1 }], input: "Position", output: "Position" },
      { name: "displace", syntax: "displace(strength)", params: [{ name: "strength", default: 0.1 }], input: "Position", output: "Position", note: "Noise-based displacement before SDF evaluation" },
    ],
  },
  sdf_modifiers: {
    description: "Sdf -> Sdf: Modify SDF result after shape generation. Place after SDF generators.",
    entries: [
      { name: "mask_arc", syntax: "mask_arc(angle)", params: [{ name: "angle", default: null }], input: "Sdf", output: "Sdf", note: "Clips SDF to arc sector (0..tau). Required param, no default." },
      { name: "threshold", syntax: "threshold(cutoff)", params: [{ name: "cutoff", default: 0.5 }], input: "Sdf", output: "Sdf" },
      { name: "onion", syntax: "onion(thickness)", params: [{ name: "thickness", default: 0.02 }], input: "Sdf", output: "Sdf", note: "Creates concentric shells from any SDF" },
      { name: "round", syntax: "round(radius)", params: [{ name: "radius", default: 0.02 }], input: "Sdf", output: "Sdf", note: "Rounds sharp corners/edges" },
    ],
  },
  position_to_color: {
    description: "Position -> Color: Full-screen color generators. Bypass SDF stage entirely.",
    entries: [
      { name: "gradient", syntax: "gradient(color_a, color_b, mode)", params: [{ name: "color_a", default: null }, { name: "color_b", default: null }, { name: "mode", default: null }], input: "Position", output: "Color", note: "mode: 'x', 'y', or 'radial'. Colors can be named: gradient(deep_blue, black, \"radial\")" },
      { name: "spectrum", syntax: "spectrum(bass, mid, treble)", params: [{ name: "bass", default: 0.0 }, { name: "mid", default: 0.0 }, { name: "treble", default: 0.0 }], input: "Position", output: "Color", note: "Audio-reactive concentric rings per frequency band" },
    ],
  },
  signals: {
    description: "Real-time signals for parameter modulation via the ~ operator. Syntax: param: base ~ signal * scale",
    entries: [
      { name: "audio.bass", syntax: "~ audio.bass", description: "Low frequency FFT band (0..1)" },
      { name: "audio.mid", syntax: "~ audio.mid", description: "Mid frequency FFT band (0..1)" },
      { name: "audio.treble", syntax: "~ audio.treble", description: "High frequency FFT band (0..1)" },
      { name: "audio.energy", syntax: "~ audio.energy", description: "Overall audio energy (0..1)" },
      { name: "audio.beat", syntax: "~ audio.beat", description: "Beat detection impulse (0 or 1)" },
      { name: "mouse.x", syntax: "~ mouse.x", description: "Normalized cursor X (0..1)" },
      { name: "mouse.y", syntax: "~ mouse.y", description: "Normalized cursor Y (0..1)" },
      { name: "data.*", syntax: "~ data.fieldname", description: "Web Component property binding. E.g., data.value, data.progress" },
      { name: "time", syntax: "time", description: "Elapsed seconds (wraps at 120s). Use in expressions: time * 0.5, sin(time)" },
    ],
  },
  named_colors: {
    description: "Built-in color names for use with tint(), shade(), gradient(). Pass as bare identifiers.",
    entries: [
      { name: "black", rgb: [0.0, 0.0, 0.0] },
      { name: "white", rgb: [1.0, 1.0, 1.0] },
      { name: "red", rgb: [1.0, 0.0, 0.0] },
      { name: "green", rgb: [0.0, 1.0, 0.0] },
      { name: "blue", rgb: [0.0, 0.0, 1.0] },
      { name: "cyan", rgb: [0.0, 1.0, 1.0] },
      { name: "orange", rgb: [1.0, 0.5, 0.0] },
      { name: "gold", rgb: [0.831, 0.686, 0.216] },
      { name: "ember", rgb: [0.8, 0.2, 0.05] },
      { name: "frost", rgb: [0.85, 0.92, 1.0] },
      { name: "ivory", rgb: [1.0, 0.97, 0.92] },
      { name: "midnight", rgb: [0.0, 0.0, 0.1] },
      { name: "obsidian", rgb: [0.04, 0.04, 0.06] },
      { name: "deep_blue", rgb: [0.0, 0.02, 0.15] },
    ],
  },
  language_features: {
    description: "Core language constructs beyond pipe chains.",
    entries: [
      { name: "define", syntax: "define name(params) { stages }", description: "Reusable macro. Expands inline at compile time. E.g., define glow_ring(r, t) { ring(r, t) | glow(2.0) }" },
      { name: "layer", syntax: "layer name { fn: chain }", description: "Named visual layer. Multiple layers composite additively. Params use ~ for modulation." },
      { name: "arc", syntax: "arc { time label { transitions } }", description: "Timeline system. Moments at timestamps with param transitions. Supports ALL keyword. E.g., 0:03 \"expand\" { radius -> 0.5 ease(expo_out) over 2s }" },
      { name: "lens", syntax: "lens { mode: flat | raymarch }", description: "Camera/render mode. Default: flat (2D). Options: raymarch (3D with orbit camera)." },
      { name: "import", syntax: "import \"path\" expose name1, name2", description: "Import defines from external .game files. Supports `expose ALL`." },
      { name: "react", syntax: "react { signal -> action }", description: "Map user inputs to actions. Signals: mouse.click, key(\"x\"), audio.field > threshold. Actions: arc.pause_toggle, arc.restart, param.set(value), param.toggle(a, b), param.bias(amount)." },
      { name: "resonate", syntax: "resonate { a.param ~ b.param * factor, damping: 0.95 }", description: "Cross-layer parameter feedback. Bidirectional coupling. Damping prevents runaway." },
      { name: "math constants", syntax: "pi, tau, e, phi", description: "pi=3.14159, tau=6.28318, e=2.71828, phi=1.61803 (golden ratio)" },
    ],
  },
  easing_functions: {
    description: "Easing functions for arc timeline transitions. Syntax: ease(name)",
    entries: [
      { name: "linear", description: "Constant speed (default)" },
      { name: "smooth", description: "Smooth ease in/out (smoothstep)" },
      { name: "expo_in", description: "Slow start, fast end" },
      { name: "expo_out", description: "Fast start, slow end" },
      { name: "cubic_in_out", description: "Cubic ease in/out" },
      { name: "elastic", description: "Springy overshoot" },
      { name: "bounce", description: "Ball-drop bounce at end" },
    ],
  },
};

// =============================================================================
// Stdlib Data (planned standard library modules)
// =============================================================================

/**
 * Standard library modules available via `import "stdlib/module" expose func1, func2`.
 * Each module provides reusable define macros for common patterns.
 */
const STDLIB_DATA = {
  primitives: {
    description: "Extended shape primitives built from core builtins",
    functions: [
      { name: "rounded_box", signature: "rounded_box(w, h, r)", description: "Box with rounded corners" },
      { name: "hollow_ring", signature: "hollow_ring(radius, width)", description: "Ring with transparent center" },
      { name: "cross_shape", signature: "cross_shape(size, thickness)", description: "Plus/cross shape from two boxes" },
      { name: "gear", signature: "gear(teeth, radius, depth)", description: "Gear shape with teeth around a ring" },
      { name: "soft_dot", signature: "soft_dot(radius)", description: "Circle with very soft glow falloff" },
      { name: "diamond", signature: "diamond(size)", description: "Rotated box (45 degrees)" },
    ],
  },
  noise: {
    description: "Noise-based pattern generators",
    functions: [
      { name: "marble", signature: "marble(scale, distortion)", description: "Marble-like veined texture via warped fbm" },
      { name: "turbulence", signature: "turbulence(scale, octaves)", description: "Absolute-value fbm for turbulent patterns" },
      { name: "cloud", signature: "cloud(scale, softness)", description: "Soft cloud-like noise field" },
      { name: "cellular", signature: "cellular(scale)", description: "Voronoi-based cellular pattern" },
      { name: "flow", signature: "flow(scale, speed)", description: "Animated flowing noise" },
    ],
  },
  post: {
    description: "Preset post-processing chains",
    functions: [
      { name: "cinematic_grade", signature: "cinematic_grade()", description: "Tonemap + vignette + subtle grain" },
      { name: "retro_crt", signature: "retro_crt()", description: "Scanlines + chromatic + vignette" },
      { name: "dream_glow", signature: "dream_glow()", description: "Heavy bloom + soft grain" },
      { name: "noir", signature: "noir()", description: "Desaturated + high contrast + grain" },
      { name: "glitch_fx", signature: "glitch_fx()", description: "Glitch + chromatic + scanlines" },
    ],
  },
  backgrounds: {
    description: "Full-screen background generators",
    functions: [
      { name: "starfield", signature: "starfield(density, speed)", description: "Animated star field background" },
      { name: "nebula", signature: "nebula(color_a, color_b)", description: "Nebula-like gradient with fbm overlay" },
      { name: "gradient_bg", signature: "gradient_bg(color_a, color_b)", description: "Simple vertical gradient background" },
      { name: "radial_bg", signature: "radial_bg(center_color, edge_color)", description: "Radial gradient from center" },
      { name: "noise_bg", signature: "noise_bg(scale, color)", description: "Subtle noise-textured background" },
    ],
  },
  transitions: {
    description: "Animated transition effects for arc timelines",
    functions: [
      { name: "fade_circle", signature: "fade_circle(progress)", description: "Circular wipe from center" },
      { name: "dissolve_ring", signature: "dissolve_ring(progress)", description: "Ring dissolution transition" },
      { name: "bloom_wipe", signature: "bloom_wipe(progress)", description: "Bloom-based reveal" },
      { name: "shatter", signature: "shatter(progress)", description: "Voronoi-based shatter effect" },
      { name: "ripple", signature: "ripple(progress)", description: "Concentric wave reveal" },
    ],
  },
  ui: {
    description: "UI-oriented components (loading, progress, badges)",
    functions: [
      { name: "loading_spinner", signature: "loading_spinner(speed, color)", description: "Rotating arc loading indicator" },
      { name: "progress_ring", signature: "progress_ring(progress, color)", description: "Ring that fills based on progress (0-1)" },
      { name: "pulse_dot", signature: "pulse_dot(color)", description: "Breathing dot indicator" },
      { name: "metric_ring", signature: "metric_ring(value, max, color)", description: "Data-driven metric ring" },
      { name: "badge", signature: "badge(shape, color)", description: "Achievement/status badge" },
    ],
  },
  patterns: {
    description: "Repeating geometric patterns",
    functions: [
      { name: "checkerboard", signature: "checkerboard(scale)", description: "Classic checkerboard" },
      { name: "stripes", signature: "stripes(angle, width)", description: "Angled stripe pattern" },
      { name: "dots", signature: "dots(scale, radius)", description: "Dot grid pattern" },
      { name: "hexgrid", signature: "hexgrid(scale)", description: "Hexagonal grid" },
      { name: "concentric_rings", signature: "concentric_rings(count, width)", description: "Evenly spaced concentric rings" },
      { name: "spiral", signature: "spiral(arms, tightness)", description: "Spiral pattern" },
      { name: "wave_pattern", signature: "wave_pattern(frequency, amplitude)", description: "Sine wave pattern" },
      { name: "grid_lines", signature: "grid_lines(spacing, thickness)", description: "Grid line overlay" },
    ],
  },
  motion: {
    description: "Animation primitives for parameter modulation",
    functions: [
      { name: "orbit_motion", signature: "orbit_motion(radius, speed)", description: "Circular orbit path" },
      { name: "pendulum", signature: "pendulum(amplitude, speed)", description: "Pendulum swing" },
      { name: "bounce_motion", signature: "bounce_motion(height, speed)", description: "Bouncing motion" },
      { name: "pulse", signature: "pulse(speed, depth)", description: "Rhythmic pulse" },
      { name: "drift", signature: "drift(speed_x, speed_y)", description: "Slow directional drift" },
      { name: "spin", signature: "spin(speed)", description: "Continuous rotation" },
      { name: "breathe", signature: "breathe(speed, depth)", description: "Gentle breathing animation" },
      { name: "flicker", signature: "flicker(speed, randomness)", description: "Candle-like flicker" },
    ],
  },
  color: {
    description: "Color palette and gradient presets",
    functions: [
      { name: "warm_glow", signature: "warm_glow(intensity)", description: "Warm amber/gold glow" },
      { name: "cool_glow", signature: "cool_glow(intensity)", description: "Cool blue/cyan glow" },
      { name: "fire", signature: "fire(intensity)", description: "Fire color palette (red -> orange -> yellow)" },
      { name: "ice", signature: "ice(intensity)", description: "Ice color palette (white -> blue -> deep blue)" },
      { name: "ocean", signature: "ocean(depth)", description: "Ocean color palette (cyan -> deep blue)" },
      { name: "neon", signature: "neon(hue)", description: "Neon color by hue angle" },
      { name: "sunset_gradient", signature: "sunset_gradient()", description: "Sunset color gradient" },
      { name: "northern_lights", signature: "northern_lights()", description: "Aurora-like color shift" },
      { name: "lava", signature: "lava(intensity)", description: "Lava color palette (dark red -> orange -> yellow)" },
      { name: "crystal", signature: "crystal(facets)", description: "Crystalline color refraction" },
    ],
  },
  audio: {
    description: "Audio-reactive visual patterns",
    functions: [
      { name: "beat_ring", signature: "beat_ring(radius, color)", description: "Ring that pulses on beat" },
      { name: "spectrum_bars", signature: "spectrum_bars(count, color)", description: "Vertical frequency bars" },
      { name: "bass_pulse", signature: "bass_pulse(shape, color)", description: "Shape that pulses with bass" },
      { name: "treble_scatter", signature: "treble_scatter(count, color)", description: "Particles scattered by treble" },
      { name: "energy_field", signature: "energy_field(color)", description: "Noise field modulated by energy" },
      { name: "rhythm_ring", signature: "rhythm_ring(radius, color)", description: "Ring with rhythm-synced segments" },
      { name: "frequency_glow", signature: "frequency_glow(band, color)", description: "Glow intensity tied to frequency band" },
      { name: "audio_terrain", signature: "audio_terrain(color)", description: "Terrain-like visualization of audio" },
    ],
  },
  effects: {
    description: "Complex visual effects composed from multiple builtins",
    functions: [
      { name: "electric", signature: "electric(intensity, color)", description: "Electric arc/lightning effect" },
      { name: "plasma_field", signature: "plasma_field(scale, speed)", description: "Animated plasma field" },
      { name: "smoke", signature: "smoke(density, speed)", description: "Rising smoke effect" },
      { name: "hologram", signature: "hologram(color)", description: "Holographic scan lines + flicker" },
      { name: "interference", signature: "interference(frequency)", description: "Wave interference pattern" },
      { name: "caustics", signature: "caustics(scale, speed)", description: "Water caustics light pattern" },
      { name: "static_noise", signature: "static_noise(intensity)", description: "TV static noise" },
      { name: "retro_screen", signature: "retro_screen(color)", description: "CRT monitor effect" },
      { name: "dream_haze", signature: "dream_haze(intensity)", description: "Dreamy soft-focus haze" },
      { name: "void_pulse", signature: "void_pulse(color)", description: "Dark pulsing void effect" },
    ],
  },
};

// =============================================================================
// Presets Data
// =============================================================================

/**
 * Preset configurations: complete .game snippets for common use cases.
 * Grouped by category. Each preset is a named starting point.
 */
const PRESETS_DATA = {
  ambient: {
    description: "Ambient background visuals for idle/decorative use",
    presets: ["breathing-orb", "slow-nebula", "starfield-drift", "gradient-pulse", "noise-shimmer"],
  },
  ui: {
    description: "UI components for dashboards and app interfaces",
    presets: ["progress-ring", "loading-spinner", "status-orb", "metric-gauge", "notification-pulse", "health-bar"],
  },
  audio: {
    description: "Audio-reactive visualizations",
    presets: ["spectrum-ring", "bass-pulse", "beat-circles", "frequency-bars", "energy-field", "audio-landscape"],
  },
  cinematic: {
    description: "Full-screen cinematic effects with timelines",
    presets: ["galaxy-spin", "neon-grid", "fire-ice-duality", "kaleidoscope", "particle-storm", "ocean-depth"],
  },
  transition: {
    description: "Transition effects for scene changes",
    presets: ["circle-wipe", "dissolve", "bloom-reveal", "shatter-out", "ripple-fade"],
  },
  generative: {
    description: "Procedural generative art patterns",
    presets: ["voronoi-crystal", "fbm-landscape", "spiral-tunnel", "fractal-tree", "interference-pattern"],
  },
  data: {
    description: "Data-driven visualizations bound to external values",
    presets: ["data-ring", "completion-burst", "level-indicator", "score-display", "achievement-badge"],
  },
};

// =============================================================================
// Lint Helpers — static analysis and error suggestions
// =============================================================================

/** All 37 builtin names from the compiler */
const ALL_BUILTIN_NAMES = new Set([
  "circle", "ring", "star", "box", "polygon", "fbm", "simplex", "voronoi", "concentric_waves",
  "glow", "shade", "emissive",
  "tint", "bloom", "grain", "blend", "vignette", "tonemap", "scanlines", "chromatic", "saturate_color", "glitch",
  "translate", "rotate", "scale", "twist", "mirror", "repeat", "domain_warp", "curl_noise", "displace",
  "mask_arc", "threshold", "onion", "round",
  "gradient", "spectrum",
]);

/** Common misspellings / removed builtins that people try to use */
const BUILTIN_SUGGESTIONS: Record<string, string> = {
  "sphere": "circle (GAME is 2D — use circle for round shapes)",
  "torus": "ring (use ring(radius, width) for torus-like shapes)",
  "line": "box (use a thin box for line shapes, or two translate + circle for endpoints)",
  "fog": "vignette (fog was removed — use vignette for depth darkening)",
  "invert": "saturate_color(0.0) for desaturation, or use shade with inverted colors",
  "iridescent": "chromatic (iridescent was removed — use chromatic for color shifting)",
  "colormap": "gradient (use gradient(color_a, color_b, mode) for color mapping)",
  "particles": "Use fbm or voronoi with repeat for particle-like effects",
  "colour": "color (use American English spelling)",
  "glow_ring": "This is typically a define — use: ring(r, w) | glow(intensity)",
};

interface LintSuggestion {
  line: number | null;
  level: "info" | "warning";
  message: string;
}

/**
 * Static analysis of .game source for common mistakes.
 * Runs without the compiler — purely pattern-based.
 */
function lintSource(source: string): LintSuggestion[] {
  const suggestions: LintSuggestion[] = [];
  const lines = source.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineNum = i + 1;
    const trimmed = line.trim();

    // Skip comments
    if (trimmed.startsWith("#") || trimmed.startsWith("//")) continue;

    // Check for pipe chains with wrong ordering
    const pipeMatch = trimmed.match(/^fn:\s*(.+)/);
    if (pipeMatch) {
      const chain = pipeMatch[1];
      const stages = chain.split("|").map((s) => s.trim());
      let lastState = "Position";
      for (const stage of stages) {
        const funcName = stage.match(/^(\w+)/)?.[1];
        if (!funcName) continue;

        // Check for unknown builtins
        if (!ALL_BUILTIN_NAMES.has(funcName) && funcName !== "p") {
          const suggestion = BUILTIN_SUGGESTIONS[funcName];
          if (suggestion) {
            suggestions.push({
              line: lineNum,
              level: "warning",
              message: `'${funcName}' is not a builtin. Did you mean: ${suggestion}`,
            });
          }
        }

        // Check for common state mismatches
        if (funcName === "glow" && lastState === "Color") {
          suggestions.push({
            line: lineNum,
            level: "warning",
            message: "glow() expects Sdf input but is placed after a Color stage. Move it before tint/bloom/etc.",
          });
        }
        if ((funcName === "translate" || funcName === "rotate" || funcName === "scale") && lastState === "Sdf") {
          suggestions.push({
            line: lineNum,
            level: "warning",
            message: `${funcName}() is a Position->Position transform but appears after an SDF stage. Place domain ops before shapes.`,
          });
        }

        // Track state
        if (["circle", "ring", "star", "box", "polygon", "fbm", "simplex", "voronoi", "concentric_waves"].includes(funcName!)) {
          lastState = "Sdf";
        } else if (["glow", "shade", "emissive", "gradient", "spectrum"].includes(funcName!)) {
          lastState = "Color";
        } else if (["tint", "bloom", "grain", "blend", "vignette", "tonemap", "scanlines", "chromatic", "saturate_color", "glitch"].includes(funcName!)) {
          lastState = "Color";
        }
      }
    }

    // Check for blend as pass name (reserved keyword)
    if (trimmed.match(/^pass\s+blend\b/)) {
      suggestions.push({
        line: lineNum,
        level: "warning",
        message: "'blend' is a keyword — do not use it as a pass name. Choose a different name.",
      });
    }

    // Check for WGSL mod gotcha in expressions
    if (trimmed.includes(" % ") && !trimmed.startsWith("#")) {
      suggestions.push({
        line: lineNum,
        level: "info",
        message: "WGSL % is truncation-based (not floor-based like GLSL mod). Use game_mod() for floor-based modulo.",
      });
    }
  }

  // Check for missing cinematic block
  if (!source.includes("cinematic")) {
    suggestions.push({
      line: null,
      level: "warning",
      message: "No 'cinematic' block found. Every .game file should have: cinematic \"Title\" { ... }",
    });
  }

  return suggestions;
}

/**
 * Generate helpful suggestions based on compiler error messages.
 */
function getErrorSuggestions(errorOutput: string): LintSuggestion[] {
  const suggestions: LintSuggestion[] = [];
  const lower = errorOutput.toLowerCase();

  if (lower.includes("unexpected token")) {
    suggestions.push({
      line: null,
      level: "info",
      message: "Check for missing commas between parameters, unclosed braces, or incorrect pipe chain syntax.",
    });
  }

  if (lower.includes("type mismatch") || lower.includes("state mismatch")) {
    suggestions.push({
      line: null,
      level: "info",
      message: "Type state mismatch. Pipeline order: Position->Position (domain ops) | Position->Sdf (shapes) | Sdf->Sdf (modifiers) | Sdf->Color (glow/shade) | Color->Color (post fx). Check your pipe chain order.",
    });
  }

  if (lower.includes("unknown function") || lower.includes("unknown builtin")) {
    const nameMatch = errorOutput.match(/unknown (?:function|builtin)\s+['"`]?(\w+)/i);
    if (nameMatch) {
      const name = nameMatch[1];
      const suggestion = BUILTIN_SUGGESTIONS[name];
      if (suggestion) {
        suggestions.push({
          line: null,
          level: "info",
          message: `'${name}' is not available. Try: ${suggestion}`,
        });
      } else {
        suggestions.push({
          line: null,
          level: "info",
          message: `'${name}' is not a recognized builtin. Use list_primitives to see all 37 available builtins.`,
        });
      }
    }
  }

  if (lower.includes("expected") && lower.includes("cinematic")) {
    suggestions.push({
      line: null,
      level: "info",
      message: "Every .game file must start with: cinematic \"Title\" { ... }",
    });
  }

  return suggestions;
}

// =============================================================================
// Server Setup
// =============================================================================

const server = new Server(
  {
    name: "game-server",
    version: "0.3.0",
  },
  {
    capabilities: {
      tools: {},
      resources: {},
      prompts: {},
    },
  }
);

// =============================================================================
// Tools
// =============================================================================

server.setRequestHandler(ListToolsRequestSchema, async () => {
  return {
    tools: [
      {
        name: "compile",
        description:
          "Compile .game source code into WebGPU shader (WGSL), self-contained HTML, or ES module Web Component. " +
          "Writes source to a temp file, invokes the GAME compiler, and returns the compiled output. " +
          "On compiler errors, returns the error message with line/column information.",
        inputSchema: {
          type: "object" as const,
          properties: {
            source: {
              type: "string",
              description: "The .game DSL source code to compile",
            },
            format: {
              type: "string",
              enum: ["wgsl", "html", "component"],
              description: "Output format: 'wgsl' for raw WGSL shader, 'html' for self-contained HTML page, 'component' for ES module Web Component (default: 'component')",
            },
            tag: {
              type: "string",
              description: "Custom HTML element tag name for component format (e.g., 'my-shader'). Only used when format is 'component'.",
            },
          },
          required: ["source"],
        },
      },
      {
        name: "validate",
        description:
          "Check .game source code for syntax and semantic errors without returning the full compiled output. " +
          "Fast validation pass that attempts WGSL compilation and checks the exit code.",
        inputSchema: {
          type: "object" as const,
          properties: {
            source: {
              type: "string",
              description: "The .game DSL source code to validate",
            },
          },
          required: ["source"],
        },
      },
      {
        name: "lint",
        description:
          "Lint .game source code — validates syntax/semantics AND surfaces structured warnings " +
          "(e.g., unresolvable arc targets, unrecognized react signals, unused lens properties). " +
          "Returns { valid, warnings[], error? }.",
        inputSchema: {
          type: "object" as const,
          properties: {
            source: {
              type: "string",
              description: "The .game DSL source code to lint",
            },
          },
          required: ["source"],
        },
      },
      {
        name: "list_primitives",
        description:
          "Return all 37 GAME language builtins organized by type-state transition " +
          "(Position->Sdf, Sdf->Color, Color->Color, Position->Position, Sdf->Sdf, Position->Color). " +
          "Each entry includes exact parameter names, defaults, and input/output states. " +
          "Also includes named colors, signals, language features, and easing functions.",
        inputSchema: {
          type: "object" as const,
          properties: {
            category: {
              type: "string",
              enum: [
                "all",
                "sdf_generators",
                "sdf_to_color",
                "color_processors",
                "position_transforms",
                "sdf_modifiers",
                "position_to_color",
                "signals",
                "named_colors",
                "language_features",
                "easing_functions",
              ],
              description: "Filter by category (default: 'all'). Use specific categories to reduce output size.",
            },
          },
        },
      },
      {
        name: "list_stdlib",
        description:
          "Return all GAME standard library modules and their exported functions. " +
          "Stdlib modules are imported via `import \"stdlib/module\" expose func1, func2`. " +
          "Covers: primitives, noise, post, backgrounds, transitions, ui, patterns, motion, color, audio, effects.",
        inputSchema: {
          type: "object" as const,
          properties: {
            module: {
              type: "string",
              enum: [
                "all",
                "primitives",
                "noise",
                "post",
                "backgrounds",
                "transitions",
                "ui",
                "patterns",
                "motion",
                "color",
                "audio",
                "effects",
              ],
              description: "Filter by module name (default: 'all'). Returns only the specified module.",
            },
          },
        },
      },
      {
        name: "list_presets",
        description:
          "Return all available GAME preset names grouped by category " +
          "(ambient, ui, audio, cinematic, transition, generative, data). " +
          "Presets are complete .game file starting points for common use cases.",
        inputSchema: {
          type: "object" as const,
          properties: {
            category: {
              type: "string",
              enum: ["all", "ambient", "ui", "audio", "cinematic", "transition", "generative", "data"],
              description: "Filter by category (default: 'all').",
            },
          },
        },
      },
    ],
  };
});

server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  try {
    switch (name) {
      // -----------------------------------------------------------------------
      // compile
      // -----------------------------------------------------------------------
      case "compile": {
        const source = (args as Record<string, unknown>)?.source;
        if (typeof source !== "string" || source.trim().length === 0) {
          throw new Error("'source' is required and must be a non-empty string");
        }

        const format = ((args as Record<string, unknown>)?.format as string) || "component";
        const tag = (args as Record<string, unknown>)?.tag as string | undefined;

        if (!["wgsl", "html", "component"].includes(format)) {
          throw new Error(`Invalid format '${format}'. Must be one of: wgsl, html, component`);
        }

        if (tag && format !== "component") {
          throw new Error("'tag' parameter is only valid when format is 'component'");
        }

        if (tag && !/^[a-z][a-z0-9]*(-[a-z0-9]+)+$/.test(tag)) {
          throw new Error(
            `Invalid custom element tag '${tag}'. Must contain a hyphen and use lowercase letters/numbers (e.g., 'my-shader').`
          );
        }

        // Verify compiler exists
        if (!existsSync(COMPILER_PATH)) {
          throw new Error(
            `GAME compiler not found at: ${COMPILER_PATH}\n` +
            `Set the GAME_COMPILER_PATH environment variable to the correct path.`
          );
        }

        const tempFile = await writeTempGameFile(source);
        try {
          const compilerArgs = ["compile", tempFile];
          if (format === "html") {
            compilerArgs.push("--html");
          } else if (format === "component") {
            compilerArgs.push("--component");
            if (tag) {
              compilerArgs.push("--tag", tag);
            }
          }
          // format === "wgsl" uses no extra flags (default output)

          const result = await runCompiler(compilerArgs);

          if (result.exitCode !== 0) {
            const errorOutput = result.stderr.trim() || result.stdout.trim() || "Unknown compiler error";
            return {
              content: [
                {
                  type: "text",
                  text: JSON.stringify(
                    {
                      success: false,
                      error: errorOutput,
                      format,
                    },
                    null,
                    2
                  ),
                },
              ],
              isError: true,
            };
          }

          return {
            content: [
              {
                type: "text",
                text: JSON.stringify(
                  {
                    success: true,
                    format,
                    tag: tag || undefined,
                    output: result.stdout,
                    bytesGenerated: result.stdout.length,
                  },
                  null,
                  2
                ),
              },
            ],
          };
        } finally {
          await cleanupTempFile(tempFile);
        }
      }

      // -----------------------------------------------------------------------
      // validate
      // -----------------------------------------------------------------------
      case "validate": {
        const source = (args as Record<string, unknown>)?.source;
        if (typeof source !== "string" || source.trim().length === 0) {
          throw new Error("'source' is required and must be a non-empty string");
        }

        if (!existsSync(COMPILER_PATH)) {
          throw new Error(
            `GAME compiler not found at: ${COMPILER_PATH}\n` +
            `Set the GAME_COMPILER_PATH environment variable to the correct path.`
          );
        }

        const tempFile = await writeTempGameFile(source);
        try {
          // Use WGSL output (fastest, no HTML/component wrapping overhead)
          const result = await runCompiler(["compile", tempFile]);

          if (result.exitCode === 0) {
            return {
              content: [
                {
                  type: "text",
                  text: JSON.stringify({ valid: true }, null, 2),
                },
              ],
            };
          } else {
            const errorOutput = result.stderr.trim() || result.stdout.trim() || "Unknown error";
            return {
              content: [
                {
                  type: "text",
                  text: JSON.stringify(
                    {
                      valid: false,
                      error: errorOutput,
                    },
                    null,
                    2
                  ),
                },
              ],
            };
          }
        } finally {
          await cleanupTempFile(tempFile);
        }
      }

      // -----------------------------------------------------------------------
      // lint
      // -----------------------------------------------------------------------
      case "lint": {
        const source = (args as Record<string, unknown>)?.source;
        if (typeof source !== "string" || source.trim().length === 0) {
          throw new Error("'source' is required and must be a non-empty string");
        }

        if (!existsSync(COMPILER_PATH)) {
          throw new Error(
            `GAME compiler not found at: ${COMPILER_PATH}\n` +
            `Set the GAME_COMPILER_PATH environment variable to the correct path.`
          );
        }

        const tempFile = await writeTempGameFile(source);
        try {
          const result = await runCompiler(["compile", tempFile]);

          // Parse warnings from stderr (compiler prints "warning: <msg>" lines)
          const warningLines = result.stderr
            .split("\n")
            .filter((line) => line.toLowerCase().startsWith("warning:"));

          const warnings = warningLines.map((line) => {
            const msg = line.replace(/^warning:\s*/i, "").trim();
            // Try to extract line numbers from patterns like "line 5:" or "[5:12]"
            const lineMatch = msg.match(/(?:line\s+(\d+)|\[(\d+):(\d+)\])/);
            return {
              message: msg,
              line: lineMatch ? parseInt(lineMatch[1] || lineMatch[2]) : null,
              column: lineMatch && lineMatch[3] ? parseInt(lineMatch[3]) : null,
            };
          });

          // Run static analysis on the source for common issues
          const suggestions = lintSource(source);

          if (result.exitCode === 0) {
            return {
              content: [
                {
                  type: "text",
                  text: JSON.stringify(
                    {
                      valid: true,
                      warnings,
                      suggestions,
                      warningCount: warnings.length,
                      suggestionCount: suggestions.length,
                    },
                    null,
                    2
                  ),
                },
              ],
            };
          } else {
            const errorLines = result.stderr
              .split("\n")
              .filter((line) => !line.toLowerCase().startsWith("warning:"));
            const errorOutput = errorLines.join("\n").trim() || result.stdout.trim() || "Unknown error";

            // Try to extract structured error info
            const errorLineMatch = errorOutput.match(/(?:line\s+(\d+)|\[(\d+):(\d+)\]|:(\d+):(\d+))/);
            const errorInfo: Record<string, unknown> = {
              valid: false,
              error: errorOutput,
              errorLine: errorLineMatch ? parseInt(errorLineMatch[1] || errorLineMatch[2] || errorLineMatch[4]) : null,
              errorColumn: errorLineMatch ? parseInt(errorLineMatch[3] || errorLineMatch[5]) || null : null,
              warnings,
              suggestions: [
                ...suggestions,
                ...getErrorSuggestions(errorOutput),
              ],
              warningCount: warnings.length,
            };

            return {
              content: [
                {
                  type: "text",
                  text: JSON.stringify(errorInfo, null, 2),
                },
              ],
            };
          }
        } finally {
          await cleanupTempFile(tempFile);
        }
      }

      // -----------------------------------------------------------------------
      // list_primitives
      // -----------------------------------------------------------------------
      case "list_primitives": {
        const category = ((args as Record<string, unknown>)?.category as string) || "all";

        let data: Record<string, unknown>;
        if (category === "all") {
          data = PRIMITIVES_DATA;
        } else if (category in PRIMITIVES_DATA) {
          data = { [category]: (PRIMITIVES_DATA as Record<string, unknown>)[category] };
        } else {
          throw new Error(
            `Unknown category '${category}'. Valid: all, sdf_generators, sdf_to_color, ` +
            `color_processors, position_transforms, sdf_modifiers, position_to_color, ` +
            `signals, named_colors, language_features, easing_functions`
          );
        }

        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(data, null, 2),
            },
          ],
        };
      }

      // -----------------------------------------------------------------------
      // list_stdlib
      // -----------------------------------------------------------------------
      case "list_stdlib": {
        const module = ((args as Record<string, unknown>)?.module as string) || "all";

        let data: Record<string, unknown>;
        if (module === "all") {
          data = STDLIB_DATA;
        } else if (module in STDLIB_DATA) {
          data = { [module]: (STDLIB_DATA as Record<string, unknown>)[module] };
        } else {
          throw new Error(
            `Unknown stdlib module '${module}'. Valid: all, primitives, noise, post, ` +
            `backgrounds, transitions, ui, patterns, motion, color, audio, effects`
          );
        }

        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(data, null, 2),
            },
          ],
        };
      }

      // -----------------------------------------------------------------------
      // list_presets
      // -----------------------------------------------------------------------
      case "list_presets": {
        const presetCategory = ((args as Record<string, unknown>)?.category as string) || "all";

        let data: Record<string, unknown>;
        if (presetCategory === "all") {
          data = PRESETS_DATA;
        } else if (presetCategory in PRESETS_DATA) {
          data = { [presetCategory]: (PRESETS_DATA as Record<string, unknown>)[presetCategory] };
        } else {
          throw new Error(
            `Unknown preset category '${presetCategory}'. Valid: all, ambient, ui, audio, ` +
            `cinematic, transition, generative, data`
          );
        }

        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(data, null, 2),
            },
          ],
        };
      }

      default:
        throw new Error(`Unknown tool: ${name}`);
    }
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    return {
      content: [
        {
          type: "text",
          text: JSON.stringify({ error: errorMessage }, null, 2),
        },
      ],
      isError: true,
    };
  }
});

// =============================================================================
// Resources
// =============================================================================

server.setRequestHandler(ListResourcesRequestSchema, async () => {
  return {
    resources: [
      {
        uri: "game://language-reference",
        name: "GAME Language Reference",
        description: "The .game DSL language specification: syntax, core concepts (fields, pipes, modulation, lenses, arcs, resonance), grammar, and compilation model.",
        mimeType: "text/markdown",
      },
      {
        uri: "game://primitives",
        name: "GAME Primitives Reference",
        description: "All built-in primitives: SDF shapes, boolean ops, domain ops, noise functions, shading, post-processing, camera, math, and signals.",
        mimeType: "text/markdown",
      },
      {
        uri: "game://examples",
        name: "GAME Example Files",
        description: "Example .game files demonstrating basic shapes, audio reactivity, interactivity, and resonance.",
        mimeType: "text/markdown",
      },
    ],
  };
});

server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
  const uri = request.params.uri;

  switch (uri) {
    case "game://language-reference": {
      const languagePath = join(GAME_ROOT, "LANGUAGE.md");
      if (!existsSync(languagePath)) {
        throw new Error(`Language reference not found at: ${languagePath}`);
      }
      const content = await readFile(languagePath, "utf-8");
      return {
        contents: [
          {
            uri,
            mimeType: "text/markdown",
            text: content,
          },
        ],
      };
    }

    case "game://primitives": {
      const primitivesPath = join(GAME_ROOT, "PRIMITIVES.md");
      if (!existsSync(primitivesPath)) {
        throw new Error(`Primitives reference not found at: ${primitivesPath}`);
      }
      const content = await readFile(primitivesPath, "utf-8");
      return {
        contents: [
          {
            uri,
            mimeType: "text/markdown",
            text: content,
          },
        ],
      };
    }

    case "game://examples": {
      const examplesDir = join(GAME_ROOT, "examples");
      if (!existsSync(examplesDir)) {
        throw new Error(`Examples directory not found at: ${examplesDir}`);
      }

      const files = await readdir(examplesDir);
      const gameFiles = files.filter((f) => f.endsWith(".game")).sort();

      const sections: string[] = ["# GAME Examples\n"];
      for (const file of gameFiles) {
        const filePath = join(examplesDir, file);
        const content = await readFile(filePath, "utf-8");
        sections.push(`## ${file}\n\n\`\`\`game\n${content.trim()}\n\`\`\`\n`);
      }

      return {
        contents: [
          {
            uri,
            mimeType: "text/markdown",
            text: sections.join("\n"),
          },
        ],
      };
    }

    default:
      throw new Error(`Unknown resource URI: ${uri}`);
  }
});

// =============================================================================
// Prompts
// =============================================================================

server.setRequestHandler(ListPromptsRequestSchema, async () => {
  return {
    prompts: [
      {
        name: "generate-component",
        description:
          "Generate a .game file from a natural language description. " +
          "Includes language syntax reference, available primitives, and examples " +
          "to guide the generation of valid .game source code.",
        arguments: [
          {
            name: "description",
            description: "Natural language description of the desired visual effect or animation",
            required: true,
          },
        ],
      },
      {
        name: "iterate-component",
        description:
          "Refine existing .game source code based on natural language feedback. " +
          "Preserves working parts and applies targeted modifications. " +
          "Includes language reference for context.",
        arguments: [
          {
            name: "source",
            description: "The current .game source code to modify",
            required: true,
          },
          {
            name: "feedback",
            description: "Natural language description of what to change (e.g., 'make it glow more', 'add a blue tint', 'slow down the animation')",
            required: true,
          },
        ],
      },
      {
        name: "describe-component",
        description:
          "Describe what a .game visual effect does in plain English. " +
          "Explains layers, parameters, modulation, timeline events, and overall aesthetic.",
        arguments: [
          {
            name: "source",
            description: "The .game source code to describe",
            required: true,
          },
        ],
      },
      {
        name: "generate-4da-component",
        description:
          "Generate a .game component tuned for the 4DA desktop app. " +
          "Produces small, data-driven components with gold/dark theme, " +
          "no audio signals, and subtle animations suited for UI integration.",
        arguments: [
          {
            name: "description",
            description:
              "What the component should visualize (e.g., 'progress ring for task completion', 'status indicator that breathes when active')",
            required: true,
          },
        ],
      },
      {
        name: "generate-achievement-visual",
        description:
          "Generate a .game component for achievement/progression UI. " +
          "Produces progress rings, unlock glows, and badge visuals " +
          "driven by data.progress and data.unlocked signals.",
        arguments: [
          {
            name: "description",
            description:
              "What the achievement visual should show (e.g., 'circular progress ring with gold glow on unlock', 'star badge that scales with completion')",
            required: true,
          },
        ],
      },
      {
        name: "generate-game-indicator",
        description:
          "Generate a .game component for status indicators, health bars, " +
          "and XP gauges. Produces compact, data-driven visual indicators " +
          "using data.value and data.status signals.",
        arguments: [
          {
            name: "description",
            description:
              "What the indicator should visualize (e.g., 'health orb that dims when low', 'XP bar with glow at fill milestones')",
            required: true,
          },
        ],
      },
    ],
  };
});

/**
 * Try to load a prompt template from the prompts/ directory.
 * Returns the file contents with {{placeholders}} substituted, or null if the file doesn't exist.
 */
async function loadPromptFile(
  name: string,
  vars: Record<string, string>,
): Promise<string | null> {
  const promptPath = join(GAME_ROOT, "mcp-game-server", "prompts", `${name}.md`);
  if (!existsSync(promptPath)) return null;
  let content = await readFile(promptPath, "utf-8");
  for (const [key, value] of Object.entries(vars)) {
    content = content.replaceAll(`{{${key}}}`, value);
  }
  return content;
}

/**
 * Load reference material (language spec, primitives, examples) for prompt context.
 * Returns { languageRef, primitivesRef, examplesRef } with empty strings if files are missing.
 */
async function loadReferenceContext(): Promise<{
  languageRef: string;
  primitivesRef: string;
  examplesRef: string;
}> {
  let languageRef = "";
  let primitivesRef = "";
  let examplesRef = "";

  const languagePath = join(GAME_ROOT, "LANGUAGE.md");
  const primitivesPath = join(GAME_ROOT, "PRIMITIVES.md");
  const examplesDir = join(GAME_ROOT, "examples");

  if (existsSync(languagePath)) {
    languageRef = await readFile(languagePath, "utf-8");
  }
  if (existsSync(primitivesPath)) {
    primitivesRef = await readFile(primitivesPath, "utf-8");
  }
  if (existsSync(examplesDir)) {
    const files = await readdir(examplesDir);
    const gameFiles = files.filter((f) => f.endsWith(".game")).sort();
    const sections: string[] = [];
    for (const file of gameFiles) {
      const content = await readFile(join(examplesDir, file), "utf-8");
      sections.push(`### ${file}\n\`\`\`game\n${content.trim()}\n\`\`\``);
    }
    examplesRef = sections.join("\n\n");
  }

  return { languageRef, primitivesRef, examplesRef };
}

server.setRequestHandler(GetPromptRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  switch (name) {
    // -----------------------------------------------------------------------
    // generate-component
    // -----------------------------------------------------------------------
    case "generate-component": {
      const description = args?.description;
      if (typeof description !== "string" || description.trim().length === 0) {
        throw new Error("'description' argument is required");
      }

      // Try loading from prompts/generate.md first
      const promptFile = await loadPromptFile("generate", { description });
      if (promptFile) {
        return {
          messages: [{ role: "user", content: { type: "text", text: promptFile } }],
        };
      }

      // Inline fallback
      const { languageRef, primitivesRef, examplesRef } = await loadReferenceContext();

      return {
        messages: [
          {
            role: "user",
            content: {
              type: "text",
              text: `You are a GAME language expert. Generate a .game file that creates the following visual effect:

**Description:** ${description}

## Type State Pipeline

GAME uses a type-state pipeline enforced at compile time:
- Position -> Position: translate, rotate, scale, twist, mirror, repeat, domain_warp, curl_noise, displace
- Position -> Sdf: circle, ring, star, box, polygon, fbm, simplex, voronoi, concentric_waves
- Sdf -> Sdf: mask_arc, threshold, onion, round
- Sdf -> Color: glow, shade, emissive
- Position -> Color: gradient, spectrum (bypasses SDF)
- Color -> Color: tint, bloom, grain, blend, vignette, tonemap, scanlines, chromatic, saturate_color, glitch

Chain order: domain ops | shapes | modifiers | bridge | post-processing

## Guidelines

1. Start with a \`cinematic "Title" { ... }\` block
2. Define layers with \`fn:\` pipe chains following the type-state pipeline
3. Use modulation (\`~\`) to make parameters react to signals (time, audio, mouse, data.*)
4. Include post-processing effects for visual polish (bloom, vignette, grain)
5. Use descriptive names for layers and parameters
6. Keep it focused — a good effect is simple but expressive
7. Use \`define\` for reusable patterns when you repeat similar pipe chains
8. Use \`resonate\` for cross-layer feedback when multiple layers should interact
9. Use \`react\` to map user inputs (mouse, keyboard) to actions
10. Named colors: black, white, red, green, blue, cyan, orange, gold, ember, frost, ivory, midnight, obsidian, deep_blue

## Output Format

Return ONLY the .game source code inside a single code block. No explanation before or after.

---

## Language Reference

${languageRef}

---

## Available Primitives

${primitivesRef}

---

## Examples

${examplesRef}`,
            },
          },
        ],
      };
    }

    // -----------------------------------------------------------------------
    // iterate-component
    // -----------------------------------------------------------------------
    case "iterate-component": {
      const source = args?.source;
      if (typeof source !== "string" || source.trim().length === 0) {
        throw new Error("'source' argument is required");
      }
      const feedback = args?.feedback;
      if (typeof feedback !== "string" || feedback.trim().length === 0) {
        throw new Error("'feedback' argument is required");
      }

      // Try loading from prompts/iterate.md first
      const iteratePrompt = await loadPromptFile("iterate", { source, feedback });
      if (iteratePrompt) {
        return {
          messages: [{ role: "user", content: { type: "text", text: iteratePrompt } }],
        };
      }

      // Inline fallback
      const { languageRef } = await loadReferenceContext();

      return {
        messages: [
          {
            role: "user",
            content: {
              type: "text",
              text: `You are a GAME language expert. Modify the following .game source code to address the user's feedback.

## Current Source

\`\`\`game
${source}
\`\`\`

## Requested Changes

${feedback}

## Instructions

1. **Preserve working parts** — only change what is needed to address the feedback
2. **Maintain structure** — keep the cinematic block, layer names, and overall organization unless the feedback specifically asks to restructure
3. **Validate your changes** — ensure pipe chains follow correct stage ordering (domain ops -> SDF -> modifiers -> glow -> shading -> post-processing)
4. **Use existing primitives** — refer to the language reference below for valid syntax

## Common Refinement Patterns

- **Add glow:** append \`| glow(intensity)\` after an SDF stage
- **Change color:** add or modify \`| tint(color_name)\` or \`| shade(albedo: color)\`
- **Add animation:** use \`time\` in expressions (e.g., \`rotate(time * 0.5)\`) or modulation (\`param: base ~ signal\`)
- **Add post-processing:** append effects like \`| bloom(0.5, 1.2)\`, \`| vignette(0.3)\`, \`| grain(0.02)\`
- **Make it reactive:** add \`~ audio.bass\`, \`~ mouse.x\`, or other signal modulation to parameters
- **Add layers:** create additional \`layer name { fn: ... }\` blocks for composite effects
- **Add timeline:** use an \`arc { ... }\` block with named moments and transitions
- **Add interaction:** use a \`react { ... }\` block to map inputs to actions
- **Cross-layer feedback:** use \`resonate { ... }\` for emergent behavior between layers

## Output Format

Return ONLY the modified .game source code inside a single code block. No explanation before or after.

---

## Language Reference

${languageRef}`,
            },
          },
        ],
      };
    }

    // -----------------------------------------------------------------------
    // describe-component
    // -----------------------------------------------------------------------
    case "describe-component": {
      const source = args?.source;
      if (typeof source !== "string" || source.trim().length === 0) {
        throw new Error("'source' argument is required");
      }

      // Try loading from prompts/describe.md first
      const describePrompt = await loadPromptFile("describe", { source });
      if (describePrompt) {
        return {
          messages: [{ role: "user", content: { type: "text", text: describePrompt } }],
        };
      }

      // Inline fallback
      return {
        messages: [
          {
            role: "user",
            content: {
              type: "text",
              text: `You are a GAME language expert. Describe what the following .game visual effect does in plain English.

## Source Code

\`\`\`game
${source}
\`\`\`

## Instructions

Provide a clear, concise description covering:

1. **Overall effect** — what does this look like when rendered? What is the visual impression?
2. **Layers** — describe each layer: what shape/noise it uses, how it is colored, its role in the composition
3. **Parameters and modulation** — which parameters are defined, and which react to signals (audio, mouse, time)? What is the practical effect of each modulation?
4. **Timeline (arc)** — if present, describe the sequence of events: what happens when, and how do transitions unfold?
5. **Interaction (react)** — if present, describe what user inputs trigger
6. **Resonance (resonate)** — if present, explain the cross-layer feedback and what emergent behavior it creates
7. **Post-processing** — describe any screen-space effects applied (bloom, vignette, grain, etc.)
8. **Lens/camera** — describe the rendering mode and camera setup

Be concise but thorough. Use plain language a non-programmer could understand. Avoid repeating the source code verbatim.`,
            },
          },
        ],
      };
    }

    // -----------------------------------------------------------------------
    // generate-4da-component
    // -----------------------------------------------------------------------
    case "generate-4da-component": {
      const description = args?.description;
      if (typeof description !== "string" || description.trim().length === 0) {
        throw new Error("'description' argument is required");
      }

      const prompt4da = await loadPromptFile("generate-4da", { description });
      if (prompt4da) {
        return {
          messages: [{ role: "user", content: { type: "text", text: prompt4da } }],
        };
      }

      // Inline fallback — minimal version of the 4DA prompt
      return {
        messages: [
          {
            role: "user",
            content: {
              type: "text",
              text: `You are a GAME language expert generating components for the 4DA desktop app.

**Description:** ${description}

## 4DA Constraints
- Small UI component (16–64px canvas)
- Color: gold (#D4AF37) accent on dark background
- Use \`data.*\` signals for the main driving value (no audio)
- 1–3 layers max, subtle glow (1.0–2.5)
- Gentle animation — breathing/pulsing, not rapid

## Quick Reference
Pipe order: domain ops | SDF primitives | SDF modifiers | glow | color | post
Modulation: \`param: base ~ signal * scale\`
Colors: gold, white, obsidian, black, ivory, deep_blue
Signals: time, data.*, mouse.x/y

Return ONLY .game source in a fenced code block.`,
            },
          },
        ],
      };
    }

    // -----------------------------------------------------------------------
    // generate-achievement-visual
    // -----------------------------------------------------------------------
    case "generate-achievement-visual": {
      const description = args?.description;
      if (typeof description !== "string" || description.trim().length === 0) {
        throw new Error("'description' argument is required");
      }

      const achievementPrompt = await loadPromptFile("generate-achievement", { description });
      if (achievementPrompt) {
        return {
          messages: [{ role: "user", content: { type: "text", text: achievementPrompt } }],
        };
      }

      return {
        messages: [
          {
            role: "user",
            content: {
              type: "text",
              text: `You are a GAME language expert creating achievement visuals.

**Description:** ${description}

Use data.progress (0-1) for fill, data.unlocked (0 or 1) for completion.
Gold palette on dark. 2-4 layers. ring() + mask_arc() for progress arcs.
Return ONLY .game source in a fenced code block.`,
            },
          },
        ],
      };
    }

    // -----------------------------------------------------------------------
    // generate-game-indicator
    // -----------------------------------------------------------------------
    case "generate-game-indicator": {
      const description = args?.description;
      if (typeof description !== "string" || description.trim().length === 0) {
        throw new Error("'description' argument is required");
      }

      const indicatorPrompt = await loadPromptFile("generate-indicator", { description });
      if (indicatorPrompt) {
        return {
          messages: [{ role: "user", content: { type: "text", text: indicatorPrompt } }],
        };
      }

      return {
        messages: [
          {
            role: "user",
            content: {
              type: "text",
              text: `You are a GAME language expert creating status indicators.

**Description:** ${description}

Use data.value (0-1) as primary signal, optional data.status for state.
Gold/amber palette on dark. 1-3 layers. Gentle breathing animation.
Return ONLY .game source in a fenced code block.`,
            },
          },
        ],
      };
    }

    default:
      throw new Error(`Unknown prompt: ${name}`);
  }
});

// =============================================================================
// Server Lifecycle
// =============================================================================

async function main() {
  // Pre-flight check: warn if compiler is missing (non-fatal)
  if (!existsSync(COMPILER_PATH)) {
    console.error(
      `[GAME] Warning: Compiler not found at ${COMPILER_PATH}. ` +
      `Set GAME_COMPILER_PATH to the correct location.`
    );
  } else {
    console.error(`[GAME] Compiler: ${COMPILER_PATH}`);
  }
  console.error(`[GAME] Project root: ${GAME_ROOT}`);

  const transport = new StdioServerTransport();
  await server.connect(transport);

  // Graceful shutdown
  process.on("SIGINT", () => {
    console.error("[GAME] Shutting down");
    process.exit(0);
  });

  process.on("SIGTERM", () => {
    console.error("[GAME] Shutting down");
    process.exit(0);
  });

  console.error("GAME MCP Server v0.3.0 started — 6 tools, 3 resources, 6 prompts | stdio transport");
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
