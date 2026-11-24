<script lang="ts">
	import { untrack } from 'svelte';
	import { FastAverageColor } from 'fast-average-color';
	import type { Station } from '#lib/station.svelte.ts';
	import { ACCENTS, toggleMute, ui } from '#lib/ui.svelte.ts';
	import { closeMini, miniSupported, openMini } from '#lib/mini.svelte.ts';
	import { tinted } from '#lib/logo.ts';
	import { trackTint } from '#lib/util.ts';

	/**
	 * Headless: everything that reflects the station into the browser around the page:
	 * tab title, Media Session, mini player, per-track tint, "your track started" notification.
	 */
	let { station }: { station: Station } = $props();

	const s = $derived(station.state);
	const cur = $derived(s?.current ?? null);
	const playing = $derived(!!s?.playing && !!cur);
	const title = $derived(cur?.track.title ?? null);
	const trackId = $derived(cur?.track.id ?? null);
	const queueId = $derived(cur?.queue_id ?? null);
	const thumb = $derived(!!cur?.track.thumbnail);
	const addedBy = $derived(cur?.added_by ?? '');
	// Effects below read these primitives, not `cur`: a fresh state object arrives on
	// every change, and we only want to react when the track itself changes.

	// --- tab title ---
	$effect(() => {
		document.title = playing && title ? `▶ ${title} · Cloudberry` : 'Cloudberry';
		return () => (document.title = 'Cloudberry');
	});

	// --- Media Session ---
	const ms = typeof navigator !== 'undefined' && 'mediaSession' in navigator ? navigator.mediaSession : null;
	$effect(() => {
		if (!ms) return;
		ms.metadata =
			trackId && title
				? new MediaMetadata({
						title,
						artist: `added by ${addedBy}`,
						album: 'Cloudberry',
						artwork: thumb ? [{ src: `/art/${encodeURIComponent(trackId)}.jpg`, type: 'image/jpeg' }] : []
					})
				: null;
	});
	$effect(() => {
		if (!ms) return;
		// "Paused" while muted too, so the OS key / headset offers play (= unmute) back.
		ms.playbackState = trackId ? (playing && !ui.muted ? 'playing' : 'paused') : 'none';
		// Re-anchor the OS scrubber whenever the timeline changes (play/pause/seek/track).
		void s?.anchor_server_ms;
		void s?.position_ms;
		const d = (s?.current?.track.duration_ms ?? 0) / 1000;
		try {
			if (d) ms.setPositionState({ duration: d, position: Math.min(d, station.positionMs() / 1000), playbackRate: 1 });
			else ms.setPositionState();
		} catch {
			/* older browsers / bad values: cosmetic only */
		}
	});
	$effect(() => {
		if (!ms) return;
		// Play / pause from the OS are yours only: taking headphones off (ear detection, Bluetooth
		// disconnect) sends "pause", and that must not stop the station for the whole room.
		const handlers: [MediaSessionAction | 'enterpictureinpicture', () => void][] = [
			['play', () => ui.muted && toggleMute()],
			['pause', () => ui.muted || toggleMute()],
			['nexttrack', () => station.skip()]
		];
		// Leaving the tab while it plays: Chrome opens the mini player by itself (no gesture needed).
		if (miniSupported) handlers.push(['enterpictureinpicture', () => void openMini(station)]);
		for (const [a, h] of handlers) {
			try {
				ms.setActionHandler(a as MediaSessionAction, h);
			} catch {
				/* unsupported action */
			}
		}
		return () => {
			for (const [a] of handlers) {
				try {
					ms.setActionHandler(a as MediaSessionAction, null);
				} catch {
					/* unsupported action */
				}
			}
		};
	});

	// --- mini player goes away with the station (name change, leaving) ---
	$effect(() => closeMini);

	// --- logo mark + favicons follow your accent (orange pixels recoloured, see logo.ts) ---
	const icons =
		typeof document === 'undefined'
			? []
			: [...document.querySelectorAll<HTMLLinkElement>('link[rel="icon"], link[rel="apple-touch-icon"]')].map((l) => ({
					l,
					src: l.getAttribute('href')!
				}));
	$effect(() => {
		const hex = ACCENTS.find((a) => a.id === ui.accent)!.swatch;
		const berry = ui.accent === 'berry';
		let live = true;
		const logo = berry ? Promise.resolve('/logo-mark.png') : tinted('/logo-mark.png', hex);
		logo.then((u) => live && (ui.logo = u)).catch(() => live && (ui.logo = '/logo-mark.png'));
		for (const { l, src } of icons) {
			if (berry) l.href = src;
			else tinted(src, hex).then((u) => live && (l.href = u)).catch(() => {});
		}
		return () => (live = false);
	});

	// --- per-track tint (--track-tint on :root, animated via @property in app.css) ---
	const fac = new FastAverageColor();
	$effect(() => {
		const id = trackId;
		const hex = ACCENTS.find((a) => a.id === ui.accent)!.swatch;
		const brand = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
		const root = document.documentElement;
		if (!id || !thumb) {
			root.style.removeProperty('--track-tint');
			return;
		}
		let live = true;
		fac
			.getColorAsync(`/art/${encodeURIComponent(id)}.jpg`, { algorithm: 'sqrt', mode: 'speed' })
			.then((c) => live && root.style.setProperty('--track-tint', trackTint(c.value, brand)))
			.catch(() => live && root.style.removeProperty('--track-tint'));
		return () => (live = false);
	});

	// --- "your track started" notification ---
	let lastQueueId: number | null | undefined; // undefined = first state not seen yet
	$effect(() => {
		const q = queueId;
		if (!s) return; // not connected yet
		const started = lastQueueId !== undefined && q !== lastQueueId; // not what was playing when we joined
		lastQueueId = q;
		if (!started || q === null || !untrack(() => ui.notify) || addedBy !== station.name) return;
		if (typeof Notification === 'undefined' || Notification.permission !== 'granted') return;
		if (document.visibilityState === 'visible' && document.hasFocus()) return; // you can see it already
		const n = new Notification('Your track is on', {
			body: untrack(() => title) ?? '',
			icon: thumb ? `/art/${encodeURIComponent(trackId!)}.jpg` : '/icon-192.png',
			tag: 'cloudberry-track',
			silent: true // the music is the sound
		});
		n.onclick = () => {
			window.focus();
			n.close();
		};
	});
</script>
