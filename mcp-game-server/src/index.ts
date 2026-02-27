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
 *   - list_primitives: List all available GAME language primitives
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

const PRIMITIVES_DATA = {
  sdf_primitives: {
    description: "Signed distance field primitives. Produce sdf_result. Use in fn: pipe chains.",
    entries: [
      { name: "circle", syntax: "circle(r)", params: "r: radius (default 0.5)" },
      { name: "sphere", syntax: "sphere(r)", params: "r: radius (default 0.5, uses 3D SDF projected to 2D)" },
      { name: "ring", syntax: "ring(radius, thickness)", params: "radius: ring center distance (default 0.3), thickness: wall width (default 0.04)" },
      { name: "box", syntax: "box(w, h)", params: "w: width (default 0.5), h: height (default 0.5)" },
      { name: "torus", syntax: "torus(R, r)", params: "R: major radius (default 0.3), r: minor radius (default 0.05)" },
      { name: "line", syntax: "line(x1, y1, x2, y2, thickness)", params: "Segment endpoints + thickness (default 0.02)" },
      { name: "polygon", syntax: "polygon(sides, radius)", params: "sides: number of sides (default 6), radius: size (default 0.3)" },
      { name: "star", syntax: "star(points, outer, inner)", params: "points: number (default 5), outer: radius (default 0.4), inner: radius (default 0.2)" },
    ],
  },
  domain_operations: {
    description: "Transform position before SDF evaluation. Place before shapes in pipe chain.",
    entries: [
      { name: "translate", syntax: "translate(x, y)", params: "x/y: offset (default 0.0)" },
      { name: "rotate", syntax: "rotate(angle)", params: "angle: radians (default 0.0). Use time expressions for animation." },
      { name: "scale", syntax: "scale(s)", params: "s: uniform scale factor (default 1.0). SDF result auto-corrected." },
      { name: "repeat", syntax: "repeat(spacing)", params: "spacing: grid cell size (default 1.0). Infinite tiling." },
      { name: "mirror", syntax: "mirror(axis)", params: "axis: 'x', 'y', or 'xy' (default 'xy'). Reflect across axis." },
      { name: "twist", syntax: "twist(amount)", params: "amount: twist strength (default 1.0). Twists along Y axis." },
    ],
  },
  sdf_modifiers: {
    description: "Modify an existing SDF. Place after a shape in the pipe chain.",
    entries: [
      { name: "mask_arc", syntax: "mask_arc(angle)", params: "angle: arc extent in radians (0..6.283). Clips SDF to arc sector." },
      { name: "displace", syntax: "displace(strength)", params: "strength: noise displacement amount (default 0.1). Uses simplex noise." },
      { name: "round", syntax: "round(r)", params: "r: rounding radius (default 0.05). Rounds sharp edges." },
      { name: "onion", syntax: "onion(thickness)", params: "thickness: shell wall width (default 0.02). Creates concentric shells." },
      { name: "threshold", syntax: "threshold(value)", params: "Binary step threshold on SDF result. value: cutoff (default 0.5). Produces hard edges." },
    ],
  },
  noise_functions: {
    description: "Procedural noise as SDF source. Produces sdf_result.",
    entries: [
      { name: "fbm", syntax: "fbm(pos, octaves:N, persistence:P, lacunarity:L)", params: "Fractal Brownian Motion. pos: coordinate (default p), octaves: 1-8 (default 6), persistence (default 0.5), lacunarity (default 2.0)" },
      { name: "simplex", syntax: "simplex(frequency)", params: "frequency: spatial frequency (default 1.0). Smooth organic noise." },
      { name: "voronoi", syntax: "voronoi(frequency)", params: "frequency: cell density (default 1.0). Cellular/crystal pattern." },
      { name: "curl_noise", syntax: "curl_noise(pos, frequency, amplitude)", params: "Curl of 2D simplex noise. Creates flowing, divergence-free patterns. pos: coordinate (default p), frequency (default 1.0), amplitude (default 1.0)" },
      { name: "concentric_waves", syntax: "concentric_waves(origins, decay, speed)", params: "Expanding concentric wave pattern from center. origins: number of wave centers (default 1), decay: falloff (default 1.0), speed: expansion rate (default 1.0)" },
    ],
  },
  glow: {
    description: "Convert SDF distance to glow intensity. Bridges SDF state to Glow state.",
    entries: [
      { name: "glow", syntax: "glow(intensity)", params: "intensity: glow strength (default 2.0). Exponential distance falloff." },
    ],
  },
  shading_and_color: {
    description: "Color stages. Can follow SDF, glow, or other color stages.",
    entries: [
      { name: "shade", syntax: "shade(albedo: color, emissive: color)", params: "Named params. albedo: base color vec3f (default 0.8), emissive: glow color vec3f (default 0.0)" },
      { name: "emissive", syntax: "emissive()", params: "Quick self-illuminating gold glow." },
      { name: "colormap", syntax: "colormap()", params: "Maps SDF distance to color gradient (dark blue to gold)." },
      { name: "spectrum", syntax: "spectrum(bass, mid, treble)", params: "Audio-reactive concentric rings. Each param maps to frequency band intensity." },
      { name: "tint", syntax: "tint(color)", params: "Multiplies current glow/color by a color. Accepts named colors or vec3f." },
      { name: "gradient", syntax: "gradient(color_a, color_b, direction)", params: "Spatial gradient. direction: 'x', 'y', or 'radial' (default 'y')." },
      { name: "particles", syntax: "particles(count, size, color, trail)", params: "Hash-based pseudo-particle field. count: number of particles (default 1000), size: particle radius (default 1.5), color: named color or vec3f (default white), trail: motion blur length (default 0.0)" },
    ],
  },
  post_processing: {
    description: "Screen-space effects. Apply after color stages in the pipe chain.",
    entries: [
      { name: "bloom", syntax: "bloom(threshold, intensity)", params: "threshold: luminance cutoff (default 0.6), intensity: bloom strength (default 1.5)" },
      { name: "chromatic", syntax: "chromatic(strength)", params: "strength: RGB separation amount (default 0.5)" },
      { name: "vignette", syntax: "vignette(strength)", params: "strength: edge darkening (default 0.3)" },
      { name: "grain", syntax: "grain(amount)", params: "amount: noise intensity (default 0.02)" },
      { name: "fog", syntax: "fog(density, color)", params: "density: fog thickness (default 1.0), color: fog color vec3f (default black)" },
      { name: "glitch", syntax: "glitch(intensity)", params: "intensity: artifact strength (default 0.5). Digital distortion effect." },
      { name: "scanlines", syntax: "scanlines(count, intensity)", params: "count: line frequency (default 100), intensity: darkening (default 0.3)" },
      { name: "tonemap", syntax: "tonemap(exposure)", params: "exposure: brightness (default 1.0). Reinhard-style HDR compression." },
      { name: "invert", syntax: "invert()", params: "Inverts all colors (1.0 - rgb)." },
      { name: "saturate_color", syntax: "saturate_color(amount)", params: "amount: saturation multiplier (default 1.5). >1 increases, <1 decreases." },
      { name: "iridescent", syntax: "iridescent(strength)", params: "Thin-film interference / rainbow color shift effect. strength: effect intensity (default 0.3). Best after shading stages." },
    ],
  },
  signals: {
    description: "Real-time signals for parameter modulation via the ~ operator. Use: param: base ~ signal * scale",
    entries: [
      { name: "audio.bass", syntax: "~ audio.bass", params: "Low frequency FFT band (0..1)" },
      { name: "audio.mid", syntax: "~ audio.mid", params: "Mid frequency FFT band (0..1)" },
      { name: "audio.treble", syntax: "~ audio.treble", params: "High frequency FFT band (0..1)" },
      { name: "audio.energy", syntax: "~ audio.energy", params: "Overall audio energy (0..1)" },
      { name: "audio.beat", syntax: "~ audio.beat", params: "Beat detection impulse (0 or 1)" },
      { name: "mouse.x", syntax: "~ mouse.x", params: "Normalized cursor X (0..1)" },
      { name: "mouse.y", syntax: "~ mouse.y", params: "Normalized cursor Y (0..1)" },
      { name: "data.*", syntax: "~ data.fieldname", params: "Data signal bound to Web Component property. E.g., data.value, data.progress" },
    ],
  },
  named_colors: {
    description: "Built-in color names for use with tint(), shade(), gradient().",
    entries: [
      { name: "black", syntax: "tint(black)", params: "vec3f(0.0, 0.0, 0.0)" },
      { name: "white", syntax: "tint(white)", params: "vec3f(1.0, 1.0, 1.0)" },
      { name: "red", syntax: "tint(red)", params: "vec3f(1.0, 0.0, 0.0)" },
      { name: "green", syntax: "tint(green)", params: "vec3f(0.0, 1.0, 0.0)" },
      { name: "blue", syntax: "tint(blue)", params: "vec3f(0.0, 0.0, 1.0)" },
      { name: "cyan", syntax: "tint(cyan)", params: "vec3f(0.0, 1.0, 1.0)" },
      { name: "orange", syntax: "tint(orange)", params: "vec3f(1.0, 0.5, 0.0)" },
      { name: "gold", syntax: "tint(gold)", params: "vec3f(0.831, 0.686, 0.216)" },
      { name: "ember", syntax: "tint(ember)", params: "vec3f(0.8, 0.2, 0.05)" },
      { name: "frost", syntax: "tint(frost)", params: "vec3f(0.85, 0.92, 1.0)" },
      { name: "ivory", syntax: "tint(ivory)", params: "vec3f(1.0, 0.97, 0.92)" },
      { name: "midnight", syntax: "tint(midnight)", params: "vec3f(0.0, 0.0, 0.1)" },
      { name: "obsidian", syntax: "tint(obsidian)", params: "vec3f(0.04, 0.04, 0.06)" },
      { name: "deep_blue", syntax: "tint(deep_blue)", params: "vec3f(0.0, 0.02, 0.15)" },
    ],
  },
  language_features: {
    description: "Core language constructs beyond pipe chains.",
    entries: [
      { name: "define", syntax: "define name(params) { stages }", params: "Reusable macro. Expands inline at compile time. E.g., define glow_ring(r, t) { ring(r, t) | glow(2.0) }" },
      { name: "layer", syntax: "layer name { fn: chain }", params: "Named visual layer. Multiple layers composite additively. Params use ~ for modulation." },
      { name: "arc", syntax: "arc { time label { transitions } }", params: "Timeline system. Moments at timestamps with param transitions. E.g., 0:03 \"expand\" { radius -> 0.5 ease(expo_out) over 2s }" },
      { name: "lens", syntax: "lens { mode: raymarch ... }", params: "Camera/render mode. Default: flat (2D). Options: raymarch (with orbit camera)." },
      { name: "math constants", syntax: "pi, tau, e, phi", params: "pi=3.14159, tau=6.28318, e=2.71828, phi=1.61803 (golden ratio)" },
      { name: "import", syntax: "import \"path\" expose name1, name2", params: "Import defines from external .game files. Also supports `expose ALL` to import everything." },
      { name: "react", syntax: "react { signal -> action }", params: "Map user inputs to actions. E.g., `mouse.click -> particles.burst(...)`, `key(\"space\") -> arc.pause_toggle`" },
      { name: "resonate", syntax: "resonate { a.param ~ b.param * factor, damping: 0.95 }", params: "Cross-layer parameter feedback. Bidirectional coupling between layers. Damping prevents runaway feedback." },
    ],
  },
  easing_functions: {
    description: "Easing functions for arc timeline transitions. Use: ease(name)",
    entries: [
      { name: "linear", syntax: "ease(linear)", params: "Constant speed (default)" },
      { name: "smooth", syntax: "ease(smooth)", params: "Smooth ease in/out (smoothstep)" },
      { name: "expo_in", syntax: "ease(expo_in)", params: "Slow start, fast end" },
      { name: "expo_out", syntax: "ease(expo_out)", params: "Fast start, slow end" },
      { name: "cubic_in_out", syntax: "ease(cubic_in_out)", params: "Cubic ease in/out" },
      { name: "elastic", syntax: "ease(elastic)", params: "Springy overshoot" },
      { name: "bounce", syntax: "ease(bounce)", params: "Ball-drop bounce at end" },
    ],
  },
};

