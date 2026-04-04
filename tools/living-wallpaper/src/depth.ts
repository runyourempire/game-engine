/**
 * Depth estimation using Depth Anything V2 Small via Transformers.js.
 * Runs on ONNX Runtime (native in Node.js, WASM in browser).
 * Model: onnx-community/depth-anything-v2-small (~25MB quantized)
 */

import { pipeline, type DepthEstimationPipeline, RawImage } from '@huggingface/transformers';
import sharp from 'sharp';

let depthEstimator: DepthEstimationPipeline | null = null;
let initPromise: Promise<DepthEstimationPipeline> | null = null;

/** Initialize the depth estimation model (cached after first call, safe for concurrent access) */
async function getDepthEstimator(): Promise<DepthEstimationPipeline> {
  if (depthEstimator) return depthEstimator;
  if (!initPromise) {
    initPromise = (async () => {
      console.log('[depth] Loading Depth Anything V2 Small...');
      const estimator = await pipeline(
        'depth-estimation',
        'onnx-community/depth-anything-v2-small',
      );
      console.log('[depth] Model loaded.');
      depthEstimator = estimator;
      return estimator;
    })();
  }
  return initPromise;
}

export interface DepthResult {
  /** Grayscale PNG buffer */
  png: Buffer;
  /** Normalized 0-1 float values at target resolution */
  values: Float32Array;
  width: number;
  height: number;
}

/**
 * Estimate depth from an image file.
 * Returns both a PNG and raw float values in a single inference pass.
 */
export async function estimateDepth(
  imagePath: string,
  targetWidth: number,
  targetHeight: number
): Promise<DepthResult> {
  const estimator = await getDepthEstimator();

  console.log('[depth] Running inference...');
  const rawImage = await RawImage.fromURL(imagePath);
  const result = await estimator(rawImage);

  const depthImage = result.depth as RawImage;
  const depthData = depthImage.data as Uint8ClampedArray;
  const dw = depthImage.width;
  const dh = depthImage.height;

  console.log(`[depth] Raw depth: ${dw}x${dh}`);

  // Resize to target resolution
  const grayscale = Buffer.from(depthData);
  const [png, resized] = await Promise.all([
    sharp(grayscale, { raw: { width: dw, height: dh, channels: 1 } })
      .resize(targetWidth, targetHeight, { fit: 'fill' })
      .png()
      .toBuffer(),
    sharp(grayscale, { raw: { width: dw, height: dh, channels: 1 } })
      .resize(targetWidth, targetHeight, { fit: 'fill' })
      .raw()
      .toBuffer(),
  ]);

  // Normalize to 0-1
  const values = new Float32Array(targetWidth * targetHeight);
  for (let i = 0; i < resized.length; i++) {
    values[i] = resized[i] / 255.0;
  }

  console.log(`[depth] Output: ${targetWidth}x${targetHeight} PNG (${(png.length / 1024).toFixed(0)}KB)`);
  return { png, values, width: targetWidth, height: targetHeight };
}
