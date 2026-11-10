/**
 * App-wide UI state shared between the header, shortcuts, chat and now-playing.
 * Module-level so components don't have to thread it through props.
 */
import type { LrcLine } from './lyrics';

function stored(key: string, fallback: boolean): boolean {
	try {
		const v = localStorage.getItem(key);
		return v === null ? fallback : v === '1';
	} catch {
		return fallback;
	}
}
export function store(key: string, v: boolean) {
	try {
		localStorage.setItem(key, v ? '1' : '0');
	} catch {
		/* private mode */
	}
}

/** Accent presets (ramps live in app.css under :root[data-accent]); the first is the default. */
export const ACCENTS = [
	{ id: 'berry', label: 'Cloudberry', swatch: '#f28322' },
	{ id: 'pink', label: 'Pink', swatch: '#f2609f' },
	{ id: 'lime', label: 'Lime', swatch: '#9ccc1f' },
	{ id: 'cyan', label: 'Cyan', swatch: '#1fb8d6' },
	{ id: 'violet', label: 'Violet', swatch: '#9b7bff' },
	{ id: 'gold', label: 'Gold', swatch: '#f2c21b' }
] as const;
export type AccentId = (typeof ACCENTS)[number]['id'];

function storedAccent(): AccentId {
	try {
		const v = localStorage.getItem('cloudberry.accent');
		return ACCENTS.find((a) => a.id === v)?.id ?? 'berry';
	} catch {
		return 'berry';
	}
}

export const WIDE_QUERY = '(min-width: 1180px)';
export const PHONE_QUERY = '(max-width: 560px)';

export const ui = $state({
	/** Wide enough for the docked side chat (else it is a drawer). */
	wide: typeof matchMedia !== 'undefined' && matchMedia(WIDE_QUERY).matches,
	/** Phone width (compact now-playing, bottom transport). */
	phone: typeof matchMedia !== 'undefined' && matchMedia(PHONE_QUERY).matches,
	/** Docked chat panel open (wide screens; remembered). */
	dock: stored('cloudberry.chat', true),
	/** Chat drawer open (narrow screens; transient). */
	drawer: false,
	help: false,
	/** Show synced lyrics when the current track has them (remembered). */
	lyrics: stored('cloudberry.lyrics', true),
	/** Silenced for you only; the station keeps playing for everyone (remembered). */
	muted: stored('cloudberry.muted', false),
	/** Desktop notification when a track you added starts (remembered). */
	notify: stored('cloudberry.notify', false),
	/** Your accent colour, applies to you only (remembered; app.html applies it before paint). */
	accent: storedAccent(),
	/** Logo mark URL tinted to the accent; null while a non-default tint is being drawn (no orange flash). */
	logo: (storedAccent() === 'berry' ? '/logo-mark.png' : null) as string | null,
	/** Synced lyrics for the current track, null when none (also for lean-back mode). */
	lines: null as LrcLine[] | null,
	/** Highest chat message id the user has had on screen. */
	seenId: 0,
	/** DJ of the day (crown on their header avatar), from station state. */
	dj: null as string | null
});

export function setAccent(id: AccentId) {
	ui.accent = id;
	if (id === 'berry') delete document.documentElement.dataset.accent;
	else document.documentElement.dataset.accent = id;
	try {
		localStorage.setItem('cloudberry.accent', id);
	} catch {
		/* private mode */
	}
}

export const chatVisible =() => (ui.wide ? ui.dock : ui.drawer);

/** Open the chat (dock or drawer) and focus its input. */
export function focusChat() {
	if (ui.wide) ui.dock = true;
	else ui.drawer = true;
	const focus = () => document.getElementById('chat-input')?.focus();
	// Focus now if it's already on screen (so fast typing lands in it), else once it mounts.
	if (!document.getElementById('chat-input')) requestAnimationFrame(() => requestAnimationFrame(focus));
	else focus();
}

export function toggleMute() {
	store('cloudberry.muted', (ui.muted = !ui.muted));
}

export function toggleChat() {
	if (ui.wide) store('cloudberry.chat', (ui.dock = !ui.dock));
	else ui.drawer = !ui.drawer;
}
