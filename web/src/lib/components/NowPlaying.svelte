<script lang="ts">
	import { onMount } from 'svelte';
	import { Slider } from 'bits-ui';
	import type { Station } from '#lib/station.svelte.ts';
	import { dur, fmtTime } from '#lib/util.ts';
	import Art from './Art.svelte';
	import { MicVocal, Pause, Play, Volume2 } from '@lucide/svelte';
	import { store, ui } from '#lib/ui.svelte.ts';
	import Avatar from './Avatar.svelte';
	import Lyrics from './Lyrics.svelte';
	import ReactionBar from './ReactionBar.svelte';
	import ReactionFloats from './ReactionFloats.svelte';
	import { ModeSwitch, Visualizer, startAnalysis, stopAnalysis, viz } from '#lib/visualizer/index.ts';
	import Berry from './Berry.svelte';
	import HoldSkip from './HoldSkip.svelte';
	import ComboMeter from './ComboMeter.svelte';
	import { smearAnimate } from '#lib/motion/motion.ts';

	let { station }: { station: Station } = $props();

	$effect(() => {
		startAnalysis(station.player.analyser);
		return stopAnalysis;
	});

	const s = $derived(station.state);
	const cur = $derived(s?.current ?? null);
	const durationMs = $derived(cur?.track.duration_ms ?? 0);

	// Live position, advanced every animation frame from the shared timeline.
	let pos = $state(0);
	onMount(() => {
		let raf = 0;
		const tick = () => {
			pos = station.positionMs();
			raf = requestAnimationFrame(tick);
		};
		raf = requestAnimationFrame(tick);
		return () => cancelAnimationFrame(raf);
	});
	const shown = $derived(Math.min(pos, durationMs));
	const frac = $derived(durationMs ? shown / durationMs : 0);

	let volume = $state(readVolume());
	function readVolume() {
		const v = Number(localStorage.getItem('cloudberry.volume'));
		return Number.isFinite(v) && localStorage.getItem('cloudberry.volume') !== null ? v : 0.8;
	}
	$effect(() => {
		station.player.setVolume(volume * volume); // perceptual-ish curve
		localStorage.setItem('cloudberry.volume', String(volume));
	});

	function seekFromPointer(e: MouseEvent & { currentTarget: HTMLElement }) {
		if (!durationMs) return;
		const r = e.currentTarget.getBoundingClientRect();
		const f = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
		station.send({ type: 'seek', position_ms: f * durationMs });
	}
	function seekKey(e: KeyboardEvent) {
		const d = e.key === 'ArrowRight' ? 5000 : e.key === 'ArrowLeft' ? -5000 : 0;
		if (!d || !durationMs) return;
		e.preventDefault();
		station.send({ type: 'seek', position_ms: Math.min(durationMs - 1000, Math.max(0, shown + d)) });
	}

	// Art swap: the new cover flings in from the right with smear (not on first paint).
	let ready = false;
	onMount(() => (ready = true));
	function artIn(el: HTMLElement) {
		if (ready) smearAnimate(el, { x: [140, 0], rotate: [9, 0], scale: [0.86, 1], opacity: [0, 1], duration: 620, delay: 90 });
	}
	function artOut(node: Element) {
		return {
			duration: dur(240),
			css: (t: number) => `opacity:${t};translate:${(t - 1) * 90}px 0;rotate:${(t - 1) * 6}deg`
		};
	}

	function trackIn(node: Element) {
		return {
			duration: dur(520),
			delay: dur(80),
			css: (t: number) => {
				const e = 1 - Math.pow(1 - t, 4);
				return `opacity:${e};transform:translateY(${(1 - e) * 14}px) scale(${0.97 + 0.03 * e})`;
			}
		};
	}
	function trackOut(node: Element) {
		return {
			duration: dur(260),
			css: (t: number) => `opacity:${t};transform:translateY(${(t - 1) * 10}px) scale(${0.98 + 0.02 * t})`
		};
	}
</script>

