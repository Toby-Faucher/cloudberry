<script lang="ts">
	import { onMount } from 'svelte';
	import { Slider } from 'bits-ui';
	import type { Station } from '#lib/station.svelte.ts';
	import { dur, fmtTime } from '#lib/util.ts';
	import Art from './Art.svelte';
	import Icon from '#lib/icons/Icon.svelte';
	import { store, ui } from '#lib/ui.svelte.ts';
	import Avatar from './Avatar.svelte';
	import Lyrics from './Lyrics.svelte';
	import ReactionBar from './ReactionBar.svelte';
	import ReactionFloats from './ReactionFloats.svelte';
	import { Visualizer, startAnalysis, stopAnalysis, viz } from '#lib/visualizer/index.ts';
	import { MODES, setLean, setMode } from '#lib/visualizer/state.svelte.ts';
	import Berry from './Berry.svelte';
	import HoldSkip from './HoldSkip.svelte';
	import DotTime from './DotTime.svelte';
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

	// Connection + transport state, as the screen's status lamp.
	const air = $derived(
		station.status === 'connecting'
			? { cls: 'blink', text: 'Tuning in' }
			: station.status === 'reconnecting'
				? { cls: 'blink bad', text: 'Reconnecting' }
				: s?.playing && cur
					? { cls: 'on', text: 'On air' }
					: { cls: '', text: cur ? 'Paused' : 'Idle' }
	);

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

	// Art swap: the new cover slides in from the right with smear (not on first paint).
	let ready = false;
	onMount(() => (ready = true));
	function artIn(el: HTMLElement) {
		if (ready) smearAnimate(el, { x: [120, 0], opacity: [0, 1], duration: 380, delay: 60, ease: 'outQuart' });
	}
	function artOut(_n: Element) {
		return {
			duration: dur(180),
			css: (t: number) => `opacity:${t};translate:${(t - 1) * 80}px 0`
		};
	}

	function trackIn(_n: Element) {
		return {
			duration: dur(300),
			delay: dur(60),
			css: (t: number) => {
				const e = 1 - Math.pow(1 - t, 4);
				return `opacity:${e};transform:translateY(${(1 - e) * 8}px)`;
			}
		};
	}
	function trackOut(_n: Element) {
		return {
			duration: dur(160),
			css: (t: number) => `opacity:${t};transform:translateY(${(t - 1) * 6}px)`
		};
	}

	const MODE_LABEL: Record<string, string> = { halo: 'Halo', bloom: 'Bloom', bars: 'Bars' };
</script>

<!--
	Stacking (the Bloom bug): .now is its own stacking context. Visualizer layers sit at z -1 inside
	the screen, under the art (z 3); reaction floats (5) and the combo (6) sit above it.
