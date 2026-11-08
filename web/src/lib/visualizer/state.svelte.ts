export type Mode = 'halo' | 'bloom' | 'bars';
export const MODES: Mode[] = ['halo', 'bloom', 'bars'];
const KEY = 'cloudberry.vizMode';

function load(): Mode {
	try {
		const v = localStorage.getItem(KEY) as Mode;
		if (MODES.includes(v)) return v;
	} catch {
		// storage blocked (private mode etc.): default it is
	}
	return 'halo';
}

/** Shared visualizer UI state: chosen mode (persisted) and whether lean-back is open. */
export const viz = $state({ mode: load(), lean: false });

export function setMode(m: Mode) {
	viz.mode = m;
	try {
		localStorage.setItem(KEY, m);
	} catch {
		// not persisted, still works for this session
	}
}

/** Open/close lean-back. Call from a user gesture: it also asks for real fullscreen. */
export function setLean(on: boolean) {
	viz.lean = on;
	if (on) document.documentElement.requestFullscreen?.().catch(() => {});
	else if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
}
