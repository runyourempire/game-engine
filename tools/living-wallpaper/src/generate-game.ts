/**
 * .game source code generator — Living World Compositing approach.
 *
 * Instead of warping photo pixels, we compose procedural atmospheric effects
 * ON TOP of the static photo: caustics, mist, god rays, light variation.
 * The photo is the world — life comes from the atmosphere around it.
 */

import * as path from 'path';
import type { ImageRecipe } from './types.js';

interface GenerateOptions {
  imageName: string;
  outputDir: string;
  maskNames: string[];
  hasWater: boolean;
  hasSky: boolean;
  hasVegetation: boolean;
  baseName: string;
}

/**
 * Generate .game source code using Living World Compositing.
 */
export function generateGameSource(recipe: ImageRecipe, opts: GenerateOptions): string {
  const { imageName, baseName, hasWater, hasSky, hasVegetation } = opts;
  const componentName = `living-${baseName}`.replace(/[^a-z0-9-]/gi, '-').toLowerCase();

  // Derive scene parameters from recipe
  const windX = recipe.global_wind_direction[0] * 0.008;
  const windY = recipe.global_wind_direction[1] * 0.002;
  const sunPos = recipe.sun_position ?? estimateSunPosition(recipe);
  const colorTemp = recipe.color_temp ?? 'neutral';
  const timeOfDay = recipe.time_of_day ?? 'day';
  const intensity = recipe.ambient_motion_intensity ?? 0.3;

  // Color tints based on time of day / color temperature
  const mistTint = getMistTint(colorTemp, timeOfDay);
  const lightTint = getLightTint(colorTemp, timeOfDay);
  const rayTint = getRayTint(colorTemp, timeOfDay);

  const lines: string[] = [];
  lines.push(`// Living World — AI-generated atmospheric compositing`);
  lines.push(`// Scene: ${recipe.scene_type}`);
  lines.push(`// The photo is the world. Life comes from procedural atmosphere.`);
  lines.push('');
  lines.push(`cinematic "${componentName}" {`);

  // Texture declarations
  lines.push(`  texture "photo" from "${imageName}"`);
  lines.push(`  texture "depth" from "${baseName}-depth.png"`);
  if (hasWater) {
    lines.push(`  texture "flow" from "${baseName}-flow.png"`);
    lines.push(`  texture "mask_water" from "${baseName}-mask_water.png"`);
  }
  if (hasSky) {
    lines.push(`  texture "mask_sky" from "${baseName}-mask_sky.png"`);
  }

  lines.push('');

  // Config
  lines.push('  layer config {');
  lines.push(`    drift_x: ${windX.toFixed(5)}`);
  lines.push(`    drift_y: ${windY.toFixed(5)}`);
  lines.push('  }');
  lines.push('');

  // ═══ LAYER 1: THE WORLD — solid photo with depth parallax ═══
  lines.push('  // The world — solid, barely moves');
  lines.push('  layer world {');
  lines.push(`    parallax("photo", depth: "depth", strength: ${(0.01 * intensity).toFixed(4)}, orbit_speed: 0.08)`);
  lines.push('  }');

  // ═══ LAYER 2: WATER CAUSTICS (if water present) ═══
  if (hasWater) {
    const waterRegion = recipe.regions.find(r => r.animation_class === 'water');
    const flowSpeed = waterRegion?.flow_speed ?? 0.3;
    lines.push('');
    lines.push('  // Water caustics — voronoi shimmer composited as light');
    lines.push(`  layer caustics opacity: ${(0.06 * intensity).toFixed(3)} blend: screen {`);
    lines.push(`    translate(time * ${(flowSpeed * 0.04).toFixed(4)}, time * ${(flowSpeed * 0.02).toFixed(4)})`);
    lines.push('    | warp(scale: 6.0, octaves: 2, strength: 0.12)');
    lines.push('    | voronoi(12.0)');
    lines.push('    | glow(1.5)');
    lines.push('    | tint(0.5, 0.7, 1.0)');
    lines.push('    | mask("mask_water")');
    lines.push('  }');

    // Water sparkle — tiny bright points
    lines.push('');
    lines.push('  // Water sparkle — golden light catching the surface');
    lines.push(`  layer sparkle opacity: ${(0.03 * intensity).toFixed(3)} blend: add {`);
    lines.push(`    translate(time * 0.02, time * 0.01)`);
    lines.push('    | distort(scale: 15.0, speed: 0.8, strength: 0.3)');
    lines.push('    | voronoi(25.0)');
    lines.push('    | glow(4.0)');
    lines.push(`    | tint(${rayTint})`);
    lines.push('    | mask("mask_water")');
    lines.push('  }');
  }

  // ═══ LAYER 3: ATMOSPHERIC MIST (always) ═══
  lines.push('');
  lines.push('  // Atmospheric mist — drifting fog');
  lines.push(`  layer mist opacity: ${(0.05 * intensity).toFixed(3)} blend: screen {`);
  lines.push('    translate(time * drift_x, sin(time * 0.04) * 0.003)');
  lines.push('    | warp(scale: 1.2, octaves: 4, persistence: 0.65, strength: 0.1)');
  lines.push('    | fbm(scale: 1.8, octaves: 4, persistence: 0.55)');
  lines.push('    | glow(0.5)');
  lines.push(`    | tint(${mistTint})`);
  if (hasSky) {
    // Depth-masked: mist appears more in the background
    lines.push('    | mask("depth", invert: 1)');
  }
  lines.push('  }');

  // ═══ LAYER 4: LIGHT VARIATION (always) ═══
  lines.push('');
  lines.push('  // Light variation — cloud shadow modulation');
  lines.push(`  layer light_pulse opacity: ${(0.06 * intensity).toFixed(3)} blend: screen {`);
  lines.push('    translate(time * 0.004, time * 0.001)');
  lines.push('    | warp(scale: 0.6, octaves: 2, persistence: 0.5, strength: 0.06)');
  lines.push('    | fbm(scale: 0.5, octaves: 3, persistence: 0.5)');
  lines.push('    | glow(0.6)');
  lines.push(`    | tint(${lightTint})`);
  lines.push('  }');

  // ═══ LAYER 5: GOD RAYS (when sun visible or golden hour/dusk) ═══
  const showRays = sunPos !== null || timeOfDay === 'golden_hour' || timeOfDay === 'dawn' || timeOfDay === 'dusk';
  if (showRays) {
    const sx = sunPos ? sunPos[0].toFixed(2) : '0.0';
    const sy = sunPos ? sunPos[1].toFixed(2) : '0.35';
    lines.push('');
    lines.push('  // God rays — light shafts from brightest point');
    lines.push(`  layer godrays opacity: ${(0.04 * intensity).toFixed(3)} blend: add {`);
    lines.push(`    translate(${sx}, ${sy})`);
    lines.push('    | polar()');
    lines.push('    | distort(scale: 0.4, speed: 0.025, strength: 0.012)');
    lines.push('    | radial_fade(inner: 0.0, outer: 0.65)');
    lines.push('    | glow(3.0)');
    lines.push(`    | tint(${rayTint})`);
    lines.push('  }');
  }

  // ═══ LAYER 6: SKY DRIFT (if sky present) ═══
  if (hasSky) {
    lines.push('');
    lines.push('  // Sky — very subtle drift');
    lines.push('  layer sky opacity: 0.7 {');
    lines.push('    distort(scale: 0.6, speed: 0.03, strength: 0.005)');
    lines.push('    | sample("photo")');
    lines.push('    | mask("mask_sky")');
    lines.push('  }');
  }

  // ═══ POST-PROCESSING ═══
  lines.push('');
  lines.push('  // Cinematic post-processing');
  lines.push('  pass soften { blur(1.0) }');
  lines.push('  pass frame { vignette(0.15) }');
  lines.push('  pass film { film_grain(0.012) }');

  lines.push('}');
  lines.push('');

  return lines.join('\n');
}

