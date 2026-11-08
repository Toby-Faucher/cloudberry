import { bloom } from './bloom.ts';
import { scope } from './scope.ts';
import { aurora } from './aurora.ts';
import { drupelets } from './drupelets.ts';
import { tunnel } from './tunnel.ts';
import { halftone } from './halftone.ts';

export { PRELUDE, type ShaderDef } from './common.ts';
/** Picker order. */
export const SHADERS = [bloom, scope, aurora, drupelets, tunnel, halftone];
export type ShaderId = (typeof SHADERS)[number]['id'];
export const SHADER_IDS: string[] = SHADERS.map((s) => s.id);
