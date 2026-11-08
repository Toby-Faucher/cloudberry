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
	/** Desktop notification when a track you added starts (remembered). */
	notify: stored('cloudberry.notify', false),
	/** Synced lyrics for the current track, null when none (also for lean-back mode). */
	lines: null as LrcLine[] | null,
	/** Highest chat message id the user has had on screen. */
	seenId: 0,
	/** DJ of the day (crown on their header avatar), from station state. */
	dj: null as string | null
});

export const chatVisible = () => (ui.wide ? ui.dock : ui.drawer);

/** Open the chat (dock or drawer) and focus its input. */
export function focusChat() {
	if (ui.wide) ui.dock = true;
	else ui.drawer = true;
	const focus = () => document.getElementById('chat-input')?.focus();
	// Focus now if it's already on screen (so fast typing lands in it), else once it mounts.
	if (!document.getElementById('chat-input')) requestAnimationFrame(() => requestAnimationFrame(focus));
	else focus();
}

export function toggleChat() {
	if (ui.wide) store('cloudberry.chat', (ui.dock = !ui.dock));
	else ui.drawer = !ui.drawer;
}
