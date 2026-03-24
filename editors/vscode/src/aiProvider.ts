import * as vscode from "vscode";
import * as https from "https";

const SYSTEM_PROMPT = `You are a GAME DSL expert. GAME is a shader language that compiles to Web Components.

Generate a single GAME component based on the user's description. Return ONLY the .game code block, nothing else.

GAME syntax:
cinematic "component-name" {
  layer main {
    circle(0.3) | glow(4.0) | tint(0.8, 0.6, 0.2)
  }
}

Key syntax rules:
- Top level: cinematic "name" { ... } (kebab-case name in quotes)
- Visual layers: layer name { pipeline }
- Pipeline chains: stage1() | stage2() | stage3()
- Pipeline order: Position transforms -> SDF generators -> Bridges -> Color processors

Builtin categories:
- Position->Position: translate(x,y), rotate(speed), scale(s), warp(scale,octaves,persistence,lacunarity,strength), distort(scale,speed,strength), polar(), repeat(sx,sy), mirror(), radial(count)
- Position->SDF: circle(r), ring(r,w), star(points,r,inner), box(w,h), hex(r), fbm(scale,octaves,persistence,lacunarity), simplex(scale), voronoi(scale), line(x1,y1,x2,y2), grid(spacing,width)
- SDF->Color (bridges): glow(intensity), shade(r,g,b), palette(name), emissive(intensity)
- Color->Color: tint(r,g,b), bloom(threshold,strength), grain(amount), outline(width)
- SDF->SDF: mask_arc(angle), round(radius), shell(width)

Layer modifiers:
- Blend modes: layer name blend: add { ... } (additive blending)
- Memory: layer name memory: 0.9 { ... } (frame persistence, 0-1)

Available palettes (30): fire, ocean, neon, aurora, sunset, ice, ember, lava, magma, inferno, plasma, electric, cyber, matrix, forest, moss, earth, desert, blood, rose, candy, royal, deep_sea, coral, arctic, twilight, vapor, gold, silver, monochrome

Guidelines:
- kebab-case cinematic names (e.g. "fire-ring", "ocean-wave")
- Combine 3-6 pipeline stages for rich visuals
- palette() for complex color schemes, tint() for single colors
- warp() + fbm() for organic textures
- memory modifier for trails and persistence
- Use multiple layers with blend modes for complex effects
- 5-15 lines is ideal`;

const FIX_SYSTEM_PROMPT = `You are a GAME DSL expert. The user will provide GAME code that failed to compile, along with the error message. Fix the code and return ONLY the corrected .game code block, nothing else. Keep the visual intent. GAME uses cinematic "name" { layer name { pipeline... } } syntax with pipe-separated stages.`;

export interface AiGenerationResult {
  code: string;
  name: string;
  explanation: string;
}

interface ClaudeMessage {
  role: string;
  content: string;
}

export class AiProvider {
  private _apiKey: string | undefined;

  constructor() {
    this._apiKey = vscode.workspace
      .getConfiguration("game")
      .get<string>("ai.apiKey");
  }

  async generate(
    prompt: string,
    onChunk?: (text: string) => void
  ): Promise<AiGenerationResult> {
    const messages: ClaudeMessage[] = [{ role: "user", content: prompt }];
    const response = await this._callClaude(messages, SYSTEM_PROMPT, onChunk);
    return this._parseResponse(response);
  }

  async fixCompilationError(
    code: string,
    error: string,
    onChunk?: (text: string) => void
  ): Promise<AiGenerationResult> {
    const messages: ClaudeMessage[] = [
      {
        role: "user",
        content: `This GAME code failed to compile:\n\n\`\`\`game\n${code}\n\`\`\`\n\nError:\n${error}\n\nFix it.`,
      },
    ];
    const response = await this._callClaude(
      messages,
      FIX_SYSTEM_PROMPT,
      onChunk
    );
    return this._parseResponse(response);
  }

  private _parseResponse(response: string): AiGenerationResult {
    let code = response;
    const codeBlock = response.match(/```(?:game)?\s*\n([\s\S]*?)```/);
    if (codeBlock) {
      code = codeBlock[1].trim();
    }

    let name = "generated-component";
    const nameMatch = code.match(/cinematic\s+"([^"]+)"/);
    if (nameMatch) {
      name = nameMatch[1];
    }

