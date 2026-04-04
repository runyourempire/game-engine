/**
 * Mask generation from animation recipe + depth map.
 * Creates per-region grayscale masks where white = animate, black = freeze.
 * Uses depth similarity + bounding boxes for reasonably accurate masks.
 */

import sharp from 'sharp';
import type { ImageRecipe, AnimationClass } from './types.js';

/** Mask classes that get their own mask texture */
const MASK_CLASSES: AnimationClass[] = ['water', 'sky', 'vegetation', 'fire', 'smoke'];

/**
 * Generate per-region mask PNGs from recipe and depth data.
 * Returns a Map of mask_name -> PNG buffer.
 */
export async function generateMasks(
  depthData: Float32Array,
  width: number,
  height: number,
  recipe: ImageRecipe
): Promise<Map<string, Buffer>> {
  console.log('[masks] Generating region masks...');
  const masks = new Map<string, Buffer>();

  // Group regions by animation class
  const regionsByClass = new Map<AnimationClass, ImageRecipe['regions']>();
  for (const region of recipe.regions) {
    if (!MASK_CLASSES.includes(region.animation_class)) continue;
    const existing = regionsByClass.get(region.animation_class) || [];
    existing.push(region);
    regionsByClass.set(region.animation_class, existing);
  }

  for (const [animClass, regions] of regionsByClass) {
    const maskData = new Float32Array(width * height);

    for (const region of regions) {
      const { bounds, depth_hint } = region;

      for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
          const nx = x / width;
          const ny = y / height;
          const i = y * width + x;

          // Check bounds with soft edges (feather)
          const feather = 0.05;
          const bx0 = bounds.x - feather;
          const bx1 = bounds.x + bounds.width + feather;
          const by0 = bounds.y - feather;
          const by1 = bounds.y + bounds.height + feather;

          if (nx < bx0 || nx > bx1 || ny < by0 || ny > by1) continue;

          // Distance to bounds edge (for feathering)
          const edgeX = Math.min(
            Math.max(0, nx - bounds.x) / feather,
            Math.max(0, bounds.x + bounds.width - nx) / feather,
            1.0
          );
          const edgeY = Math.min(
            Math.max(0, ny - bounds.y) / feather,
            Math.max(0, bounds.y + bounds.height - ny) / feather,
            1.0
          );
          const edgeFade = Math.min(edgeX, edgeY);

          // Depth similarity — pixels at similar depth to the region's hint
          // are more likely to be part of this region
          const depth = depthData[i];
          const depthDiff = Math.abs(depth - depth_hint);
          const depthWeight = Math.max(0, 1.0 - depthDiff * 2.5);

          // Combine: bounds proximity * depth similarity
          const value = edgeFade * depthWeight;
          maskData[i] = Math.max(maskData[i], value);
        }
      }
    }

    // Blur the mask for smooth edges
    for (let pass = 0; pass < 4; pass++) {
      blurMask(maskData, width, height);
    }

    // Convert to grayscale PNG
    const grayscale = Buffer.alloc(width * height);
    for (let i = 0; i < width * height; i++) {
      grayscale[i] = Math.round(Math.max(0, Math.min(255, maskData[i] * 255)));
    }

    const png = await sharp(grayscale, { raw: { width, height, channels: 1 } })
      .png()
      .toBuffer();

    const maskName = `mask_${animClass}`;
    masks.set(maskName, png);
    console.log(`  - ${maskName}: ${(png.length / 1024).toFixed(0)}KB`);
  }

  return masks;
}

function blurMask(data: Float32Array, width: number, height: number): void {
  const tmp = new Float32Array(data);
  for (let y = 1; y < height - 1; y++) {
    for (let x = 1; x < width - 1; x++) {
      let sum = 0;
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          sum += tmp[(y + dy) * width + (x + dx)];
        }
      }
      data[y * width + x] = sum / 9;
    }
  }
}
