#!/usr/bin/env node
/**
 * CLI entry point for the living wallpaper pipeline.
 *
 * Usage:
 *   npx tsx src/cli.ts <image> [output-dir]
 *   npx tsx src/cli.ts photo.jpg ./output
 *   npx tsx src/cli.ts photo.jpg ./output --skip-analysis
 *   npx tsx src/cli.ts photo.jpg ./output --depth existing-depth.png
 *
 * Environment:
 *   ANTHROPIC_API_KEY — for Claude Vision analysis (optional, falls back to generic recipe)
 */

import * as path from 'path';
import { runPipeline, type PipelineOptions } from './pipeline.js';

function usage(): never {
  console.log(`
GAME Living Wallpaper Pipeline
===============================

Transforms a single image into a region-aware living wallpaper.

Usage:
  npx tsx src/cli.ts <image> [output-dir] [options]

Options:
  --skip-analysis       Skip Claude Vision API, use default landscape recipe
  --depth <path>        Use existing depth map instead of running inference
  --width <n>           Target width (default: source image width)
  --height <n>          Target height (default: source image height)

Environment:
  ANTHROPIC_API_KEY     Claude API key for intelligent region analysis

Examples:
  npx tsx src/cli.ts landscape.jpg ./output
  npx tsx src/cli.ts beach.png ./output --skip-analysis
  npx tsx src/cli.ts photo.jpg ./output --depth photo-depth.png

Output:
  <name>-depth.png      Depth map (bright=near, dark=far)
  <name>-flow.png       Flow direction map (RG channels)
  <name>-mask_water.png Water region mask
  <name>-mask_sky.png   Sky region mask
  <name>-living.game    GAME source code (ready to compile)
`);
  process.exit(1);
}

async function main() {
  const args = process.argv.slice(2);
  if (args.length === 0 || args[0] === '--help' || args[0] === '-h') {
    usage();
  }

  const imagePath = args[0];
  const outputDir = args[1] || './output';

  const options: PipelineOptions = {};

  // Parse flags
  for (let i = 2; i < args.length; i++) {
    switch (args[i]) {
      case '--skip-analysis':
        options.skipAnalysis = true;
        break;
      case '--depth':
        options.depthMapPath = args[++i];
        break;
      case '--width':
        options.width = parseInt(args[++i], 10);
        break;
      case '--height':
        options.height = parseInt(args[++i], 10);
        break;
      default:
        console.error(`Unknown option: ${args[i]}`);
        usage();
    }
  }

  console.log('GAME Living Wallpaper Pipeline');
  console.log('=============================');

  const start = Date.now();
  await runPipeline(imagePath, outputDir, options);
  const elapsed = ((Date.now() - start) / 1000).toFixed(1);
  console.log(`\nTotal time: ${elapsed}s`);
}

main().catch(err => {
  console.error('Pipeline failed:', err);
  process.exit(1);
});
