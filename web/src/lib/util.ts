export const reducedMotion =
	typeof matchMedia !== 'undefined' && matchMedia('(prefers-reduced-motion: reduce)').matches;

/** Transition/animation duration, collapsed to 0 when the user asked for less motion. */
export const dur = (ms: number) => (reducedMotion ? 0 : ms);

export function fmtTime(ms: number): string {
	const s = Math.max(0, Math.floor(ms / 1000));
	return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

export function initials(name: string): string {
	const parts = name.trim().split(/\s+/);
	return ((parts[0]?.[0] ?? '') + (parts[1]?.[0] ?? '')).toUpperCase() || '?';
}

/** Stable warm-ish hue per name, so avatars are distinguishable but on-palette. */
export function hue(name: string): number {
	let h = 0;
	for (const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
	return (h % 140) + 340; // 340..480 -> rose, orange, amber, olive
}
