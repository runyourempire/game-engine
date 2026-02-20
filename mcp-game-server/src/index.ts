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
    description: "Signed distance field primitives. Negative inside, positive outside, zero at surface.",
    entries: [
      { name: "sphere", syntax: "sphere(r)", params: "r: radius" },
      { name: "box", syntax: "box(w, h, d) or box(size)", params: "w/h/d: dimensions, or size for cube" },
      { name: "torus", syntax: "torus(R, r)", params: "R: major radius, r: minor radius" },
      { name: "cylinder", syntax: "cylinder(r, h)", params: "r: radius, h: height" },
      { name: "plane", syntax: "plane(normal, offset)", params: "normal: direction vector, offset: distance from origin" },
      { name: "capsule", syntax: "capsule(a, b, r)", params: "a/b: endpoints, r: radius" },
      { name: "cone", syntax: "cone(angle, h)", params: "angle: half-angle, h: height" },
      { name: "line", syntax: "line(a, b, r)", params: "a/b: segment endpoints, r: thickness" },
      { name: "circle", syntax: "circle(r)", params: "r: radius (2D mode)" },
    ],
  },
  boolean_operations: {
    description: "Combine SDFs to build complex shapes.",
    entries: [
      { name: "union", syntax: "union(a, b)", params: "Merge two shapes" },
      { name: "smooth_union", syntax: "smooth_union(a, b, k)", params: "k: blend smoothness" },
      { name: "intersect", syntax: "intersect(a, b)", params: "Only overlapping regions" },
      { name: "smooth_intersect", syntax: "smooth_intersect(a, b, k)", params: "k: blend smoothness" },
      { name: "subtract", syntax: "subtract(a, b)", params: "Cut b from a" },
      { name: "smooth_subtract", syntax: "smooth_subtract(a, b, k)", params: "k: blend smoothness" },
    ],
  },
  domain_operations: {
    description: "Transform input position before evaluating the SDF.",
    entries: [
      { name: "translate", syntax: "translate(x, y, z)", params: "Move in space" },
      { name: "rotate", syntax: "rotate(ax, ay, az)", params: "Euler rotation (radians)" },
      { name: "rotate_axis", syntax: "rotate_axis(axis, angle)", params: "Rotate around arbitrary axis" },
      { name: "scale", syntax: "scale(s) or scale(x, y, z)", params: "Uniform or non-uniform scale" },
      { name: "repeat", syntax: "repeat(spacing) or repeat(x, y, z)", params: "Infinite spatial repetition" },
      { name: "repeat_n", syntax: "repeat_n(spacing, count)", params: "Finite repetition" },
      { name: "mirror", syntax: "mirror(axis)", params: "Mirror across x, y, z, xy, etc." },
      { name: "twist", syntax: "twist(amount)", params: "Twist around Y axis" },
      { name: "bend", syntax: "bend(amount)", params: "Bend around Y axis" },
      { name: "elongate", syntax: "elongate(x, y, z)", params: "Stretch the SDF" },
      { name: "displace", syntax: "displace(noise_fn)", params: "Noise-based surface displacement" },
      { name: "round", syntax: "round(r)", params: "Round edges by r" },
      { name: "shell", syntax: "shell(thickness)", params: "Hollow out with wall thickness" },
      { name: "onion", syntax: "onion(thickness)", params: "Concentric shell layers" },
      { name: "symmetry", syntax: "symmetry(axes)", params: "Force symmetry across axes" },
    ],
  },
  noise_functions: {
    description: "Procedural noise functions. Accept position, return float.",
    entries: [
      { name: "simplex", syntax: "simplex(p)", params: "Smooth, organic noise" },
      { name: "perlin", syntax: "perlin(p)", params: "Classic gradient noise" },
      { name: "value_noise", syntax: "value_noise(p)", params: "Simple interpolated random" },
      { name: "worley", syntax: "worley(p, jitter)", params: "Cellular, crystal-like" },
      { name: "voronoi", syntax: "voronoi(p, jitter)", params: "Cell boundary distance" },
      { name: "fbm", syntax: "fbm(p, octaves, lacunarity, persistence)", params: "Fractal Brownian Motion" },
      { name: "turbulence", syntax: "turbulence(p, octaves)", params: "Absolute-value FBM (sharp)" },
      { name: "ridged", syntax: "ridged(p, octaves)", params: "Inverted turbulence (ridges)" },
      { name: "curl_noise", syntax: "curl_noise(p, frequency)", params: "Divergence-free 3D noise" },
      { name: "warp", syntax: "warp(p, noise_fn, strength)", params: "Domain warping" },
    ],
  },
  shading: {
    description: "Transform SDF distance/position into color.",
    entries: [
      { name: "shade", syntax: "shade(albedo, roughness, metallic)", params: "PBR shading" },
      { name: "emissive", syntax: "emissive(color, intensity)", params: "Self-illuminating glow" },
      { name: "fresnel", syntax: "fresnel(color, power)", params: "Edge glow effect" },
      { name: "iridescent", syntax: "iridescent(strength)", params: "Angle-dependent color shift" },
      { name: "toon", syntax: "toon(colors, steps)", params: "Cel-shaded look" },
      { name: "matcap", syntax: "matcap(texture_fn)", params: "Material capture via function" },
      { name: "colormap", syntax: "colormap(palette)", params: "Map scalar to color gradient" },
    ],
  },
  post_processing: {
    description: "Screen-space effects applied after rendering.",
    entries: [
      { name: "bloom", syntax: "bloom(intensity, threshold?)", params: "Glow on bright areas" },
      { name: "chromatic", syntax: "chromatic(strength)", params: "RGB channel separation" },
      { name: "vignette", syntax: "vignette(strength)", params: "Darkened edges" },
      { name: "grain", syntax: "grain(intensity)", params: "Film grain noise" },
      { name: "fog", syntax: "fog(density, color)", params: "Distance-based atmospheric fog" },
      { name: "distort", syntax: "distort(noise_fn, strength)", params: "Screen-space distortion" },
      { name: "glitch", syntax: "glitch(intensity, speed)", params: "Digital artifact effect" },
      { name: "scanlines", syntax: "scanlines(count, intensity)", params: "CRT monitor effect" },
      { name: "sharpen", syntax: "sharpen(strength)", params: "Edge enhancement" },
      { name: "blur", syntax: "blur(radius)", params: "Gaussian blur" },
      { name: "dof", syntax: "dof(focus_dist, aperture)", params: "Depth of field" },
      { name: "grade", syntax: "grade(lift, gamma, gain)", params: "Film-style color correction" },
      { name: "tonemap", syntax: "tonemap(method)", params: "HDR to SDR: aces, reinhard, filmic" },
    ],
  },
  camera: {
    description: "Camera modes for raymarch and volume lens modes.",
    entries: [
      { name: "orbit", syntax: "orbit(radius, height, speed)", params: "Circle around origin" },
      { name: "static", syntax: "static(position, target)", params: "Fixed position and look-at" },
      { name: "dolly", syntax: "dolly(from, to, ease)", params: "Linear movement" },
      { name: "crane", syntax: "crane(height_from, height_to, radius)", params: "Vertical arc" },
      { name: "handheld", syntax: "handheld(position, shake)", params: "Noise-based camera shake" },
      { name: "track", syntax: "track(path, speed)", params: "Follow a defined path" },
      { name: "fps", syntax: "fps(position, look_dir)", params: "Mouse-controlled first person" },
    ],
  },
  signals: {
    description: "Real-time signals for parameter modulation via the ~ operator.",
    entries: [
      { name: "audio.bass", syntax: "~ audio.bass", params: "Low frequency FFT band" },
      { name: "audio.mid", syntax: "~ audio.mid", params: "Mid frequency FFT band" },
      { name: "audio.treble", syntax: "~ audio.treble", params: "High frequency FFT band" },
      { name: "audio.energy", syntax: "~ audio.energy", params: "Overall audio energy" },
      { name: "audio.beat", syntax: "~ audio.beat", params: "Beat detection impulse" },
      { name: "mouse.x", syntax: "~ mouse.x", params: "Normalized cursor X (0..1)" },
      { name: "mouse.y", syntax: "~ mouse.y", params: "Normalized cursor Y (0..1)" },
      { name: "mouse.velocity", syntax: "~ mouse.velocity", params: "Cursor movement speed" },
      { name: "mouse.click", syntax: "~ mouse.click", params: "Click impulse (decays ~200ms)" },
      { name: "time", syntax: "~ time", params: "Elapsed time in seconds" },
      { name: "beat", syntax: "~ beat", params: "BPM-synchronized pulse" },
      { name: "random", syntax: "~ random", params: "Per-frame random value" },
    ],
  },
};

// =============================================================================
// Server Setup
// =============================================================================

const server = new Server(
  {
    name: "game-server",
    version: "0.1.0",
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
    ],
  };
});

server.setRequestHandler(GetPromptRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  if (name !== "generate-component") {
    throw new Error(`Unknown prompt: ${name}`);
  }

  const description = args?.description;
  if (typeof description !== "string" || description.trim().length === 0) {
    throw new Error("'description' argument is required");
  }

  // Load language reference and primitives for context
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

  console.error("GAME MCP Server v0.1.0 started — 3 tools, 3 resources, 1 prompt | stdio transport");
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
