const vscode = require('vscode');
const path = require('path');
const fs = require('fs');

/** @type {import('vscode').DiagnosticCollection} */
let diagnosticCollection;

/** @type {any} WASM module (loaded lazily) */
let wasmModule = null;

/** @type {boolean} Whether we've already warned about missing WASM */
let wasmWarningShown = false;

/** @type {import('vscode').WebviewPanel | null} */
let previewPanel = null;

/** @type {NodeJS.Timeout | null} */
let diagnosticDebounce = null;

const DEBOUNCE_MS = 500;

// ═══════════════════════════════════════════════════════════════════════════
// Language Knowledge Base
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Complete builtin function database with signatures, docs, and type states.
 */
const BUILTINS = [
    // ── SDF Primitives ─────────────────────────────────────────────────
    {
        name: 'circle',
        signature: 'circle(radius: 0.2)',
        description: 'Signed distance field for a circle.',
        detail: 'SDF Primitive',
        typeState: 'Position → Sdf',
        params: [{ name: 'radius', default: '0.2', description: 'Circle radius in UV space' }],
    },
    {
        name: 'ring',
        signature: 'ring(radius: 0.3, width: 0.02)',
        description: 'Hollow ring (annulus) SDF.',
        detail: 'SDF Primitive',
        typeState: 'Position → Sdf',
        params: [
            { name: 'radius', default: '0.3', description: 'Outer radius' },
            { name: 'width', default: '0.02', description: 'Ring thickness' },
        ],
    },
    {
        name: 'star',
        signature: 'star(points: 5.0, radius: 0.3, inner: 0.15)',
        description: 'Star polygon SDF with configurable point count.',
        detail: 'SDF Primitive',
        typeState: 'Position → Sdf',
        params: [
            { name: 'points', default: '5.0', description: 'Number of star points' },
            { name: 'radius', default: '0.3', description: 'Outer radius' },
            { name: 'inner', default: '0.15', description: 'Inner radius (controls pointiness)' },
        ],
    },
    {
        name: 'box',
        signature: 'box(w: 0.2, h: 0.2)',
        description: 'Axis-aligned rectangle SDF.',
        detail: 'SDF Primitive',
        typeState: 'Position → Sdf',
        params: [
            { name: 'w', default: '0.2', description: 'Width' },
            { name: 'h', default: '0.2', description: 'Height' },
        ],
    },
    {
        name: 'polygon',
        signature: 'polygon(sides: 6.0, radius: 0.3)',
        description: 'Regular polygon SDF.',
        detail: 'SDF Primitive',
        typeState: 'Position → Sdf',
        params: [
            { name: 'sides', default: '6.0', description: 'Number of sides' },
            { name: 'radius', default: '0.3', description: 'Circumscribed radius' },
        ],
    },

    // ── Noise Functions ────────────────────────────────────────────────
    {
        name: 'fbm',
        signature: 'fbm(scale: 1.0, octaves: 4.0, persistence: 0.5, lacunarity: 2.0)',
        description: 'Fractal Brownian Motion — layered noise for organic textures.',
        detail: 'Noise',
        typeState: 'Position → Sdf',
        params: [
            { name: 'scale', default: '1.0', description: 'Spatial frequency' },
            { name: 'octaves', default: '4.0', description: 'Number of noise layers' },
            { name: 'persistence', default: '0.5', description: 'Amplitude decay per octave' },
            { name: 'lacunarity', default: '2.0', description: 'Frequency multiplier per octave' },
        ],
    },
    {
        name: 'simplex',
        signature: 'simplex(scale: 1.0)',
        description: 'Simplex noise — smooth, gradient-based noise function.',
        detail: 'Noise',
        typeState: 'Position → Sdf',
        params: [{ name: 'scale', default: '1.0', description: 'Noise frequency' }],
    },
    {
        name: 'voronoi',
        signature: 'voronoi(scale: 5.0)',
        description: 'Voronoi (cellular) noise — distance to nearest random point.',
        detail: 'Noise',
        typeState: 'Position → Sdf',
        params: [{ name: 'scale', default: '5.0', description: 'Cell density' }],
    },
    {
        name: 'concentric_waves',
        signature: 'concentric_waves(amplitude: 1.0, width: 0.5, frequency: 3.0)',
        description: 'Expanding concentric wave pattern.',
        detail: 'Noise',
        typeState: 'Position → Sdf',
        params: [
            { name: 'amplitude', default: '1.0', description: 'Wave height' },
            { name: 'width', default: '0.5', description: 'Ring width' },
            { name: 'frequency', default: '3.0', description: 'Ring count' },
        ],
    },
    {
        name: 'curl_noise',
        signature: 'curl_noise(frequency: 1.0, amplitude: 0.1)',
        description: 'Divergence-free noise for fluid-like motion.',
        detail: 'Noise',
        typeState: 'Position → Sdf',
        params: [
            { name: 'frequency', default: '1.0', description: 'Noise frequency' },
            { name: 'amplitude', default: '0.1', description: 'Displacement strength' },
        ],
    },

    // ── Visual / Shading ───────────────────────────────────────────────
    {
        name: 'glow',
        signature: 'glow(intensity: 1.5)',
        description: 'Adds soft glow emission around SDF edges.',
        detail: 'Visual',
        typeState: 'Sdf → Color',
        params: [{ name: 'intensity', default: '1.5', description: 'Glow brightness multiplier' }],
    },
    {
        name: 'shade',
        signature: 'shade(r: 1.0, g: 1.0, b: 1.0)',
        description: 'Fill an SDF with a solid color. Uses fwidth() anti-aliasing.',
        detail: 'Visual',
        typeState: 'Sdf → Color',
        params: [
            { name: 'r', default: '1.0', description: 'Red channel (0.0 - 1.0)' },
            { name: 'g', default: '1.0', description: 'Green channel (0.0 - 1.0)' },
            { name: 'b', default: '1.0', description: 'Blue channel (0.0 - 1.0)' },
        ],
    },
    {
        name: 'emissive',
        signature: 'emissive(intensity: 1.0)',
        description: 'Apply emissive lighting (self-illumination) to a shape.',
        detail: 'Visual',
        typeState: 'Sdf → Color',
        params: [{ name: 'intensity', default: '1.0', description: 'Emission strength' }],
    },
    {
        name: 'tint',
        signature: 'tint(r: 1.0, g: 1.0, b: 1.0)',
        description: 'Multiply-blend a color tint over existing color.',
        detail: 'Visual',
        typeState: 'Color → Color',
        params: [
            { name: 'r', default: '1.0', description: 'Red multiplier' },
            { name: 'g', default: '1.0', description: 'Green multiplier' },
            { name: 'b', default: '1.0', description: 'Blue multiplier' },
        ],
    },
    {
        name: 'gradient',
        signature: 'gradient(color_a, color_b, mode)',
        description: 'Linear or radial gradient between two colors.',
        detail: 'Visual',
        typeState: 'Position → Color',
        params: [
            { name: 'color_a', default: 'black', description: 'Start color' },
            { name: 'color_b', default: 'white', description: 'End color' },
            { name: 'mode', default: 'linear', description: 'Gradient mode (linear, radial)' },
        ],
    },
    {
        name: 'spectrum',
        signature: 'spectrum(bass: 0.0, mid: 0.0, treble: 0.0)',
        description: 'Color mapping based on audio frequency bands.',
        detail: 'Visual',
        typeState: 'Sdf → Color',
        params: [
            { name: 'bass', default: '0.0', description: 'Low frequency influence' },
            { name: 'mid', default: '0.0', description: 'Mid frequency influence' },
            { name: 'treble', default: '0.0', description: 'High frequency influence' },
        ],
    },

    // ── Post-Processing ────────────────────────────────────────────────
    {
        name: 'bloom',
        signature: 'bloom(threshold: 0.3, strength: 2.0)',
        description: 'Bright-pass bloom effect — extracts highlights and blurs them.',
        detail: 'Post-Processing',
        typeState: 'Color → Color',
        params: [
            { name: 'threshold', default: '0.3', description: 'Luminance threshold for bloom' },
            { name: 'strength', default: '2.0', description: 'Bloom intensity' },
        ],
    },
    {
        name: 'grain',
        signature: 'grain(amount: 0.1)',
        description: 'Film grain noise overlay.',
        detail: 'Post-Processing',
        typeState: 'Color → Color',
        params: [{ name: 'amount', default: '0.1', description: 'Grain strength (0.0 - 1.0)' }],
    },
    {
        name: 'blend',
        signature: 'blend(factor: 0.5)',
        description: 'Blend between two layers or values.',
        detail: 'Post-Processing',
        typeState: 'Color → Color',
        params: [{ name: 'factor', default: '0.5', description: 'Blend factor (0.0 = a, 1.0 = b)' }],
    },
    {
        name: 'vignette',
        signature: 'vignette(strength: 0.5, radius: 0.8)',
        description: 'Darkens edges for cinematic framing.',
        detail: 'Post-Processing',
        typeState: 'Color → Color',
        params: [
            { name: 'strength', default: '0.5', description: 'Darkening intensity' },
            { name: 'radius', default: '0.8', description: 'Clear center radius' },
        ],
    },
    {
        name: 'tonemap',
        signature: 'tonemap(exposure: 1.0)',
        description: 'HDR tonemapping (ACES filmic curve).',
        detail: 'Post-Processing',
        typeState: 'Color → Color',
        params: [{ name: 'exposure', default: '1.0', description: 'Exposure multiplier' }],
    },
    {
        name: 'scanlines',
        signature: 'scanlines(frequency: 200.0, intensity: 0.3)',
        description: 'CRT-style horizontal scanline overlay.',
        detail: 'Post-Processing',
        typeState: 'Color → Color',
        params: [
            { name: 'frequency', default: '200.0', description: 'Scanline density' },
            { name: 'intensity', default: '0.3', description: 'Overlay strength' },
        ],
    },
    {
        name: 'chromatic',
        signature: 'chromatic(offset: 0.005)',
        description: 'Chromatic aberration — splits RGB channels.',
        detail: 'Post-Processing',
        typeState: 'Color → Color',
        params: [{ name: 'offset', default: '0.005', description: 'Channel separation amount' }],
    },
    {
        name: 'saturate_color',
        signature: 'saturate_color(amount: 1.0)',
        description: 'Adjust color saturation. 0 = grayscale, 1 = unchanged, >1 = vivid.',
        detail: 'Post-Processing',
        typeState: 'Color → Color',
        params: [{ name: 'amount', default: '1.0', description: 'Saturation multiplier' }],
    },
    {
        name: 'glitch',
        signature: 'glitch(intensity: 0.5)',
        description: 'Digital glitch distortion effect.',
        detail: 'Post-Processing',
        typeState: 'Color → Color',
        params: [{ name: 'intensity', default: '0.5', description: 'Glitch strength' }],
    },

    // ── Domain Transforms ──────────────────────────────────────────────
    {
        name: 'translate',
        signature: 'translate(x: 0.0, y: 0.0)',
        description: 'Translate (move) the coordinate space.',
        detail: 'Domain Transform',
        typeState: 'Position → Position',
        params: [
            { name: 'x', default: '0.0', description: 'Horizontal offset' },
            { name: 'y', default: '0.0', description: 'Vertical offset' },
        ],
    },
    {
        name: 'rotate',
        signature: 'rotate(angle: 0.0)',
        description: 'Rotate the coordinate space (radians).',
        detail: 'Domain Transform',
        typeState: 'Position → Position',
        params: [{ name: 'angle', default: '0.0', description: 'Rotation angle in radians' }],
    },
    {
        name: 'scale',
        signature: 'scale(s: 1.0)',
        description: 'Uniform scale of the coordinate space.',
        detail: 'Domain Transform',
        typeState: 'Position → Position',
        params: [{ name: 's', default: '1.0', description: 'Scale factor' }],
    },
    {
        name: 'twist',
        signature: 'twist(amount: 0.0)',
        description: 'Twist distortion — rotates more at extremes.',
        detail: 'Domain Transform',
        typeState: 'Position → Position',
        params: [{ name: 'amount', default: '0.0', description: 'Twist intensity' }],
    },
    {
        name: 'mirror',
        signature: 'mirror(axis: 0.0)',
        description: 'Mirror (reflect) across an axis. 0 = vertical, 1 = horizontal.',
        detail: 'Domain Transform',
        typeState: 'Position → Position',
        params: [{ name: 'axis', default: '0.0', description: 'Mirror axis (0 = vertical, 1 = horizontal)' }],
    },
    {
        name: 'repeat',
        signature: 'repeat(count: 4.0)',
        description: 'Tile/repeat the space in a polar or grid pattern.',
        detail: 'Domain Transform',
        typeState: 'Position → Position',
        params: [{ name: 'count', default: '4.0', description: 'Repetition count' }],
    },
    {
        name: 'domain_warp',
        signature: 'domain_warp(amount: 0.1, freq: 3.0)',
        description: 'Warp coordinates using noise for organic distortion.',
        detail: 'Domain Transform',
        typeState: 'Position → Position',
        params: [
            { name: 'amount', default: '0.1', description: 'Warp displacement strength' },
            { name: 'freq', default: '3.0', description: 'Noise frequency' },
        ],
    },
    {
        name: 'displace',
        signature: 'displace(strength: 0.1)',
        description: 'Displace SDF surface using noise.',
        detail: 'Domain Transform',
        typeState: 'Sdf → Sdf',
        params: [{ name: 'strength', default: '0.1', description: 'Displacement amount' }],
    },

    // ── SDF Modifiers ──────────────────────────────────────────────────
    {
        name: 'mask_arc',
        signature: 'mask_arc(angle)',
        description: 'Mask an SDF to a circular arc sector.',
        detail: 'SDF Modifier',
        typeState: 'Sdf → Sdf',
        params: [{ name: 'angle', default: '3.14', description: 'Arc angle in radians' }],
    },
    {
        name: 'threshold',
        signature: 'threshold(cutoff: 0.5)',
        description: 'Hard cutoff — values below cutoff become 0, above become 1.',
        detail: 'SDF Modifier',
        typeState: 'Sdf → Sdf',
        params: [{ name: 'cutoff', default: '0.5', description: 'Threshold value' }],
    },
    {
        name: 'onion',
        signature: 'onion(thickness: 0.02)',
        description: 'Hollow out an SDF to create concentric shells.',
        detail: 'SDF Modifier',
        typeState: 'Sdf → Sdf',
        params: [{ name: 'thickness', default: '0.02', description: 'Shell thickness' }],
    },
    {
        name: 'round',
        signature: 'round(radius: 0.02)',
        description: 'Round the corners/edges of an SDF.',
        detail: 'SDF Modifier',
        typeState: 'Sdf → Sdf',
        params: [{ name: 'radius', default: '0.02', description: 'Rounding radius' }],
    },
];