<section class="card now" aria-label="Now playing">
	<div class="stage" class:halo={viz.mode === 'halo'}>
		<div class="glow" class:on={s?.playing} aria-hidden="true"></div>
		{#if viz.mode !== 'bars'}<Visualizer mode={viz.mode} />{/if}
		<div class="stack">
			{#key cur?.queue_id}
				<div class="swap" use:artIn out:artOut>
					<div class="art-wrap">
						{#if cur}
							<Art track={cur.track} size="100%" />
						{:else}
							<div class="idle-art"><Berry bead={44} opacity={0.5} /></div>
						{/if}
					</div>
				</div>
			{/key}
		</div>
		<ReactionFloats {station} />
		<ComboMeter {station} />
	</div>

	<div class="stack meta">
		{#key cur?.queue_id}
			<div in:trackIn out:trackOut>
				{#if cur}
					<h1 class="title" id="np-title" title={cur.track.title}>{cur.track.title}</h1>
					<p class="by">added by <span class="who"><Avatar name={cur.added_by} size={22} />{cur.added_by}</span></p>
				{:else}
					<h1 class="title" id="np-title">The air is quiet</h1>
					<p class="by">
						{s?.queue.length ? 'Press play to start the next track.' : 'Paste a YouTube link to put something on.'}
					</p>
				{/if}
			</div>
		{/key}
	</div>

	<Lyrics {station} />

	<div
		class="progress"
		class:disabled={!cur}
		role="slider"
		tabindex={cur ? 0 : -1}
		aria-label="Seek"
		aria-valuemin={0}
		aria-valuemax={Math.round(durationMs / 1000)}
		aria-valuenow={Math.round(shown / 1000)}
		aria-valuetext="{fmtTime(shown)} of {fmtTime(durationMs)}"
		onclick={seekFromPointer}
		onkeydown={seekKey}
	>
		<div class="rail"><div class="fill" style:transform="scaleX({frac})"></div></div>
	</div>
	<div class="times num">
		<span>{fmtTime(shown)}</span>
		<span>{cur ? fmtTime(durationMs) : '0:00'}</span>
	</div>

	<div class="controls">
		<div class="vol">
			<Volume2 size={16} />
			<Slider.Root type="single" bind:value={volume} min={0} max={1} step={0.01} class="vol-slider" aria-label="Volume (only for you)">
				<span class="vol-rail"><Slider.Range class="vol-range" /></span>
				<Slider.Thumb index={0} class="vol-thumb" />
			</Slider.Root>
		</div>

		<button
			class="play"
			aria-label={s?.playing ? 'Pause for everyone' : 'Play for everyone'}
			disabled={!cur && !s?.queue.length}
			onclick={() => station.togglePlay()}
		>
			{#if s?.playing}<Pause size={30} fill="currentColor" strokeWidth={0} />{:else}<Play
					size={30}
					fill="currentColor"
					strokeWidth={0}
					class="play-glyph"
				/>{/if}
		</button>

		<div class="right">
			{#if ui.lines}
				<button
					class="icon-btn"
					class:on={ui.lyrics}
					aria-label="Lyrics"
					aria-pressed={ui.lyrics}
					title={ui.lyrics ? 'Hide lyrics' : 'Show lyrics'}
					onclick={() => store('cloudberry.lyrics', (ui.lyrics = !ui.lyrics))}
				>
					<MicVocal size={18} />
				</button>
			{/if}
			<HoldSkip {station} disabled={!cur} />
		</div>
	</div>

	<ReactionBar {station} />

	{#if viz.mode === 'bars'}<Visualizer mode="bars" />{/if}
	<ModeSwitch />
</section>

<style>
	.now {
		padding: 28px;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.stage {
		position: relative;
		width: min(100%, 340px);
		margin: 8px auto 24px;
	}
	/* A flat tinted disc behind the art that swells with the music: a shape, not a glow. */
	.glow {
		position: absolute;
		inset: -9%;
		border-radius: 50%;
		background: var(--track-tint);
		opacity: 0.28;
		transform: scale(calc(0.9 + var(--energy) * 0.35));
		will-change: transform;
		pointer-events: none;
	}
	.halo .glow {
		display: none; /* Halo draws its own disc */
	}
	.glow:not(.on) {
		opacity: 0.16;
		transform: scale(0.9);
		transition:
			opacity 1.2s var(--ease-out),
			transform 1.2s var(--ease-out);
	}

	/* Visualizer: art breathes on beats; Halo turns it into a disc inside the ring. */
	.stage > .stack {
		scale: var(--viz-pulse, 1);
	}
	.art-wrap {
		transition:
			border-radius 480ms var(--ease-in-out),
			scale 480ms var(--ease-in-out);
	}
	.halo .art-wrap {
		border-radius: 50%;
		scale: 0.66; /* = 2 * ART in Halo.svelte */
	}

	/* Keyed children overlap so old and new cross-fade in place. */
	.stack {
		display: grid;
	}
	.stack > :global(*) {
		grid-area: 1 / 1;
		min-width: 0;
	}
	.art-wrap {
		position: relative;
		aspect-ratio: 1;
		border-radius: 22px;
		overflow: hidden;
		/* Sticker: die-cut outline + a small drop. */
		box-shadow: var(--die-cut), var(--shadow-pop);
	}
	.art-wrap :global(.art) {
		border-radius: 0;
	}

	.idle-art {
		width: 100%;
		height: 100%;
		background: var(--surface-2);
		display: grid;
		place-items: center;
	}
	.meta {
		text-align: center;
		min-height: 6.4rem;
	}
	/* The one focal point: display-size, confident. */
	.title {
		font-size: clamp(1.9rem, 1.3rem + 1.4vw, 2.6rem);
		font-weight: 700;
		line-height: 1.06;
		letter-spacing: -0.03em;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		text-wrap: balance;
		padding-bottom: 0.06em; /* descenders under line-clamp */
	}
	.by {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		margin: 10px 0 0;
		color: var(--ink-2);
		font-size: 0.9rem;
	}
	.who {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		padding: 3px 12px 3px 3px;
		border-radius: 999px;
		background: var(--surface-2);
		color: var(--ink);
		font-weight: 600;
	}

	/* Progress: raw rAF scaleX, never eased (sync is sacred). */
	.progress {
		margin-top: 18px;
		padding: 10px 0;
		cursor: pointer;
	}
	.progress.disabled {
		cursor: default;
	}
	.rail {
		height: 6px;
		border-radius: 99px;
		background: var(--surface-2);
		overflow: hidden;
		transition: height 160ms var(--ease-out);
	}
	@media (hover: hover) {
		.progress:not(.disabled):hover .rail {
			height: 10px;
		}
	}
	.fill {
		height: 100%;
		background: var(--accent-line);
		transform-origin: left;
		border-radius: inherit;
	}
	.times {
		display: flex;
		justify-content: space-between;
		font-size: 0.86rem;
		font-weight: 600;
		color: var(--ink-2);
	}

	.controls {
		display: grid;
		grid-template-columns: 1fr auto 1fr;
		align-items: center;
		margin: 16px 0 20px;
	}
	.play {
		width: 76px;
		height: 76px;
		border-radius: 50%;
		display: grid;
		place-items: center;
		background: var(--accent);
		color: var(--on-accent);
		box-shadow: var(--lip), var(--die-cut), var(--shadow);
		transition:
			background-color 160ms var(--ease-out),
			box-shadow 200ms var(--ease-out);
	}
	@media (hover: hover) {
		.play:hover:not(:disabled) {
			background: var(--accent-hover);
		}
	}
	.play:active:not(:disabled) {
		box-shadow: inset 0 3px 0 rgb(110 45 0 / 0.3), var(--die-cut);
	}
	.play:disabled {
		opacity: 0.4;
		cursor: not-allowed;
		box-shadow: none;
	}
	.right {
		display: flex;
		justify-content: flex-end;
		align-items: center;
		gap: 8px;
	}
	/* The triangle's visual centre sits left of its box. */
	.play :global(.play-glyph) {
		translate: 2px 0;
	}

	.vol {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--ink-2);
		max-width: 130px;
	}
	.vol :global(.vol-slider) {
		position: relative;
		flex: 1;
		display: flex;
		align-items: center;
		height: 32px;
		touch-action: none;
		cursor: pointer;
	}
	.vol-rail {
		position: relative;
		flex: 1;
		height: 5px;
		border-radius: 99px;
		background: var(--surface-2);
		overflow: hidden;
	}
	.vol :global(.vol-range) {
		position: absolute;
		height: 100%;
		background: var(--ink-2);
	}
	.vol :global(.vol-thumb) {
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: var(--ink);
		border: 3px solid var(--surface);
		box-shadow: 0 1px 4px rgb(0 0 0 / 0.25);
		transition: scale 120ms var(--ease-out);
	}
	.vol :global(.vol-thumb:active) {
		scale: 1.15;
	}

	/* Phone: compact now-playing (art beside the title) and the transport in the thumb zone. */
	@media (max-width: 560px) {
		.now {
			padding: 16px;
			display: grid;
			grid-template-columns: 96px minmax(0, 1fr);
			column-gap: 16px;
			align-items: center;
		}
		.now > :global(*) {
			grid-column: 1 / -1;
		}
		.stage {
			grid-column: 1;
			width: 96px;
			margin: 0;
		}
		.stage :global(.berry) {
			scale: 0.5;
		}
		.meta {
			grid-column: 2;
			text-align: left;
			min-height: 0;
		}
		.title {
			font-size: 1.35rem;
			line-height: 1.12;
		}
		.by {
			justify-content: flex-start;
			font-size: 0.82rem;
			margin-top: 6px;
		}
		.progress {
			margin-top: 10px;
		}
		.times {
			margin-bottom: 16px;
		}
		.vol {
			visibility: hidden; /* phones have hardware volume; keep the grid column */
		}
		.controls {
			position: fixed;
			z-index: 40;
			left: 0;
			right: 0;
			bottom: 0;
			margin: 0;
			padding: 10px 20px max(10px, env(safe-area-inset-bottom));
			background: var(--surface);
			border-top: 3px solid var(--line);
		}
		.play {
			width: 64px;
			height: 64px;
		}
	}
</style>
