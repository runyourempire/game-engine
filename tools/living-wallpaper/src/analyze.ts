/**
 * Claude Vision API analysis — identifies regions and animation parameters.
 * Returns a structured recipe for generating flowmaps, masks, and .game code.
 */

import Anthropic from '@anthropic-ai/sdk';
import * as fs from 'fs';
import * as path from 'path';
import type { ImageRecipe } from './types.js';

const ANALYSIS_PROMPT = `Analyze this image for a living wallpaper system that composites procedural atmospheric effects on top of a static photo. I need to understand the scene structure, lighting, and regions.

Return ONLY valid JSON (no markdown, no explanation) matching this exact schema:

{
  "scene_type": "brief description of the scene",
  "sun_position": [0.0, 0.3],
  "color_temp": "warm",
  "time_of_day": "golden_hour",
  "has_water": true,
  "has_sky": true,
  "global_wind_direction": [1.0, 0.0],
  "ambient_motion_intensity": 0.3,
  "regions": [
    {
      "name": "descriptive_name_snake_case",
      "bounds": { "x": 0.0, "y": 0.0, "width": 1.0, "height": 0.5 },
      "depth_hint": 0.5,
      "animation_class": "water",
      "flow_direction": [0.0, 1.0],
      "flow_speed": 0.4,
      "warp_amount": 0.3,
      "distort_frequency": 0.5
    }
  ]
}

Top-level fields:
- sun_position: [x, y] normalized position of brightest light source in GAME coordinates (x: -1 to 1, y: -1 to 1, 0,0 is center). Null if no visible light source.
- color_temp: "warm" (sunset, golden), "cool" (overcast, blue), "neutral"
- time_of_day: "dawn", "day", "golden_hour", "dusk", "night"
- has_water: true if visible water surface (river, lake, ocean)
- has_sky: true if sky is visible
- ambient_motion_intensity: 0.1 (calm) to 0.5 (dramatic). Most scenes: 0.2-0.4.

Region rules:
- bounds: normalized 0-1 coordinates, (0,0) is top-left
- depth_hint: 0.0=far background, 1.0=near foreground
- animation_class: one of "static", "water", "sky", "vegetation", "fire", "smoke"
- flow_direction: [dx, dy] unit vector. Water flows downhill, sky/clouds drift with wind
- flow_speed: 0.0=still, 1.0=fast. Water: 0.3-0.6, clouds: 0.1-0.3

Identify 3-8 regions. Every pixel should be covered.`;

/**
 * Analyze an image using Claude Vision API.
 * Requires ANTHROPIC_API_KEY environment variable.
 */
export async function analyzeImage(imagePath: string): Promise<ImageRecipe> {
  if (imagePath === '__skip__') {
    return fallbackAnalysis();
  }

  const apiKey = process.env.ANTHROPIC_API_KEY;
  if (!apiKey) {
    console.log('[analyze] No ANTHROPIC_API_KEY — using fallback analysis');
    return fallbackAnalysis();
  }

  console.log('[analyze] Sending image to Claude Vision...');
  const client = new Anthropic({ apiKey });

  const imageBuffer = fs.readFileSync(imagePath);
  const ext = path.extname(imagePath).toLowerCase();
  const mediaType = ext === '.png' ? 'image/png'
    : ext === '.webp' ? 'image/webp'
    : 'image/jpeg';

  const response = await client.messages.create({
    model: 'claude-sonnet-4-6-20250514',
    max_tokens: 2048,
    messages: [{
      role: 'user',
      content: [
        {
          type: 'image',
          source: {
            type: 'base64',
            media_type: mediaType,
            data: imageBuffer.toString('base64'),
          },
        },
        { type: 'text', text: ANALYSIS_PROMPT },
      ],
    }],
  });

  const text = response.content[0].type === 'text' ? response.content[0].text : '';

  // Extract JSON from response (handle potential markdown wrapping)
  const jsonMatch = text.match(/\{[\s\S]*\}/);
  if (!jsonMatch) {
    console.warn('[analyze] Could not parse Claude response, using fallback');
    return fallbackAnalysis();
  }

  const recipe = JSON.parse(jsonMatch[0]) as ImageRecipe;
  console.log(`[analyze] Identified ${recipe.regions.length} regions in "${recipe.scene_type}"`);
  for (const r of recipe.regions) {
    console.log(`  - ${r.name}: ${r.animation_class} (speed=${r.flow_speed}, warp=${r.warp_amount})`);
  }

  return recipe;
}

/** Fallback when no API key — produces a basic 3-region recipe */
function fallbackAnalysis(): ImageRecipe {
  console.log('[analyze] Using default landscape recipe (sky + water + ground)');
  return {
    scene_type: 'generic landscape',
    sun_position: [0.0, 0.35],
    color_temp: 'warm',
    time_of_day: 'golden_hour',
    has_water: true,
    has_sky: true,
    regions: [
      {
        name: 'sky',
        bounds: { x: 0, y: 0, width: 1, height: 0.4 },
        depth_hint: 0.0,
        animation_class: 'sky',
        flow_direction: [1.0, 0.0],
        flow_speed: 0.15,
        warp_amount: 0.15,
        distort_frequency: 0.3,
      },
      {
        name: 'midground',
        bounds: { x: 0, y: 0.3, width: 1, height: 0.4 },
        depth_hint: 0.4,
        animation_class: 'vegetation',
        flow_direction: [0, 0],
        flow_speed: 0,
        warp_amount: 0.5,
        distort_frequency: 0.4,
      },
      {
        name: 'foreground',
        bounds: { x: 0, y: 0.6, width: 1, height: 0.4 },
        depth_hint: 0.8,
        animation_class: 'water',
        flow_direction: [0.3, 0.95],
        flow_speed: 0.35,
        warp_amount: 0.3,
        distort_frequency: 0.5,
      },
    ],
    global_wind_direction: [1.0, 0.0],
    ambient_motion_intensity: 0.3,
  };
}
