/** Animation behavior for a region */
export type AnimationClass = 'static' | 'water' | 'sky' | 'vegetation' | 'fire' | 'smoke';

/** A region identified by Claude Vision with animation parameters */
export interface RegionRecipe {
  name: string;
  bounds: { x: number; y: number; width: number; height: number };
  depth_hint: number;
  animation_class: AnimationClass;
  flow_direction: [number, number];
  flow_speed: number;
  warp_amount: number;
  distort_frequency: number;
}

/** Full image analysis result from Claude Vision */
export interface ImageRecipe {
  scene_type: string;
  regions: RegionRecipe[];
  global_wind_direction: [number, number];
  ambient_motion_intensity: number;
}

/** Pipeline output: all generated assets */
export interface PipelineOutput {
  depthMap: Buffer;
  flowMap: Buffer;
  masks: Map<string, Buffer>;
  gameSource: string;
  recipe: ImageRecipe;
  width: number;
  height: number;
}
