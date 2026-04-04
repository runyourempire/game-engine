/**
 * Flow map generation from depth map + animation recipe.
 * Produces an RG texture where (0.5, 0.5) = no motion,
 * and the offset from 0.5 encodes flow direction and magnitude.
 */

import sharp from 'sharp';
import type { ImageRecipe, AnimationClass } from './types.js';

/**
 * Generate a flow map from depth data and animation recipe.
 * Returns a PNG buffer (RGB, R=flow_x, G=flow_y, B=0).
 */
export async function generateFlowMap(
  depthData: Float32Array,
  width: number,
  height: number,
  recipe: ImageRecipe
): Promise<Buffer> {
  console.log('[flowmap] Computing Sobel gradients...');

  // Step 1: Compute depth gradients (Sobel)
  const gradX = new Float32Array(width * height);
  const gradY = new Float32Array(width * height);

  for (let y = 1; y < height - 1; y++) {
    for (let x = 1; x < width - 1; x++) {
      const i = y * width + x;
      const tl = depthData[(y - 1) * width + (x - 1)];
      const tc = depthData[(y - 1) * width + x];
      const tr = depthData[(y - 1) * width + (x + 1)];
      const ml = depthData[y * width + (x - 1)];
      const mr = depthData[y * width + (x + 1)];
      const bl = depthData[(y + 1) * width + (x - 1)];
      const bc = depthData[(y + 1) * width + x];
      const br = depthData[(y + 1) * width + (x + 1)];

      gradX[i] = (-tl + tr - 2 * ml + 2 * mr - bl + br) / 8.0;
      gradY[i] = (-tl - 2 * tc - tr + bl + 2 * bc + br) / 8.0;
    }
  }

  // Step 2: Build per-pixel flow vectors based on region classification
  console.log('[flowmap] Assigning flow vectors per region...');
  const flowX = new Float32Array(width * height);
  const flowY = new Float32Array(width * height);

  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const i = y * width + x;
      const nx = x / width;   // normalized 0-1
      const ny = y / height;

      // Find the best matching region for this pixel
      const region = findRegion(nx, ny, depthData[i], recipe);
      if (!region) continue;

      const { animation_class, flow_direction, flow_speed } = region;
      assignFlowVector(flowX, flowY, i, animation_class, flow_direction, flow_speed, gradX[i], gradY[i]);
    }
  }

  // Step 3: Blur flow field to smooth region boundaries
  console.log('[flowmap] Smoothing boundaries...');
  for (let pass = 0; pass < 3; pass++) {
    blurFlowField(flowX, flowY, width, height);
  }

  // Step 4: Encode to RGB PNG (R=flow_x, G=flow_y mapped from [-1,1] to [0,255])
  const rgb = Buffer.alloc(width * height * 3);
  for (let i = 0; i < width * height; i++) {
    rgb[i * 3 + 0] = Math.round(Math.max(0, Math.min(255, (flowX[i] * 0.5 + 0.5) * 255)));
    rgb[i * 3 + 1] = Math.round(Math.max(0, Math.min(255, (flowY[i] * 0.5 + 0.5) * 255)));
    rgb[i * 3 + 2] = 128; // B channel unused, set to neutral
  }

  const png = await sharp(rgb, { raw: { width, height, channels: 3 } })
    .png()
    .toBuffer();

  console.log(`[flowmap] Output: ${width}x${height} PNG (${(png.length / 1024).toFixed(0)}KB)`);
  return png;
}

function findRegion(
  nx: number, ny: number, depth: number,
  recipe: ImageRecipe
): ImageRecipe['regions'][0] | null {
  // Score each region by how well this pixel fits
  let bestRegion: ImageRecipe['regions'][0] | null = null;
  let bestScore = -Infinity;

  for (const region of recipe.regions) {
    const { bounds, depth_hint } = region;

    // Check if pixel is within the region bounds (with soft edges)
    const inX = nx >= bounds.x && nx <= bounds.x + bounds.width;
    const inY = ny >= bounds.y && ny <= bounds.y + bounds.height;
    if (!inX || !inY) continue;

    // Score: prefer regions where depth matches the hint
    const depthDiff = Math.abs(depth - depth_hint);
    const score = 1.0 - depthDiff;

    if (score > bestScore) {
      bestScore = score;
      bestRegion = region;
    }
  }

  return bestRegion;
}

function assignFlowVector(
  flowX: Float32Array, flowY: Float32Array, i: number,
  animClass: AnimationClass,
  direction: [number, number],
  speed: number,
  gx: number, gy: number
): void {
  switch (animClass) {
    case 'water': {
      // Water flows in depth gradient direction (downhill)
      const mag = Math.sqrt(gx * gx + gy * gy) + 1e-6;
      // Blend gradient direction with specified direction
      const blendedX = (gx / mag) * 0.6 + direction[0] * 0.4;
      const blendedY = (gy / mag) * 0.6 + direction[1] * 0.4;
      const bMag = Math.sqrt(blendedX * blendedX + blendedY * blendedY) + 1e-6;
      flowX[i] = (blendedX / bMag) * speed;
      flowY[i] = (blendedY / bMag) * speed;
      break;
    }
    case 'sky':
    case 'smoke': {
      // Constant directional flow
      flowX[i] = direction[0] * speed;
      flowY[i] = direction[1] * speed;
      break;
    }
    case 'fire': {
      // Upward flow
      flowX[i] = direction[0] * speed * 0.3;
      flowY[i] = -speed; // upward in UV space
      break;
    }
    case 'vegetation':
    case 'static':
    default: {
      // No directional flow — animation handled by warp/distort in shader
      flowX[i] = 0;
      flowY[i] = 0;
      break;
    }
  }
}

function blurFlowField(
  flowX: Float32Array, flowY: Float32Array,
  width: number, height: number
): void {
  const tmpX = new Float32Array(flowX);
  const tmpY = new Float32Array(flowY);

  for (let y = 1; y < height - 1; y++) {
    for (let x = 1; x < width - 1; x++) {
      let sx = 0, sy = 0;
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          const j = (y + dy) * width + (x + dx);
          sx += tmpX[j];
          sy += tmpY[j];
        }
      }
      const i = y * width + x;
      flowX[i] = sx / 9;
      flowY[i] = sy / 9;
    }
  }
}