/** Estimate sun position from region analysis when not explicitly provided */
function estimateSunPosition(recipe: ImageRecipe): [number, number] | null {
  const skyRegion = recipe.regions.find(r => r.animation_class === 'sky');
  if (!skyRegion) return null;
  // Center of the sky region, biased toward the top
  return [
    skyRegion.bounds.x + skyRegion.bounds.width * 0.5 - 0.5,
    (skyRegion.bounds.y + skyRegion.bounds.height * 0.3) * 2.0 - 1.0,
  ];
}

/** Mist color tint based on scene lighting */
function getMistTint(colorTemp: string, timeOfDay: string): string {
  if (timeOfDay === 'golden_hour' || timeOfDay === 'dawn') return '0.9, 0.85, 0.8';
  if (timeOfDay === 'dusk') return '0.75, 0.7, 0.85';
  if (timeOfDay === 'night') return '0.5, 0.55, 0.7';
  if (colorTemp === 'warm') return '0.85, 0.82, 0.78';
  if (colorTemp === 'cool') return '0.75, 0.82, 0.95';
  return '0.8, 0.85, 0.95';
}

/** Light variation tint */
function getLightTint(colorTemp: string, timeOfDay: string): string {
  if (timeOfDay === 'golden_hour' || timeOfDay === 'dawn') return '1.0, 0.88, 0.65';
  if (timeOfDay === 'dusk') return '0.9, 0.75, 0.85';
  if (timeOfDay === 'night') return '0.6, 0.65, 0.85';
  if (colorTemp === 'warm') return '1.0, 0.9, 0.7';
  return '0.95, 0.9, 0.8';
}

/** God ray / sparkle tint */
function getRayTint(colorTemp: string, timeOfDay: string): string {
  if (timeOfDay === 'golden_hour' || timeOfDay === 'dawn') return '1.0, 0.88, 0.55';
  if (timeOfDay === 'dusk') return '1.0, 0.75, 0.6';
  if (timeOfDay === 'night') return '0.7, 0.75, 1.0';
  if (colorTemp === 'warm') return '1.0, 0.92, 0.78';
  return '1.0, 0.95, 0.85';
}