/**
 * Signal database: audio, mouse, time, and constants.
 */
const SIGNALS = [
    { name: 'audio.bass', description: 'Low-frequency audio energy.', range: '0.0 - 1.0' },
    { name: 'audio.mid', description: 'Mid-frequency audio energy.', range: '0.0 - 1.0' },
    { name: 'audio.treble', description: 'High-frequency audio energy.', range: '0.0 - 1.0' },
    { name: 'audio.energy', description: 'Total audio energy across all frequencies.', range: '0.0 - 1.0' },
    { name: 'audio.beat', description: 'Beat detection pulse (1.0 on beat, decays to 0.0).', range: '0.0 - 1.0' },
    { name: 'mouse.x', description: 'Horizontal mouse position in UV space.', range: '0.0 - 1.0' },
    { name: 'mouse.y', description: 'Vertical mouse position in UV space.', range: '0.0 - 1.0' },
    { name: 'time', description: 'Elapsed time in seconds since component mount.', range: '0.0 - inf' },
];

/**
 * Named color database with RGB values.
 */
const NAMED_COLORS = [
    { name: 'black', rgb: '0.0, 0.0, 0.0', hex: '#000000' },
    { name: 'white', rgb: '1.0, 1.0, 1.0', hex: '#FFFFFF' },
    { name: 'red', rgb: '1.0, 0.0, 0.0', hex: '#FF0000' },
    { name: 'green', rgb: '0.0, 1.0, 0.0', hex: '#00FF00' },
    { name: 'blue', rgb: '0.0, 0.0, 1.0', hex: '#0000FF' },
    { name: 'gold', rgb: '0.831, 0.686, 0.216', hex: '#D4AF37' },
    { name: 'midnight', rgb: '0.05, 0.05, 0.15', hex: '#0D0D26' },
    { name: 'obsidian', rgb: '0.07, 0.07, 0.09', hex: '#121217' },
    { name: 'ember', rgb: '0.9, 0.3, 0.1', hex: '#E64D1A' },
    { name: 'cyan', rgb: '0.0, 1.0, 1.0', hex: '#00FFFF' },
    { name: 'ivory', rgb: '1.0, 1.0, 0.94', hex: '#FFFFF0' },
    { name: 'frost', rgb: '0.85, 0.92, 1.0', hex: '#D9EBFF' },
    { name: 'orange', rgb: '1.0, 0.647, 0.0', hex: '#FFA500' },
    { name: 'deep_blue', rgb: '0.05, 0.1, 0.35', hex: '#0D1A59' },
    { name: 'ash', rgb: '0.45, 0.45, 0.45', hex: '#737373' },
    { name: 'charcoal', rgb: '0.2, 0.2, 0.2', hex: '#333333' },
    { name: 'plasma', rgb: '0.6, 0.2, 0.8', hex: '#9933CC' },
    { name: 'violet', rgb: '0.54, 0.17, 0.89', hex: '#8A2BE2' },
    { name: 'magenta', rgb: '1.0, 0.0, 1.0', hex: '#FF00FF' },
];

