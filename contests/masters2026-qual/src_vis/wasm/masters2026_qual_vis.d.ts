/* tslint:disable */
/* eslint-disable */

export class Ret {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    err: string;
    score: bigint;
    svg: string;
}

export function gen(seed: number, problem_id: string): string;

export function get_max_turn(input: string, output: string): number;

export function vis(input: string, output: string, turn: number): Ret;

export function vis_mode(input: string, output: string, turn: number, mode: number, focus_robot: number): Ret;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_ret_free: (a: number, b: number) => void;
    readonly __wbg_get_ret_score: (a: number) => bigint;
    readonly __wbg_set_ret_score: (a: number, b: bigint) => void;
    readonly __wbg_get_ret_err: (a: number) => [number, number];
    readonly __wbg_set_ret_err: (a: number, b: number, c: number) => void;
    readonly __wbg_get_ret_svg: (a: number) => [number, number];
    readonly __wbg_set_ret_svg: (a: number, b: number, c: number) => void;
    readonly gen: (a: number, b: number, c: number) => [number, number];
    readonly get_max_turn: (a: number, b: number, c: number, d: number) => number;
    readonly vis: (a: number, b: number, c: number, d: number, e: number) => [number, number, number];
    readonly vis_mode: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
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
