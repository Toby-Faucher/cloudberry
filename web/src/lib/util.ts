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

/**
 * Stable warm hue per name (FNV-1a, so "Toby" and "Tobi" land apart), in 340..480
 * mod 360 (rose, orange, amber, olive). Avatar.svelte pairs it with fixed oklch
 * lightness/chroma so text contrast holds in light and dark.
 */
export function hue(name: string): number {
	let h = 0x811c9dc5;
	for (const c of name.trim().toLowerCase()) h = Math.imul(h ^ c.codePointAt(0)!, 0x01000193) >>> 0;
	return ((h % 140) + 340) % 360;
}

// Same hosts the server accepts; the server re-validates, this just gives instant feedback.
const YT_HOSTS = ['youtube.com', 'www.youtube.com', 'm.youtube.com', 'music.youtube.com', 'youtu.be'];
export function isYouTubeUrl(v: string): boolean {
	try {
		const u = new URL(v.trim());
		return /^https?:$/.test(u.protocol) && YT_HOSTS.includes(u.hostname);
	} catch {
		return false;
	}
}

/** True when an event targets a text field (shortcuts and global paste must leave it alone). */
export function isTyping(t: EventTarget | null): boolean {
	const el = t as HTMLElement | null;
	return !!el && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName ?? ''));
}

const BRAND = [240, 145, 43]; // --berry-500 #F0912B

/**
 * Per-track tint: the artwork's average colour pulled halfway toward the accent (`brand`, rgb),
 * then held to a saturation/lightness band so it never goes muddy, grey or neon and
 * reads on both themes. Returns a CSS rgb() string.
 */
export function trackTint(rgb: readonly number[], brand: readonly number[] = BRAND): string {
	const [R, G, B] = rgb.map((v, i) => (v * 0.5 + brand[i] * 0.5) / 255);
	// RGB -> HSL
	const max = Math.max(R, G, B);
	const min = Math.min(R, G, B);
	const l = (max + min) / 2;
	const d = max - min;
	let h = 0;
	if (d) h = 60 * (max === R ? ((G - B) / d + 6) % 6 : max === G ? (B - R) / d + 2 : (R - G) / d + 4);
	const s = d ? d / (1 - Math.abs(2 * l - 1)) : 0;
	const S = Math.min(0.9, Math.max(0.55, s));
	const L = Math.min(0.62, Math.max(0.48, l));
	// HSL -> RGB
	const f = (n: number) => {
		const k = (n + h / 30) % 12;
		return Math.round(255 * (L - S * Math.min(L, 1 - L) * Math.max(-1, Math.min(k - 3, 9 - k, 1))));
	};
	return `rgb(${f(0)} ${f(8)} ${f(4)})`;
}