/**
 * Math constants.
 */
const CONSTANTS = [
    { name: 'pi', value: '3.14159265', description: 'Pi (ratio of circumference to diameter).' },
    { name: 'tau', value: '6.28318530', description: 'Tau (2 * pi, full circle in radians).' },
    { name: 'e', value: '2.71828182', description: 'Euler\'s number (base of natural logarithm).' },
    { name: 'phi', value: '1.61803398', description: 'Golden ratio.' },
];

/**
 * Language keywords.
 */
const KEYWORDS = [
    { name: 'cinematic', description: 'Top-level composition block. Contains layers, arcs, and interaction blocks.', syntax: 'cinematic "Name" { ... }' },
    { name: 'layer', description: 'A visual layer with a generative function pipeline.', syntax: 'layer name { fn: ... }' },
    { name: 'fn', description: 'Property that defines the generative function pipeline for a layer.', syntax: 'fn: circle(0.3) | glow(2.0)' },
    { name: 'define', description: 'Define a reusable function with parameters.', syntax: 'define name(param) { ... }' },
    { name: 'import', description: 'Import definitions from another .game file.', syntax: 'import "file.game" expose name1, name2' },
    { name: 'expose', description: 'Specifies which names to import from a file.', syntax: 'import "file.game" expose name' },
    { name: 'as', description: 'Alias an imported name.', syntax: 'import "file.game" expose name as alias' },
    { name: 'arc', description: 'Timeline block for keyframed animations with eased transitions.', syntax: 'arc { 0:00 "start" { ... } }' },
    { name: 'resonate', description: 'Cross-layer coupling — parameters influence each other.', syntax: 'resonate { a.x ~ b.y * 0.5 }' },
    { name: 'react', description: 'Interaction binding — maps events to actions.', syntax: 'react { mouse.click -> action(...) }' },
    { name: 'listen', description: 'Audio analysis block — configures audio input processing.', syntax: 'listen { source: mic, fft: 1024 }' },
    { name: 'voice', description: 'Voice/audio input processing block.', syntax: 'voice { ... }' },
    { name: 'score', description: 'Musical score definition for synchronized visuals.', syntax: 'score { bpm: 120, ... }' },
    { name: 'gravity', description: 'Physics gravity simulation block.', syntax: 'gravity { strength: 9.8, ... }' },
    { name: 'lens', description: 'Rendering configuration — mode, post-processing, camera.', syntax: 'lens name { mode: flat, ... }' },
    { name: 'breed', description: 'Evolutionary/generative breeding block.', syntax: 'breed { ... }' },
    { name: 'project', description: 'Project settings and metadata.', syntax: 'project { ... }' },
    { name: 'memory', description: 'Persistent state — values survive across frames. Use in layer block.', syntax: 'memory: [var1, var2]' },
    { name: 'cast', description: 'Broadcast values from one layer to others.', syntax: 'cast: [param1, param2]' },
    { name: 'ease', description: 'Apply an easing function to a transition.', syntax: 'ease(expo_out)' },
    { name: 'over', description: 'Duration specifier for transitions.', syntax: 'param -> value ease(smooth) over 3s' },
];

