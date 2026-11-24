/**
 * Mini player: a small always-on-top window with the now-playing card, like Spotify's web
 * miniplayer. Document Picture-in-Picture (Chromium desktop only). Chrome opens it by itself when
 * you switch away from the playing tab (Integrations registers the "enterpictureinpicture" Media
 * Session action; Chrome 134+), and the header button opens it on demand.
 */
import { mount, unmount } from 'svelte';
import MiniPlayer from './components/MiniPlayer.svelte';
import type { Station } from './station.svelte';

interface DocumentPictureInPicture extends EventTarget {
	requestWindow(options?: { width?: number; height?: number }): Promise<Window>;
	readonly window: Window | null;
}
declare global {
	interface Window {
		documentPictureInPicture?: DocumentPictureInPicture;
	}
}

export const miniSupported = typeof window !== 'undefined' && !!window.documentPictureInPicture;

const WIDTH = 380;
const HEIGHT = 156;

/** Whether the mini window is open (for the header key). */
export const mini = $state({ open: false });

let win: Window | null = null;
let opening = false;

export async function openMini(station: Station) {
	const pip = window.documentPictureInPicture;
	if (!pip || win || opening) return;
	opening = true;
	let w: Window;
	try {
		w = await pip.requestWindow({ width: WIDTH, height: HEIGHT });
	} catch {
		return; // no user gesture / dismissed / already open from elsewhere
	} finally {
		opening = false;
	}
	win = w;
	const doc = w.document;
	doc.title = 'Cloudberry';
	doc.documentElement.dataset.mini = '';
	copyStyles(doc);
	const stopMirror = mirrorRoot(doc.documentElement);
	const app = mount(MiniPlayer, { target: doc.body, props: { station, win: w } });
	mini.open = true;
	// Fires on our close(), the window's own close button and Chrome's "back to tab".
	w.addEventListener(
		'pagehide',
		() => {
			stopMirror();
			void unmount(app);
			if (win === w) win = null;
			mini.open = false;
		},
		{ once: true }
	);
}

export function closeMini() {
	win?.close();
}

export function toggleMini(station: Station) {
	if (win) closeMini();
	else void openMini(station);
}

/** The PiP document starts empty: give it the page's base URL (art, fonts) and every stylesheet. */
function copyStyles(to: Document) {
	const base = to.createElement('base');
	base.href = document.baseURI;
	to.head.append(base);
	for (const sheet of document.styleSheets) {
		if (sheet.href) {
			// Built CSS: link the same file (cached), so its relative font URLs keep resolving.
			const link = to.createElement('link');
			link.rel = 'stylesheet';
			link.href = sheet.href;
			to.head.append(link);
			continue;
		}
		try {
			const style = to.createElement('style');
			style.textContent = [...sheet.cssRules].map((r) => r.cssText).join('\n');
			to.head.append(style);
		} catch {
			/* unreadable sheet: skip */
		}
	}
}

/** Accent (data-accent) and per-track tint (--track-tint in style) live on the page's <html>. */
function mirrorRoot(to: HTMLElement): () => void {
	const from = document.documentElement;
	const sync = () => {
		for (const a of ['data-accent', 'style']) {
			const v = from.getAttribute(a);
			if (v === null) to.removeAttribute(a);
			else to.setAttribute(a, v);
		}
	};
	sync();
	const obs = new MutationObserver(sync);
	obs.observe(from, { attributes: true, attributeFilter: ['data-accent', 'style'] });
	return () => obs.disconnect();
}