// =============================================================================
// Server Setup
// =============================================================================

const server = new Server(
  {
    name: "game-server",
    version: "0.2.0",
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
        name: "list_primitives",
        description:
          "Return all available GAME language primitives grouped by category: SDF primitives, boolean ops, " +
          "domain ops, noise functions, shading, post-processing, camera modes, and signals. " +
          "Each entry includes syntax and parameter descriptions.",
        inputSchema: {
          type: "object" as const,
          properties: {},
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
      // list_primitives
      // -----------------------------------------------------------------------
      case "list_primitives": {
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(PRIMITIVES_DATA, null, 2),
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
    ],
  };
});

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

      const { languageRef, primitivesRef, examplesRef } = await loadReferenceContext();

      return {
        messages: [
          {
            role: "user",
            content: {
              type: "text",
              text: `You are a GAME language expert. Generate a .game file that creates the following visual effect:

**Description:** ${description}

Use the language reference, primitives, and examples below to produce valid .game source code. The output should be a complete cinematic block that compiles with the GAME compiler.

## Guidelines

1. Start with a \`cinematic\` block with an appropriate title
2. Define layers with generative functions using the pipe operator \`|\`
3. Use modulation (\`~\`) to make parameters react to signals (time, audio, mouse)
4. Add a lens block with appropriate mode (flat for 2D, raymarch for 3D)
5. Include post-processing effects for visual polish
6. Use descriptive names for layers and parameters
7. Keep it focused — a good effect is simple but expressive
8. Use \`define\` for reusable patterns when you repeat similar pipe chains
9. Use \`resonate\` for cross-layer feedback when multiple layers should interact
10. Use \`react\` to map user inputs (mouse, keyboard) to actions

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

  console.error("GAME MCP Server v0.2.0 started — 3 tools, 3 resources, 3 prompts | stdio transport");
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
