import { SHADER_IDS } from './shaders/index.ts';

export type Mode = 'halo' | 'bloom' | 'bars';
export const MODES: Mode[] = ['halo', 'bloom', 'bars'];
const KEY = 'cloudberry.vizMode';
const SHADER_KEY = 'cloudberry.leanShader';
const AUTO_KEY = 'cloudberry.leanAuto';

function read(key: string): string | null {
	try {
		return localStorage.getItem(key);
	} catch {
		return null; // storage blocked (private mode etc.): defaults it is
	}
}
function save(key: string, v: string) {
	try {
		localStorage.setItem(key, v);
	} catch {
		// not persisted, still works for this session
	}
}

const mode = read(KEY) as Mode;
const shader = read(SHADER_KEY) ?? '';

/** Shared visualizer UI state: mode, lean-back open, lean-back shader + auto-cycle (all but `lean` persisted). */
export const viz = $state({
	mode: MODES.includes(mode) ? mode : ('halo' as Mode),
	lean: false,
	shader: SHADER_IDS.includes(shader) ? shader : SHADER_IDS[0],
	auto: read(AUTO_KEY) === '1'
});

export function setMode(m: Mode) {
	viz.mode = m;
	save(KEY, m);
}

export function setShader(id: string) {
	if (!SHADER_IDS.includes(id)) return;
	viz.shader = id;
	save(SHADER_KEY, id);
}

/** Next (+1) / previous (-1) shader, wrapping. */
export function stepShader(dir: number) {
	const n = SHADER_IDS.length;
	setShader(SHADER_IDS[(((SHADER_IDS.indexOf(viz.shader) + dir) % n) + n) % n]);
}

export function setAuto(on: boolean) {
	viz.auto = on;
	save(AUTO_KEY, on ? '1' : '0');
}

/** Lean-back's own keys: ←/→ change shader, A toggles auto-cycle. True if handled. */
export function leanKey(key: string): boolean {
	if (key === 'ArrowLeft') stepShader(-1);
	else if (key === 'ArrowRight') stepShader(1);
	else if (key === 'a' || key === 'A') setAuto(!viz.auto);
	else return false;
	return true;
}

/** Call when the playing track changes: with auto-cycle on, move to the next shader. */
export function trackChanged() {
	if (viz.auto) stepShader(1);
}

/** Open/close lean-back. Call from a user gesture: it also asks for real fullscreen. */
export function setLean(on: boolean) {
	viz.lean = on;
	if (on) document.documentElement.requestFullscreen?.().catch(() => {});
	else if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
}