/**
 * Easing functions.
 */
const EASING_FUNCTIONS = [
    { name: 'linear', description: 'No easing — constant rate of change.' },
    { name: 'smooth', description: 'Hermite interpolation (smoothstep).' },
    { name: 'cubic_in_out', description: 'Cubic ease-in-out — slow start and end, fast middle.' },
    { name: 'expo_in', description: 'Exponential ease-in — slow start, fast end.' },
    { name: 'expo_out', description: 'Exponential ease-out — fast start, slow end.' },
    { name: 'elastic', description: 'Elastic overshoot — springs past target and settles.' },
    { name: 'bounce', description: 'Bounce effect — bounces at the end.' },
];

/**
 * Math/WGSL functions available in GAME.
 */
const MATH_FUNCTIONS = [
    { name: 'sin', signature: 'sin(x)', description: 'Sine of angle (radians).' },
    { name: 'cos', signature: 'cos(x)', description: 'Cosine of angle (radians).' },
    { name: 'tan', signature: 'tan(x)', description: 'Tangent of angle (radians).' },
    { name: 'abs', signature: 'abs(x)', description: 'Absolute value.' },
    { name: 'floor', signature: 'floor(x)', description: 'Round down to nearest integer.' },
    { name: 'ceil', signature: 'ceil(x)', description: 'Round up to nearest integer.' },
    { name: 'fract', signature: 'fract(x)', description: 'Fractional part (x - floor(x)).' },
    { name: 'sqrt', signature: 'sqrt(x)', description: 'Square root.' },
    { name: 'exp', signature: 'exp(x)', description: 'Natural exponential (e^x).' },
    { name: 'log', signature: 'log(x)', description: 'Natural logarithm (ln).' },
    { name: 'sign', signature: 'sign(x)', description: 'Sign of x (-1.0, 0.0, or 1.0).' },
    { name: 'round', signature: 'round(x)', description: 'Round to nearest integer.' },
    { name: 'length', signature: 'length(v)', description: 'Length (magnitude) of a vector.' },
    { name: 'normalize', signature: 'normalize(v)', description: 'Normalize a vector to unit length.' },
    { name: 'min', signature: 'min(a, b)', description: 'Minimum of two values.' },
    { name: 'max', signature: 'max(a, b)', description: 'Maximum of two values.' },
    { name: 'mix', signature: 'mix(a, b, t)', description: 'Linear interpolation: a * (1-t) + b * t.' },
    { name: 'clamp', signature: 'clamp(x, lo, hi)', description: 'Clamp x to range [lo, hi].' },
    { name: 'smoothstep', signature: 'smoothstep(edge0, edge1, x)', description: 'Hermite interpolation between edge0 and edge1.' },
    { name: 'step', signature: 'step(edge, x)', description: '0.0 if x < edge, else 1.0.' },
    { name: 'distance', signature: 'distance(a, b)', description: 'Euclidean distance between two points.' },
    { name: 'dot', signature: 'dot(a, b)', description: 'Dot product of two vectors.' },
    { name: 'cross', signature: 'cross(a, b)', description: 'Cross product of two 3D vectors.' },
    { name: 'reflect', signature: 'reflect(I, N)', description: 'Reflect incident vector I around normal N.' },
    { name: 'atan2', signature: 'atan2(y, x)', description: 'Two-argument arctangent (angle of vector).' },
    { name: 'mod', signature: 'mod(x, y)', description: 'Floor-based modulo (GLSL-style). Note: WGSL % is trunc-based.' },
];