-->
<section class="now" aria-label="Now playing">
	<div class="screen" class:halo={viz.mode === 'halo'}>
		<span class="status label" role="status"><span class="led {air.cls}"></span>{air.text}</span>
		<div class="stage">
			{#if viz.mode !== 'bars'}<Visualizer mode={viz.mode} />{/if}
			<div class="stack art-layer">
				{#key cur?.queue_id}
					<div class="swap" use:artIn out:artOut>
						<div class="art-wrap">
							{#if cur}
								<Art track={cur.track} size="100%" />
							{:else}
								<div class="idle-art"><Berry bead="9cqi" opacity={0.5} /></div>
							{/if}
						</div>
					</div>
				{/key}
			</div>
		</div>
		<ReactionFloats {station} />
		<ComboMeter {station} />
		<!-- Soft keys along the bottom of the screen: visualizer style + lean-back. -->
		<div class="modes">
			<div class="seg" role="group" aria-label="Visualizer style">
				{#each MODES as m (m)}
					<button class="seg-btn" aria-pressed={viz.mode === m} onclick={() => setMode(m)}>
						<span class="led" class:on={viz.mode === m}></span>{MODE_LABEL[m] ?? m}
					</button>
				{/each}
			</div>
			<button class="seg-btn lean" onclick={() => setLean(true)} aria-label="Lean back (F)" title="Lean back (F)">
				<Icon name="expand" size={16} />
			</button>
		</div>
	</div>

	<div class="meta stack">
		{#key cur?.queue_id}
			<div in:trackIn out:trackOut>
				{#if cur}
					<h1 class="title" id="np-title" title={cur.track.title}>{cur.track.title}</h1>
					<p class="by"><span class="label">Added by</span><span class="who"><Avatar name={cur.added_by} size={18} />{cur.added_by}</span></p>
				{:else}
					<h1 class="title" id="np-title">Dead air</h1>
					<p class="by plain">
						{s?.queue.length ? 'Hit play and the next one starts.' : 'Drop a YouTube link and get this going.'}
					</p>
				{/if}
			</div>
		{/key}
	</div>

	<Lyrics {station} />

	<div class="readout">
		<span class="elapsed" aria-hidden="true"><DotTime text={fmtTime(shown)} /></span>
		<span class="total num" aria-hidden="true">{cur ? fmtTime(durationMs) : '0:00'}</span>
	</div>
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
		<!-- Raw rAF transforms, never eased (sync is sacred). -->
		<div class="rail"><div class="fill" style:transform="scaleX({frac})"></div></div>
		<div class="head-track" style:transform="translateX({frac * 100}%)"><span class="head"></span></div>
	</div>

	<div class="controls">
		<div class="vol">
			<Slider.Root type="single" bind:value={volume} min={0} max={1} step={0.01} class="fader" aria-label="Volume (only for you)">
				<span class="fader-rail"><Slider.Range class="fader-range" /></span>
				<Slider.Thumb index={0} class="fader-cap" />
			</Slider.Root>
			<span class="label cap-label"><Icon name="volume" size={12} />Vol</span>
		</div>

		<div class="keys">
			<div class="slot">
				<button
					class="play"
					aria-label={s?.playing ? 'Pause for everyone' : 'Play for everyone'}
					disabled={!cur && !s?.queue.length}
					onclick={() => station.togglePlay()}
				>
					<Icon name={s?.playing ? 'pause' : 'play'} size={24} />
				</button>
				<span class="label cap-label" aria-hidden="true">{s?.playing ? 'Pause' : 'Play'}</span>
			</div>
			<div class="slot">
				<HoldSkip {station} disabled={!cur} />
				<span class="label cap-label" aria-hidden="true">Hold skip</span>
			</div>
			{#if ui.lines}
				<div class="slot">
					<button
						class="key sq"
						class:lit={ui.lyrics}
						aria-label="Lyrics"
						aria-pressed={ui.lyrics}
						title={ui.lyrics ? 'Hide lyrics' : 'Show lyrics'}
						onclick={() => store('cloudberry.lyrics', (ui.lyrics = !ui.lyrics))}
					>
						<Icon name="mic" size={20} />
					</button>
					<span class="label cap-label" aria-hidden="true">Lyrics</span>
				</div>
			{/if}
		</div>
	</div>

	<ReactionBar {station} />

	{#if viz.mode === 'bars'}<div class="bars"><Visualizer mode="bars" /></div>{/if}

</section>

<style>
	/* The deck: the one raised object on the page. Modules inside are split by hairlines. */
	.now {
		position: relative;
		isolation: isolate;
		display: flex;
		flex-direction: column;
		padding: 12px;
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: var(--r-lg);
	}
	.stack {
		display: grid;
	}
	.stack > :global(*) {
		grid-area: 1 / 1;
		min-width: 0;
	}

	/* ---------- the screen: a recessed square well; the art is the one image on the panel ---------- */
	.screen {
		position: relative;
		isolation: isolate; /* the visualizer's z -1 layers paint above the well, not under it */
		aspect-ratio: 1 / 0.9;
		border-radius: var(--r-md);
		background: var(--surface-2);
		box-shadow: inset 0 0 0 1px var(--line);
		overflow: hidden;
		display: grid;
		place-items: center;
		container-type: size; /* has a definite size from its aspect ratio; cq units size the art */

	}
	.status {
		position: absolute;
		z-index: 4;
		top: 12px;
		left: 12px;
		display: inline-flex;
		align-items: center;
		gap: 7px;
		padding: 5px 7px;
		border-radius: var(--r-sm);
		background: color-mix(in oklab, var(--surface) 88%, transparent);
		color: var(--ink);
		pointer-events: none;
	}
	.stage {
		position: relative; /* containing block for the visualizers; NOT a stacking context */
		width: min(64cqi, 66cqh);
		margin-top: -8cqh; /* optical centre above the soft-key strip */
	}
	.stage :global(canvas),
	.stage :global(.glow),
	.stage :global(.css) {
		z-index: -1;
	}
	.art-layer {
		position: relative;
		z-index: 3;
		scale: var(--viz-pulse, 1); /* art breathes on beats */
	}
	.art-wrap {
		position: relative;
		aspect-ratio: 1;
		border-radius: var(--r-sm);
		overflow: hidden;
		box-shadow: 0 0 0 1px rgb(0 0 0 / 0.08);
		transition:
			border-radius 320ms var(--ease-in-out),
			scale 320ms var(--ease-in-out);
	}
	/* Halo rings the art: the cover turns into the label of a disc. */
	.halo .stage {
		width: min(84cqi, 94cqh);
	}
	.halo .art-wrap {
		border-radius: 50%;
		scale: 0.66; /* = 2 * ART in Halo.svelte */
	}
	.art-wrap :global(.art) {
		border-radius: 0;
	}
	.idle-art {
		width: 100%;
		height: 100%;
		background: var(--surface);
		display: grid;
		place-items: center;
	}

	/* ---------- title ---------- */
	.meta {
		padding: 16px 4px 0;
		min-height: 6.2rem;
	}
	.title {
		font-size: 1.75rem;
		font-weight: 600;
		line-height: 1.08;
		letter-spacing: -0.03em;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		text-wrap: balance;
		padding-bottom: 0.06em;
	}
	.by {
		display: flex;
		align-items: center;
		gap: 10px;
		margin: 10px 0 0;
		color: var(--ink-2);
		font-size: 0.875rem;
	}
	.by.plain {
		margin-top: 8px;
	}
	.who {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		color: var(--ink);
		font-weight: 550;
	}

	/* ---------- readout + meter ---------- */
	.readout {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		margin: 16px 4px 6px;
		padding-top: 14px;
		border-top: 1px solid var(--line);
	}
	.elapsed {
		font-family: var(--font-dot);
		font-weight: 800;
		font-size: 2.25rem;
		line-height: 0.8;
		color: var(--ink);
	}
	.total {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--ink-2);
	}
	.progress {
		position: relative;
		margin: 0 4px;
		padding: 8px 0;
		cursor: pointer;
	}
	.progress.disabled {
		cursor: default;
	}
	/* A segmented meter: 3px cells, 1px gaps. */
	.rail {
		height: 10px;
		background: var(--line);
		mask-image: repeating-linear-gradient(90deg, #000 0 3px, transparent 3px 4px);
		overflow: hidden;
	}
	.fill {
		height: 100%;
		background: var(--ink);
		transform-origin: left;
	}
	.head-track {
		position: absolute;
		inset: 4px 0;
		pointer-events: none;
	}
	.head {
		position: absolute;
		left: -1px;
		top: 0;
		bottom: 0;
		width: 4px;
		box-shadow: inset 0 0 0 1px var(--led-rim);
		background: var(--accent);
	}
	.disabled .head {
		display: none;
	}

	/* ---------- transport ---------- */
	.controls {
		display: flex;
		align-items: flex-start;
		gap: 16px;
		margin: 14px 4px 0;
		padding-top: 16px;
		border-top: 1px solid var(--line);
	}
	.keys {
		display: flex;
		gap: 8px;
	}
	.slot {
		display: grid;
		justify-items: center;
		gap: 8px;
	}
	.cap-label {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: 0.625rem;
		color: var(--ink-3);
		white-space: nowrap;
	}
	.play {
		width: 88px;
		height: 56px;
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
	.sq {
		width: 56px;
		height: 56px;
	}
	.key.lit::after {
		content: '';
		position: absolute;
		top: 6px;
		right: 6px;
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: var(--led);
	}

	/* Fader: a ruled track and a rectangular cap. */
	.vol {
		flex: 1;
		min-width: 0;
		display: grid;
		gap: 8px;
		justify-items: start;
	}
	.vol :global(.fader) {
		position: relative;
		width: 100%;
		display: flex;
		align-items: center;
		height: 56px;
		touch-action: none;
		cursor: pointer;
	}
	.fader-rail {
		position: relative;
		flex: 1;
		height: 16px;
		/* tick marks every 10%: this is a measuring scale */
		background:
			linear-gradient(var(--ink-3), var(--ink-3)) center / 100% 2px no-repeat,
			repeating-linear-gradient(90deg, var(--line-2) 0 1px, transparent 1px calc(10% - 0.1px)) center / 100% 100% no-repeat;
	}
	.vol :global(.fader-range) {
		position: absolute;
		top: 7px;
		height: 2px;
		background: var(--ink);
	}
	.vol :global(.fader-cap) {
		display: block;
		width: 14px;
		height: 28px;
		border-radius: var(--r-sm);
		background: var(--key);
		box-shadow:
			inset 0 0 0 1px var(--line-2),
			0 1px 0 var(--key-edge);
		cursor: grab;
	}
	.vol :global(.fader-cap)::after {
		content: '';
		position: absolute;
		left: 3px;
		right: 3px;
		top: 13px;
		height: 2px;
		background: var(--accent);
	}
	.vol :global(.fader-cap:active) {
		cursor: grabbing;
	}

	.bars {
		margin: 4px 4px 0;
	}

	/* ---------- visualizer mode: soft keys printed along the bottom of the screen ---------- */
	.modes {
		position: absolute;
		z-index: 4;
		left: 8px;
		right: 8px;
		bottom: 8px;
		display: flex;
		border-radius: var(--r-sm);
		background: var(--surface);
		box-shadow: inset 0 0 0 1px var(--line);
		overflow: hidden;
	}
	.seg {
		flex: 1;
		display: grid;
		grid-auto-flow: column;
		grid-auto-columns: 1fr;
	}
	.seg-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 7px;
		height: 36px;
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.6875rem;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--ink-2);
		transition:
			background-color 120ms var(--ease-out),
			color 120ms var(--ease-out);
	}
	.seg-btn + .seg-btn,
	.lean {
		border-left: 1px solid var(--line);
	}
	.seg-btn[aria-pressed='true'] {
		background: var(--key);
		color: var(--ink);
		box-shadow: inset 0 -2px 0 var(--ink);
	}
	@media (hover: hover) {
		.seg-btn:not([aria-pressed='true']):hover {
			background: var(--surface-2);
			color: var(--ink);
		}
	}
	.lean {
		width: 44px;
	}


	/* Phone: the screen shrinks to a thumbnail beside the title; transport in the thumb zone. */
	@media (max-width: 560px) {
		.now {
			padding: 8px;
		}
		.screen {
			aspect-ratio: 16 / 11;
		}
		.meta {
			padding-top: 14px;
			min-height: 0;
		}
		.title {
			font-size: 1.375rem;
		}
		.readout {
			margin-top: 12px;
		}
		.elapsed {
			font-size: 1.75rem;
		}
		.vol {
			display: none; /* phones have hardware volume */
		}
		.controls {
			position: fixed;
			z-index: 40;
			left: 0;
			right: 0;
			bottom: 0;
			justify-content: center;
			margin: 0;
			padding: 10px 16px max(10px, env(safe-area-inset-bottom));
			background: var(--surface);
			border-top: 1px solid var(--line);
		}
		.keys {
			width: 100%;
			max-width: 420px;
			display: grid;
			grid-template-columns: 1fr 1fr 1fr;
			gap: 8px;
		}
		/* lyrics | play | skip, wherever lyrics exist */
		.keys .slot {
			grid-row: 1;
			justify-items: stretch;
		}
		.keys .slot:first-child {
			grid-column: 2;
		}
		.keys .slot:nth-child(2) {
			grid-column: 3;
		}
		.keys .slot:nth-child(3) {
			grid-column: 1;
		}
		.play,
		.sq,
		.keys :global(.hold) {
			width: 100%;
		}
		.cap-label {
			display: none;
		}
	}
</style>
