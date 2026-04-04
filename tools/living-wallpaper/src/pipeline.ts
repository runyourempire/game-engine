/**
 * Full pipeline orchestrator.
 * Takes a single image path and produces all living wallpaper assets.
 */

import * as fs from 'fs';
import * as path from 'path';
import sharp from 'sharp';
import { estimateDepth } from './depth.js';
import { analyzeImage } from './analyze.js';
import { generateFlowMap } from './flowmap.js';
import { generateMasks } from './masks.js';
import { generateGameSource } from './generate-game.js';
import type { PipelineOutput } from './types.js';

export interface PipelineOptions {
  /** Target resolution for texture maps (default: match source image) */
  width?: number;
  height?: number;
  /** Skip depth estimation (use existing depth map) */
  depthMapPath?: string;
  /** Skip Claude analysis (use fallback recipe) */
  skipAnalysis?: boolean;
}

/**
 * Run the full living wallpaper pipeline.
 *
 * Input: path to a single image
 * Output: depth map, flow map, region masks, .game source code
 */
export async function runPipeline(
  imagePath: string,
  outputDir: string,
  options: PipelineOptions = {}
): Promise<PipelineOutput> {
  const absoluteImagePath = path.resolve(imagePath);
  const baseName = path.basename(imagePath, path.extname(imagePath));

  // Ensure output directory exists
  fs.mkdirSync(outputDir, { recursive: true });

  // Get source image dimensions
  const meta = await sharp(absoluteImagePath).metadata();
  const width = options.width ?? meta.width ?? 1920;
  const height = options.height ?? meta.height ?? 1080;
  console.log(`\n[pipeline] Processing: ${imagePath} (${width}x${height})`);

  // Copy source image to output
  const sourceExt = path.extname(imagePath);
  const sourceOutPath = path.join(outputDir, `${baseName}${sourceExt}`);
  fs.copyFileSync(absoluteImagePath, sourceOutPath);

  // Step 1: Depth estimation (or load existing) — single inference pass
  let depthPng: Buffer;
  let depthValues: Float32Array;

  if (options.depthMapPath) {
    console.log(`[pipeline] Using existing depth map: ${options.depthMapPath}`);
    depthPng = fs.readFileSync(options.depthMapPath);
    const depthRaw = await sharp(options.depthMapPath)
      .resize(width, height, { fit: 'fill' })
      .raw()
      .toBuffer();
    depthValues = new Float32Array(width * height);
    for (let i = 0; i < depthRaw.length; i++) {
      depthValues[i] = depthRaw[i] / 255.0;
    }
  } else {
    console.log('\n=== Step 1/4: Depth Estimation ===');
    const depthResult = await estimateDepth(absoluteImagePath, width, height);
    depthPng = depthResult.png;
    depthValues = depthResult.values;
  }

  const depthPath = path.join(outputDir, `${baseName}-depth.png`);
  fs.writeFileSync(depthPath, depthPng);
  console.log(`[pipeline] Saved: ${depthPath}`);

  // Step 2: Image analysis (Claude Vision or fallback)
  console.log('\n=== Step 2/4: Image Analysis ===');
  let resolvedRecipe;
  if (options.skipAnalysis) {
    // Use fallback recipe without API call
    const { analyzeImage: analyze } = await import('./analyze.js');
    resolvedRecipe = await analyze('__skip__');
  } else {
    resolvedRecipe = await analyzeImage(absoluteImagePath);
  }

  // Step 3: Generate flow map
  console.log('\n=== Step 3/4: Flow Map Generation ===');
  const flowPng = await generateFlowMap(depthValues, width, height, resolvedRecipe);

  const flowPath = path.join(outputDir, `${baseName}-flow.png`);
  fs.writeFileSync(flowPath, flowPng);
  console.log(`[pipeline] Saved: ${flowPath}`);

  // Step 4: Generate masks
  console.log('\n=== Step 4/4: Mask Generation ===');
  const masks = await generateMasks(depthValues, width, height, resolvedRecipe);

  for (const [maskName, maskPng] of masks) {
    const maskPath = path.join(outputDir, `${baseName}-${maskName}.png`);
    fs.writeFileSync(maskPath, maskPng);
    console.log(`[pipeline] Saved: ${maskPath}`);
  }

  // Step 5: Generate .game source
  console.log('\n=== Generating .game Source ===');
  const maskNames = Array.from(masks.keys());
  const hasWater = resolvedRecipe.regions.some(r => r.animation_class === 'water');
  const hasSky = resolvedRecipe.regions.some(r => r.animation_class === 'sky');
  const hasVegetation = resolvedRecipe.regions.some(r => r.animation_class === 'vegetation');

  const gameSource = generateGameSource(resolvedRecipe, {
    imageName: `${baseName}${sourceExt}`,
    outputDir,
    maskNames,
    hasWater,
    hasSky,
    hasVegetation,
  });

  const gamePath = path.join(outputDir, `${baseName}-living.game`);
  fs.writeFileSync(gamePath, gameSource);
  console.log(`[pipeline] Saved: ${gamePath}`);
  console.log(`\n[pipeline] Done! Generated files in ${outputDir}/`);
  console.log(`\nTo compile:\n  cargo run -- build ${gamePath} -o ${outputDir} --format html`);

  return {
    depthMap: depthPng,
    flowMap: flowPng,
    masks,
    gameSource,
    recipe: resolvedRecipe,
    width,
    height,
  };
}
