<script lang="ts">
	import { onMount } from 'svelte';
	import type { Station } from '#lib/station.svelte.ts';
	import { fmtTime } from '#lib/util.ts';
	import { toggleMute, ui } from '#lib/ui.svelte.ts';
	import Icon from '#lib/icons/Icon.svelte';
	import Art from './Art.svelte';
	import Berry from './Berry.svelte';
	import HoldSkip from './HoldSkip.svelte';

	/**
	 * The mini player's contents (mounted into the PiP window by mini.svelte.ts). Same transport as
	 * the deck: play/pause and hold-skip are for everyone, mute is for you only.
	 */
	let { station, win }: { station: Station; win: Window } = $props();

	const s = $derived(station.state);
	const cur = $derived(s?.current ?? null);
	const durationMs = $derived(cur?.track.duration_ms ?? 0);

	const air = $derived(
		station.status === 'connecting'
			? { cls: 'blink', text: 'Tuning in' }
			: station.status === 'reconnecting'
				? { cls: 'blink bad', text: 'Reconnecting' }
				: s?.playing && cur
					? ui.muted
						? { cls: '', text: 'Muted for you' }
						: { cls: 'on', text: 'On air' }
					: { cls: '', text: cur ? 'Paused' : 'Idle' }
	);

	// This window's own frames: the page's are throttled while its tab is hidden.
	let pos = $state(0);
	onMount(() => {
		let raf = 0;
		const tick = () => {
			pos = station.positionMs();
			raf = win.requestAnimationFrame(tick);
		};
		raf = win.requestAnimationFrame(tick);
		const key = (e: KeyboardEvent) => {
			if ((e.key === 'm' || e.key === 'M') && !e.ctrlKey && !e.metaKey && !e.altKey) toggleMute();
		};
		win.addEventListener('keydown', key);
		return () => {
			win.cancelAnimationFrame(raf);
			win.removeEventListener('keydown', key);
		};
	});
	const shown = $derived(Math.min(pos, durationMs));
	const frac = $derived(durationMs ? shown / durationMs : 0);
</script>

<main class="mini">
	<div class="art">
		{#if cur}
			<Art track={cur.track} size="100%" />
		{:else}
			<div class="idle"><Berry bead="9cqi" opacity={0.5} /></div>
		{/if}
	</div>

	<div class="info">
		<span class="label status"><span class="led {air.cls}"></span>{air.text}</span>
		<h1 class="title" title={cur?.track.title}>{cur?.track.title ?? 'Dead air'}</h1>
		<p class="by">
			{#if cur}<span class="label">Added by</span> {cur.added_by}{:else}Nothing playing{/if}
		</p>
	</div>

	<div class="controls">
		<button
			class="play"
			aria-label={s?.playing ? 'Pause for everyone' : 'Play for everyone'}
			title={s?.playing ? 'Pause for everyone' : 'Play for everyone'}
			disabled={!cur && !s?.queue.length}
			onclick={() => station.togglePlay()}
		>
			<Icon name={s?.playing ? 'pause' : 'play'} size={20} />
		</button>
		<HoldSkip {station} {win} disabled={!cur} />
		<button
			class="key mute"
			class:lit={ui.muted}
			aria-label="Mute (only for you)"
			aria-pressed={ui.muted}
			title={ui.muted ? 'Unmute (M)' : 'Mute for you only (M)'}
			onclick={toggleMute}
		>
			<Icon name={ui.muted ? 'volume-off' : 'volume'} size={18} />
		</button>
		<span class="time num" aria-hidden="true">{fmtTime(shown)} / {cur ? fmtTime(durationMs) : '0:00'}</span>
	</div>

	<div
		class="progress"
		role="progressbar"
		aria-label="Position"
		aria-valuemin={0}
		aria-valuemax={Math.round(durationMs / 1000)}
		aria-valuenow={Math.round(shown / 1000)}
		aria-valuetext="{fmtTime(shown)} of {fmtTime(durationMs)}"
	>
		<!-- Raw rAF transform, never eased (sync is sacred). -->
		<div class="fill" style:transform="scaleX({frac})"></div>
	</div>
</main>

<style>
	/* Only the PiP document is marked (this CSS also loads in the page). */
	:global(html[data-mini]),
	:global(html[data-mini] body) {
		height: 100%;
		overflow: hidden;
		background: var(--surface);
	}

	/* Compact strip: art left, track + transport right, progress along the bottom edge. */
	.mini {
		height: 100%;
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		grid-template-rows: minmax(0, 1fr) auto auto;
		grid-template-areas:
			'art info'
			'art controls'
			'progress progress';
		column-gap: 12px;
		row-gap: 8px;
		padding: 12px 12px 0;
	}
	.art {
		grid-area: art;
		height: calc(100vh - 12px - 3px - 12px);
		max-height: 132px;
		aspect-ratio: 1;
		align-self: start;
	}
	.idle {
		width: 100%;
		height: 100%;
		display: grid;
		place-items: center;
		border-radius: var(--r-sm);
		background: var(--surface-2);
		box-shadow: inset 0 0 0 1px var(--line);
		container-type: inline-size;
	}

	.info {
		grid-area: info;
		min-width: 0;
		display: grid;
		align-content: start;
		gap: 4px;
	}
	.status {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}
	.title {
		font-size: 1rem;
		line-height: 1.2;
		letter-spacing: -0.015em;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.by {
		margin: 0;
		font-size: 0.8125rem;
		color: var(--ink-2);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.by .label {
		font-size: 0.625rem;
	}

	.controls {
		grid-area: controls;
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}
	.play {
		width: 56px;
		height: 36px;
		border-radius: var(--r-md);
		display: grid;
		place-items: center;
		background: var(--accent);
		color: var(--on-accent);
		box-shadow: 0 2px 0 color-mix(in oklab, var(--accent) 55%, #000);
		transition:
			background-color 120ms var(--ease-out),
			translate 60ms linear,
			box-shadow 60ms linear;
	}
	@media (hover: hover) {
		.play:hover:not(:disabled) {
			background: var(--accent-hover);
		}
	}
	.play:active:not(:disabled) {
		translate: 0 2px;
		box-shadow: 0 0 0 color-mix(in oklab, var(--accent) 55%, #000);
	}
	.play:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.controls :global(.hold),
	.mute {
		width: 36px;
		height: 36px;
	}
	.mute.lit::after {
		content: '';
		position: absolute;
		top: 4px;
		right: 4px;
		width: 4px;
		height: 4px;
		border-radius: 50%;
		background: var(--led);
	}
	.time {
		margin-left: auto;
		font-family: var(--font-mono);
		font-size: 0.6875rem;
		color: var(--ink-2);
		white-space: nowrap;
	}

	.progress {
		grid-area: progress;
		margin: 0 -12px;
		height: 3px;
		background: var(--line);
		overflow: hidden;
	}
	.fill {
		height: 100%;
		background: var(--accent);
		transform-origin: left;
	}

	/* Resized tall (drag the corner): Spotify-style, the cover takes the top of the window. */
	@media (min-height: 300px) {
		.mini {
			grid-template-columns: minmax(0, 1fr);
			grid-template-rows: minmax(0, 1fr) auto auto auto;
			grid-template-areas: 'art' 'info' 'controls' 'progress';
			row-gap: 12px;
		}
		.art {
			/* Square, as big as the window leaves room for under it (info + keys + progress ≈ 168px). */
			height: auto;
			max-height: none;
			width: min(100%, calc(100vh - 168px));
			justify-self: center;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.play {
			transition: none;
		}
	}
</style>
