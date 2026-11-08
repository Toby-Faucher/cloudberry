<script lang="ts">
	import { untrack } from 'svelte';
	import { FastAverageColor } from 'fast-average-color';
	import type { Station } from '#lib/station.svelte.ts';
	import { ui } from '#lib/ui.svelte.ts';
	import { trackTint } from '#lib/util.ts';

	/**
	 * Headless: everything that reflects the station into the browser around the page:
	 * tab title, Media Session, per-track tint, "your track started" notification.
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
		ms.playbackState = trackId ? (playing ? 'playing' : 'paused') : 'none';
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
		const handlers: [MediaSessionAction, () => void][] = [
			['play', () => station.state?.playing || station.togglePlay()],
			['pause', () => station.state?.playing && station.togglePlay()],
			['nexttrack', () => station.skip()]
		];
		for (const [a, h] of handlers) {
			try {
				ms.setActionHandler(a, h);
			} catch {
				/* unsupported action */
			}
		}
		return () => handlers.forEach(([a]) => ms.setActionHandler(a, null));
	});

	// --- per-track tint (--track-tint on :root, animated via @property in app.css) ---
	const fac = new FastAverageColor();
	$effect(() => {
		const id = trackId;
		const root = document.documentElement;
		if (!id || !thumb) {
			root.style.removeProperty('--track-tint');
			return;
		}
		let live = true;
		fac
			.getColorAsync(`/art/${encodeURIComponent(id)}.jpg`, { algorithm: 'sqrt', mode: 'speed' })
			.then((c) => live && root.style.setProperty('--track-tint', trackTint(c.value)))
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