// ═══════════════════════════════════════════════════════════════════════════
// WASM Loading
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Attempt to load the WASM compiler module from the bundled pkg/ directory.
 * Returns the module if available, or null if not found.
 */
async function loadWasm() {
    if (wasmModule) return wasmModule;

    const pkgDir = path.join(__dirname, '..', 'pkg');
    const wasmJsPath = path.join(pkgDir, 'game_compiler.js');

    if (!fs.existsSync(wasmJsPath)) {
        if (!wasmWarningShown) {
            wasmWarningShown = true;
            vscode.window.showInformationMessage(
                'GAME compiler WASM not found. Syntax highlighting is active, but ' +
                'diagnostics and preview require the WASM build. Run `wasm-pack build ' +
                '--target nodejs` in game-compiler/ and copy pkg/ into the extension.'
            );
        }
        return null;
    }

    try {
        wasmModule = require(wasmJsPath);
        // Some wasm-pack targets need explicit init
        if (typeof wasmModule.default === 'function') {
            await wasmModule.default();
        }
        return wasmModule;
    } catch (err) {
        if (!wasmWarningShown) {
            wasmWarningShown = true;
            vscode.window.showWarningMessage(
                `GAME: Failed to load WASM compiler: ${err.message}`
            );
        }
        return null;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Diagnostics
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Parse a compiler error message to extract byte offset span.
 * Error format: "message (at byte START..END)"
 * Returns { message: string, startByte: number | null, endByte: number | null }
 */
function parseError(errorMsg) {
    const spanMatch = errorMsg.match(/\(at byte (\d+)\.\.(\d+)\)/);
    if (spanMatch) {
        return {
            message: errorMsg.replace(/\s*\(at byte \d+\.\.\d+\)/, ''),
            startByte: parseInt(spanMatch[1], 10),
            endByte: parseInt(spanMatch[2], 10),
        };
    }
    return { message: errorMsg, startByte: null, endByte: null };
}

/**
 * Convert a byte offset to a vscode.Position using the document text.
 */
function byteOffsetToPosition(text, byteOffset) {
    let charIndex = 0;
    let byteIndex = 0;

    while (byteIndex < byteOffset && charIndex < text.length) {
        const code = text.charCodeAt(charIndex);
        if (code < 0x80) byteIndex += 1;
        else if (code < 0x800) byteIndex += 2;
        else if (code >= 0xD800 && code <= 0xDBFF) { byteIndex += 4; charIndex++; }
        else byteIndex += 3;
        charIndex++;
    }

    let line = 0;
    let col = 0;
    for (let i = 0; i < charIndex && i < text.length; i++) {
        if (text[i] === '\n') {
            line++;
            col = 0;
        } else {
            col++;
        }
    }
    return new vscode.Position(line, col);
}

/**
 * Run validation on the document and publish diagnostics.
 */
async function updateDiagnostics(document) {
    if (document.languageId !== 'game') return;

    const wasm = await loadWasm();
    if (!wasm) {
        diagnosticCollection.clear();
        return;
    }

    const source = document.getText();
    const diagnostics = [];

    try {
        const result = wasm.validate(source);

        if (!result.valid && result.error) {
            const parsed = parseError(result.error);
            let range;

            if (parsed.startByte !== null && parsed.endByte !== null) {
                const startPos = byteOffsetToPosition(source, parsed.startByte);
                const endPos = byteOffsetToPosition(source, parsed.endByte);
                range = new vscode.Range(startPos, endPos);
            } else {
                range = new vscode.Range(0, 0, 0, document.lineAt(0).text.length);
            }

            diagnostics.push(new vscode.Diagnostic(
                range,
                parsed.message,
                vscode.DiagnosticSeverity.Error
            ));
        }

        if (result.valid && result.warnings) {
            const warnings = Array.isArray(result.warnings)
                ? result.warnings
                : Array.from(result.warnings || []);

            for (const warning of warnings) {
                const parsed = parseError(warning);
                let range;

                if (parsed.startByte !== null && parsed.endByte !== null) {
                    const startPos = byteOffsetToPosition(source, parsed.startByte);
                    const endPos = byteOffsetToPosition(source, parsed.endByte);
                    range = new vscode.Range(startPos, endPos);
                } else {
                    range = new vscode.Range(0, 0, 0, document.lineAt(0).text.length);
                }

                diagnostics.push(new vscode.Diagnostic(
                    range,
                    parsed.message,
                    vscode.DiagnosticSeverity.Warning
                ));
            }
        }
    } catch (err) {
        const parsed = parseError(err.message || String(err));
        let range;

        if (parsed.startByte !== null && parsed.endByte !== null) {
            const startPos = byteOffsetToPosition(source, parsed.startByte);
            const endPos = byteOffsetToPosition(source, parsed.endByte);
            range = new vscode.Range(startPos, endPos);
        } else {
            range = new vscode.Range(0, 0, 0, Math.min(document.lineAt(0).text.length, 80));
        }

        diagnostics.push(new vscode.Diagnostic(
            range,
            parsed.message,
            vscode.DiagnosticSeverity.Error
        ));
    }

    diagnosticCollection.set(document.uri, diagnostics);
}

/**
 * Schedule a debounced diagnostics update.
 */
function scheduleDiagnostics(document) {
    if (diagnosticDebounce) {
        clearTimeout(diagnosticDebounce);
    }
    diagnosticDebounce = setTimeout(() => {
        updateDiagnostics(document);
    }, DEBOUNCE_MS);
}

// ═══════════════════════════════════════════════════════════════════════════
// Preview
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Update the preview panel with compiled HTML.
 */
async function updatePreview(document) {
    if (!previewPanel) return;
    if (document.languageId !== 'game') return;

    const wasm = await loadWasm();
    if (!wasm) return;

    const source = document.getText();

    try {
        const html = wasm.compile_to_html(source);
        previewPanel.webview.html = html;
    } catch (err) {
        previewPanel.webview.html = `
            <!DOCTYPE html>
            <html>
            <head>
                <style>
                    body {
                        background: #0A0A0A;
                        color: #EF4444;
                        font-family: 'JetBrains Mono', 'Fira Code', monospace;
                        padding: 2rem;
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        height: 100vh;
                        margin: 0;
                    }
                    .error {
                        background: #1F1F1F;
                        border: 1px solid #2A2A2A;
                        border-left: 4px solid #EF4444;
                        padding: 1.5rem;
                        border-radius: 4px;
                        max-width: 600px;
                        white-space: pre-wrap;
                        word-break: break-word;
                    }
                    .title {
                        color: #A0A0A0;
                        font-size: 0.85em;
                        margin-bottom: 0.5rem;
                    }
                </style>
            </head>
            <body>
                <div class="error">
                    <div class="title">Compilation Error</div>
                    ${escapeHtml(err.message || String(err))}
                </div>
            </body>
            </html>
        `;
    }
}

/**
 * Escape HTML entities for safe embedding.
 */
function escapeHtml(text) {
    return text
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
}

/**
 * Compile source and open result in a new untitled editor.
 */
async function compileAndShow(document, compileFunction, languageId) {
    const wasm = await loadWasm();
    if (!wasm) {
        vscode.window.showErrorMessage(
            'GAME compiler WASM not available. Build it first with wasm-pack.'
        );
        return;
    }

    const source = document.getText();

    try {
        const output = compileFunction(wasm, source);
        const doc = await vscode.workspace.openTextDocument({
            content: output,
            language: languageId,
        });
        await vscode.window.showTextDocument(doc, { preview: false });
    } catch (err) {
        vscode.window.showErrorMessage(`GAME compilation failed: ${err.message}`);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Completion Provider
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create a completion provider for GAME language.
 */
function createCompletionProvider() {
    return vscode.languages.registerCompletionItemProvider('game', {
        provideCompletionItems(document, position, token, context) {
            const lineText = document.lineAt(position).text;
            const textBeforeCursor = lineText.substring(0, position.character);
            const items = [];

            // Detect if we're after a dot (signal completion)
            const dotMatch = textBeforeCursor.match(/\b(audio|mouse|data)\.\s*$/);
            if (dotMatch) {
                const prefix = dotMatch[1];
                return getSignalMemberCompletions(prefix);
            }

            // Detect if we're inside ease()
            const easeMatch = textBeforeCursor.match(/ease\(\s*$/);
            if (easeMatch) {
                return getEasingCompletions();
            }

            // ── Builtin functions ──────────────────────────────────────
            for (const builtin of BUILTINS) {
                const item = new vscode.CompletionItem(builtin.name, vscode.CompletionItemKind.Function);
                item.detail = `${builtin.detail} -- ${builtin.signature}`;
                item.documentation = buildBuiltinDoc(builtin);
                item.insertText = new vscode.SnippetString(buildBuiltinSnippet(builtin));
                item.sortText = `0_${builtin.name}`;
                items.push(item);
            }

            // ── Math functions ─────────────────────────────────────────
            for (const fn of MATH_FUNCTIONS) {
                const item = new vscode.CompletionItem(fn.name, vscode.CompletionItemKind.Function);
                item.detail = `Math -- ${fn.signature}`;
                item.documentation = new vscode.MarkdownString(
                    `**${fn.signature}**\n\n${fn.description}`
                );
                item.sortText = `1_${fn.name}`;
                items.push(item);
            }

            // ── Signals ────────────────────────────────────────────────
            for (const signal of SIGNALS) {
                const item = new vscode.CompletionItem(signal.name, vscode.CompletionItemKind.Variable);
                item.detail = `Signal (${signal.range})`;
                item.documentation = new vscode.MarkdownString(
                    `**${signal.name}**\n\n${signal.description}\n\nRange: \`${signal.range}\``
                );
                item.sortText = `2_${signal.name}`;
                items.push(item);
            }

            // ── Named colors ───────────────────────────────────────────
            for (const color of NAMED_COLORS) {
                const item = new vscode.CompletionItem(color.name, vscode.CompletionItemKind.Color);
                item.detail = `Color -- rgb(${color.rgb})`;
                item.documentation = new vscode.MarkdownString(
                    `**${color.name}** \`${color.hex}\`\n\nRGB: \`${color.rgb}\``
                );
                item.sortText = `3_${color.name}`;
                items.push(item);
            }

            // ── Constants ──────────────────────────────────────────────
            for (const c of CONSTANTS) {
                const item = new vscode.CompletionItem(c.name, vscode.CompletionItemKind.Constant);
                item.detail = `Constant = ${c.value}`;
                item.documentation = new vscode.MarkdownString(
                    `**${c.name}** = \`${c.value}\`\n\n${c.description}`
                );
                item.sortText = `4_${c.name}`;
                items.push(item);
            }

            // ── Keywords ───────────────────────────────────────────────
            for (const kw of KEYWORDS) {
                const item = new vscode.CompletionItem(kw.name, vscode.CompletionItemKind.Keyword);
                item.detail = `Keyword`;
                item.documentation = new vscode.MarkdownString(
                    `**${kw.name}**\n\n${kw.description}\n\n\`\`\`game\n${kw.syntax}\n\`\`\``
                );
                item.sortText = `5_${kw.name}`;
                items.push(item);
            }

            // ── Easing functions (when not inside ease()) ──────────────
            for (const ef of EASING_FUNCTIONS) {
                const item = new vscode.CompletionItem(ef.name, vscode.CompletionItemKind.EnumMember);
                item.detail = `Easing`;
                item.documentation = new vscode.MarkdownString(
                    `**${ef.name}**\n\n${ef.description}\n\nUsage: \`ease(${ef.name})\``
                );
                item.sortText = `6_${ef.name}`;
                items.push(item);
            }

            return items;
        }
    }, '.', '(');
}

/**
 * Get completions for signal member access (after audio., mouse., etc.)
 */
function getSignalMemberCompletions(prefix) {
    const members = {
        audio: [
            { name: 'bass', description: 'Low-frequency audio energy.', range: '0.0 - 1.0' },
            { name: 'mid', description: 'Mid-frequency audio energy.', range: '0.0 - 1.0' },
            { name: 'treble', description: 'High-frequency audio energy.', range: '0.0 - 1.0' },
            { name: 'energy', description: 'Total audio energy across all frequencies.', range: '0.0 - 1.0' },
            { name: 'beat', description: 'Beat detection pulse (1.0 on beat, decays to 0.0).', range: '0.0 - 1.0' },
        ],
        mouse: [
            { name: 'x', description: 'Horizontal mouse position in UV space.', range: '0.0 - 1.0' },
            { name: 'y', description: 'Vertical mouse position in UV space.', range: '0.0 - 1.0' },
        ],
        data: [
            { name: 'progress', description: 'Generic progress value.', range: '0.0 - 1.0' },
            { name: 'value', description: 'Generic data value.', range: 'any' },
            { name: 'health', description: 'Health/status value.', range: '0.0 - 1.0' },
        ],
    };

    const list = members[prefix] || [];
    return list.map((m) => {
        const item = new vscode.CompletionItem(m.name, vscode.CompletionItemKind.Property);
        item.detail = `${prefix}.${m.name} (${m.range})`;
        item.documentation = new vscode.MarkdownString(
            `**${prefix}.${m.name}**\n\n${m.description}\n\nRange: \`${m.range}\``
        );
        return item;
    });
}

/**
 * Get completions for inside ease() call.
 */
function getEasingCompletions() {
    return EASING_FUNCTIONS.map((ef) => {
        const item = new vscode.CompletionItem(ef.name, vscode.CompletionItemKind.EnumMember);
        item.detail = `Easing function`;
        item.documentation = new vscode.MarkdownString(`**${ef.name}**\n\n${ef.description}`);
        return item;
    });
}

/**
 * Build a MarkdownString doc for a builtin function.
 */
function buildBuiltinDoc(builtin) {
    const md = new vscode.MarkdownString();
    md.appendCodeblock(builtin.signature, 'game');
    md.appendMarkdown(`\n\n${builtin.description}\n\n`);
    md.appendMarkdown(`**Type:** \`${builtin.typeState}\`\n\n`);
    if (builtin.params && builtin.params.length > 0) {
        md.appendMarkdown('**Parameters:**\n');
        for (const p of builtin.params) {
            md.appendMarkdown(`- \`${p.name}\` (default: ${p.default}) -- ${p.description}\n`);
        }
    }
    return md;
}

/**
 * Build a snippet string for a builtin function (tab stops for each param).
 */
function buildBuiltinSnippet(builtin) {
    if (!builtin.params || builtin.params.length === 0) {
        return `${builtin.name}()`;
    }
    const params = builtin.params
        .map((p, i) => `${p.name}: \${${i + 1}:${p.default}}`)
        .join(', ');
    return `${builtin.name}(${params})`;
}

// ═══════════════════════════════════════════════════════════════════════════
// Hover Provider
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create a hover provider for GAME language.
 */
function createHoverProvider() {
    return vscode.languages.registerHoverProvider('game', {
        provideHover(document, position) {
            const wordRange = document.getWordRangeAtPosition(position, /[a-zA-Z_][a-zA-Z0-9_]*/);
            if (!wordRange) return null;

            const word = document.getText(wordRange);

            // Check for signal with dot notation (e.g., audio.bass)
            const lineText = document.lineAt(position).text;
            const charIndex = wordRange.start.character;

            // Check if this word is the member part of a dotted signal
            if (charIndex >= 2 && lineText[charIndex - 1] === '.') {
                const prefixRange = document.getWordRangeAtPosition(
                    new vscode.Position(position.line, charIndex - 2),
                    /[a-zA-Z_][a-zA-Z0-9_]*/
                );
                if (prefixRange) {
                    const prefix = document.getText(prefixRange);
                    const fullSignal = `${prefix}.${word}`;
                    const signalHover = getSignalHover(fullSignal);
                    if (signalHover) return signalHover;
                }
            }

            // Check if this word is the prefix of a dotted signal
            const afterWord = lineText.substring(wordRange.end.character);
            if (afterWord.startsWith('.')) {
                const memberMatch = afterWord.match(/^\.([a-zA-Z_][a-zA-Z0-9_]*)/);
                if (memberMatch) {
                    const fullSignal = `${word}.${memberMatch[1]}`;
                    const signalHover = getSignalHover(fullSignal);
                    if (signalHover) return signalHover;
                }
            }

            // ── Builtin functions ──────────────────────────────────────
            const builtin = BUILTINS.find((b) => b.name === word);
            if (builtin) {
                return new vscode.Hover(buildBuiltinDoc(builtin), wordRange);
            }

            // ── Math functions ─────────────────────────────────────────
            const mathFn = MATH_FUNCTIONS.find((m) => m.name === word);
            if (mathFn) {
                const md = new vscode.MarkdownString();
                md.appendCodeblock(mathFn.signature, 'game');
                md.appendMarkdown(`\n\n${mathFn.description}`);
                return new vscode.Hover(md, wordRange);
            }

            // ── Standalone signals (time, beat, random) ────────────────
            const signal = SIGNALS.find((s) => s.name === word);
            if (signal) {
                const md = new vscode.MarkdownString();
                md.appendMarkdown(`**${signal.name}** -- Signal\n\n`);
                md.appendMarkdown(`${signal.description}\n\nRange: \`${signal.range}\``);
                return new vscode.Hover(md, wordRange);
            }

            // ── Named colors ───────────────────────────────────────────
            const color = NAMED_COLORS.find((c) => c.name === word);
            if (color) {
                const md = new vscode.MarkdownString();
                md.appendMarkdown(`**${color.name}** -- Named Color\n\n`);
                md.appendMarkdown(`Hex: \`${color.hex}\`\n\nRGB: \`${color.rgb}\``);
                return new vscode.Hover(md, wordRange);
            }

            // ── Constants ──────────────────────────────────────────────
            const constant = CONSTANTS.find((c) => c.name === word);
            if (constant) {
                const md = new vscode.MarkdownString();
                md.appendMarkdown(`**${constant.name}** = \`${constant.value}\`\n\n`);
                md.appendMarkdown(constant.description);
                return new vscode.Hover(md, wordRange);
            }

            // ── Keywords ───────────────────────────────────────────────
            const keyword = KEYWORDS.find((k) => k.name === word);
            if (keyword) {
                const md = new vscode.MarkdownString();
                md.appendMarkdown(`**${keyword.name}** -- Keyword\n\n`);
                md.appendMarkdown(`${keyword.description}\n\n`);
                md.appendCodeblock(keyword.syntax, 'game');
                return new vscode.Hover(md, wordRange);
            }

            // ── Easing functions ───────────────────────────────────────
            const easing = EASING_FUNCTIONS.find((e) => e.name === word);
            if (easing) {
                const md = new vscode.MarkdownString();
                md.appendMarkdown(`**${easing.name}** -- Easing Function\n\n`);
                md.appendMarkdown(`${easing.description}\n\nUsage: \`ease(${easing.name})\``);
                return new vscode.Hover(md, wordRange);
            }

            return null;
        }
    });
}

/**
 * Get hover for a dotted signal name (e.g., "audio.bass").
 */
function getSignalHover(fullName) {
    const signal = SIGNALS.find((s) => s.name === fullName);
    if (!signal) return null;

    const md = new vscode.MarkdownString();
    md.appendMarkdown(`**${signal.name}** -- Signal\n\n`);
    md.appendMarkdown(`${signal.description}\n\nRange: \`${signal.range}\``);
    return new vscode.Hover(md);
}

// ═══════════════════════════════════════════════════════════════════════════
// Extension Activation
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Extension activation.
 * @param {import('vscode').ExtensionContext} context
 */
function activate(context) {
    diagnosticCollection = vscode.languages.createDiagnosticCollection('game');
    context.subscriptions.push(diagnosticCollection);

    // ── Completion & Hover Providers ───────────────────────────────────

    context.subscriptions.push(createCompletionProvider());
    context.subscriptions.push(createHoverProvider());

    // ── Diagnostics on document change (debounced) ────────────────────

    context.subscriptions.push(
        vscode.workspace.onDidChangeTextDocument((event) => {
            if (event.document.languageId === 'game') {
                scheduleDiagnostics(event.document);
            }
        })
    );

    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument((document) => {
            if (document.languageId === 'game') {
                scheduleDiagnostics(document);
            }
        })
    );

    context.subscriptions.push(
        vscode.workspace.onDidSaveTextDocument((document) => {
            if (document.languageId === 'game') {
                updateDiagnostics(document);
                updatePreview(document);
            }
        })
    );

    // Run diagnostics on already-open .game files
    for (const editor of vscode.window.visibleTextEditors) {
        if (editor.document.languageId === 'game') {
            scheduleDiagnostics(editor.document);
        }
    }

    // ── Preview command ───────────────────────────────────────────────

    context.subscriptions.push(
        vscode.commands.registerCommand('game.preview', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'game') {
                vscode.window.showWarningMessage('Open a .game file to preview.');
                return;
            }

            if (previewPanel) {
                previewPanel.reveal(vscode.ViewColumn.Beside);
            } else {
                previewPanel = vscode.window.createWebviewPanel(
                    'gamePreview',
                    'GAME Preview',
                    vscode.ViewColumn.Beside,
                    {
                        enableScripts: true,
                        retainContextWhenHidden: true,
                    }
                );

                previewPanel.onDidDispose(() => {
                    previewPanel = null;
                }, null, context.subscriptions);
            }

            await updatePreview(editor.document);

            // Also update preview on text change (debounced)
            const changeDisposable = vscode.workspace.onDidChangeTextDocument(
                (event) => {
                    if (event.document === editor.document) {
                        if (diagnosticDebounce) clearTimeout(diagnosticDebounce);
                        diagnosticDebounce = setTimeout(() => {
                            updatePreview(event.document);
                            updateDiagnostics(event.document);
                        }, DEBOUNCE_MS);
                    }
                }
            );

            previewPanel.onDidDispose(() => {
                changeDisposable.dispose();
            });
        })
    );

    // ── Compile to WGSL command ───────────────────────────────────────

    context.subscriptions.push(
        vscode.commands.registerCommand('game.compile', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'game') {
                vscode.window.showWarningMessage('Open a .game file to compile.');
                return;
            }
            await compileAndShow(
                editor.document,
                (wasm, source) => wasm.compile_to_wgsl(source),
                'wgsl'
            );
        })
    );

    // ── Compile to HTML command ───────────────────────────────────────

    context.subscriptions.push(
        vscode.commands.registerCommand('game.compileHtml', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'game') {
                vscode.window.showWarningMessage('Open a .game file to compile.');
                return;
            }
            await compileAndShow(
                editor.document,
                (wasm, source) => wasm.compile_to_html(source),
                'html'
            );
        })
    );

    // ── Compile to Component command ──────────────────────────────────

    context.subscriptions.push(
        vscode.commands.registerCommand('game.compileComponent', async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'game') {
                vscode.window.showWarningMessage('Open a .game file to compile.');
                return;
            }

            const fileName = path.basename(
                editor.document.fileName,
                '.game'
            );
            let tagName = fileName
                .replace(/^\d+-?/, '')
                .replace(/_/g, '-')
                .toLowerCase();
            if (!tagName.includes('-')) {
                tagName = `game-${tagName}`;
            }

            await compileAndShow(
                editor.document,
                (wasm, source) => wasm.compile_to_component(source, tagName),
                'javascript'
            );
        })
    );

    // ── Auto-preview on open (if configured) ──────────────────────────

    const config = vscode.workspace.getConfiguration('game');
    if (config.get('autoPreview')) {
        context.subscriptions.push(
            vscode.window.onDidChangeActiveTextEditor((editor) => {
                if (editor && editor.document.languageId === 'game' && !previewPanel) {
                    vscode.commands.executeCommand('game.preview');
                }
            })
        );
    }
}

/**
 * Extension deactivation.
 */
function deactivate() {
    if (diagnosticDebounce) {
        clearTimeout(diagnosticDebounce);
    }
    if (diagnosticCollection) {
        diagnosticCollection.dispose();
    }
    if (previewPanel) {
        previewPanel.dispose();
        previewPanel = null;
    }
}

module.exports = { activate, deactivate };