    let explanation = "";
    if (codeBlock) {
      const before = response.substring(0, response.indexOf("```")).trim();
      const after = response
        .substring(response.lastIndexOf("```") + 3)
        .trim();
      explanation = [before, after].filter(Boolean).join("\n");
    }

    return { code, name, explanation };
  }

  private async _ensureApiKey(): Promise<string> {
    if (this._apiKey) return this._apiKey;

    const key = await vscode.window.showInputBox({
      prompt: "Enter your Anthropic API key for GAME AI generation",
      placeHolder: "sk-ant-...",
      password: true,
      ignoreFocusOut: true,
    });

    if (!key) {
      throw new Error(
        'API key required. Set it in Settings > GAME > AI: Api Key'
      );
    }

    this._apiKey = key;
    await vscode.workspace
      .getConfiguration("game")
      .update("ai.apiKey", key, vscode.ConfigurationTarget.Global);
    return key;
  }

  private async _callClaude(
    messages: ClaudeMessage[],
    system: string,
    onChunk?: (text: string) => void
  ): Promise<string> {
    const apiKey = await this._ensureApiKey();
    const config = vscode.workspace.getConfiguration("game");
    const model = config.get<string>("ai.model", "claude-sonnet-4-20250514");

    const body = JSON.stringify({
      model,
      max_tokens: 4096,
      system,
      stream: true,
      messages,
    });

    return new Promise<string>((resolve, reject) => {
      const req = https.request(
        {
          hostname: "api.anthropic.com",
          path: "/v1/messages",
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "x-api-key": apiKey,
            "anthropic-version": "2023-06-01",
          },
        },
        (res) => {
          let fullText = "";
          let buffer = "";

          if (res.statusCode === 401) {
            this._apiKey = undefined;
            reject(new Error("Invalid API key. Check your Anthropic API key."));
            return;
          }
          if (res.statusCode === 429) {
            reject(new Error("Rate limited. Wait a moment and try again."));
            return;
          }
          if (res.statusCode && (res.statusCode < 200 || res.statusCode >= 300)) {
            let errBody = "";
            res.on("data", (c: Buffer) => { errBody += c.toString(); });
            res.on("end", () => {
              let msg = `API error ${res.statusCode}`;
              try {
                const p = JSON.parse(errBody);
                if (p.error?.message) msg = p.error.message;
              } catch { /* use default */ }
              reject(new Error(msg));
            });
            return;
          }

          res.on("data", (chunk: Buffer) => {
            buffer += chunk.toString();
            const lines = buffer.split("\n");
            buffer = lines.pop() || "";
            for (const line of lines) {
              if (!line.startsWith("data: ")) continue;
              const data = line.slice(6).trim();
              if (data === "[DONE]") continue;
              try {
                const evt = JSON.parse(data);
                if (
                  evt.type === "content_block_delta" &&
                  evt.delta?.type === "text_delta"
                ) {
                  fullText += evt.delta.text;
                  onChunk?.(evt.delta.text);
                }
                if (evt.type === "error") {
                  reject(new Error(evt.error?.message || "Streaming error"));
                }
              } catch { /* skip unparseable SSE lines */ }
            }
          });

          res.on("end", () => {
            if (buffer.startsWith("data: ")) {
              const data = buffer.slice(6).trim();
              if (data && data !== "[DONE]") {
                try {
                  const evt = JSON.parse(data);
                  if (evt.type === "content_block_delta" && evt.delta?.type === "text_delta") {
                    fullText += evt.delta.text;
                    onChunk?.(evt.delta.text);
                  }
                } catch { /* ignore */ }
              }
            }
            if (!fullText) {
              reject(new Error("No response from API"));
              return;
            }
            resolve(fullText);
          });

          res.on("error", (err) => reject(new Error(`Network: ${err.message}`)));
        }
      );

      req.on("error", (err) => {
        if (err.message.includes("ENOTFOUND") || err.message.includes("EAI_AGAIN")) {
          reject(new Error("Cannot reach api.anthropic.com. Check internet."));
        } else {
          reject(new Error(`Network: ${err.message}`));
        }
      });

      req.write(body);
      req.end();
    });
  }
}
