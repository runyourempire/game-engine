/* tslint:disable */
/* eslint-disable */

/**
 * Compile `.game` source to a Web Component ES module.
 *
 * `tag_name` must be a valid custom element name (must contain a hyphen).
 * Returns the JavaScript module string on success, or throws a JS error on failure.
 */
export function compile_to_component(source: string, tag_name: string): string;

/**
 * Compile `.game` source to a self-contained HTML file with WebGPU rendering.
 *
 * Returns the HTML string on success, or throws a JS error on failure.
 */
export function compile_to_html(source: string): string;

/**
 * Compile `.game` source to WGSL shader code.
 *
 * Returns the WGSL string on success, or throws a JS error on failure.
 */
export function compile_to_wgsl(source: string): string;

/**
 * Validate `.game` source without full compilation.
 *
 * Returns a JSON object with:
 * - `valid`: boolean
 * - `error`: string (only if invalid)
 * - `warnings`: string[] (only if valid)
 * - `layers`: number (only if valid)
 * - `params`: string[] (only if valid)
 * - `uses_audio`: boolean (only if valid)
 * - `uses_mouse`: boolean (only if valid)
 */
export function validate(source: string): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly compile_to_component: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly compile_to_html: (a: number, b: number) => [number, number, number, number];
    readonly compile_to_wgsl: (a: number, b: number) => [number, number, number, number];
    readonly validate: (a: number, b: number) => any;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
