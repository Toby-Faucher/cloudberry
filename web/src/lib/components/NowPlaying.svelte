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

	// The old LIVE chip, folded into the art as an on-air sticker.
	const air = $derived(
		station.status === 'connecting'
			? { cls: 'tuning', text: 'tuning in' }
			: station.status === 'reconnecting'
				? { cls: 'lost', text: 'reconnecting' }
				: s?.playing && cur
					? { cls: 'on', text: 'on air' }
					: { cls: 'off', text: 'paused' }
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

<!--
	Stacking (the Bloom bug): .now is its own stacking context. Visualizer layers sit at z -1, i.e.
	under every in-flow block of the deck (title, controls), however far the aura bleeds. The art
	(z 3), the on-air sticker (4), reaction floats (5) and the combo (6) sit above the deck.
-->
<section class="now" aria-label="Now playing">
	<div class="stage" class:halo={viz.mode === 'halo'}>
		<div class="disc" class:on={s?.playing} aria-hidden="true"></div>
		{#if viz.mode !== 'bars'}<Visualizer mode={viz.mode} />{/if}
		<div class="stack art-layer">
			{#key cur?.queue_id}
				<div class="swap" use:artIn out:artOut>
					<div class="art-wrap">
						{#if cur}
							<Art track={cur.track} size="100%" />
						{:else}
							<div class="idle-art"><Berry bead={44} opacity={0.6} /></div>
						{/if}
					</div>
				</div>
			{/key}
		</div>
		<span class="air {air.cls}" role="status">{air.text}</span>
		<ReactionFloats {station} />
		<ComboMeter {station} />
	</div>

	<div class="deck card">
		<div class="stack meta">
			{#key cur?.queue_id}
				<div in:trackIn out:trackOut>
					{#if cur}
						<h1 class="title" id="np-title" title={cur.track.title}>{cur.track.title}</h1>
						<p class="by">added by <span class="who"><Avatar name={cur.added_by} size={22} />{cur.added_by}</span></p>
					{:else}
						<h1 class="title" id="np-title">Dead air</h1>
						<p class="by">
							{s?.queue.length ? 'Hit play and the next one starts.' : 'Drop a YouTube link and get this going.'}
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
				<Icon name="volume" size={20} />
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
				<span class="glyph" class:is-play={!s?.playing}><Icon name={s?.playing ? 'pause' : 'play'} size={34} /></span>
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
						<Icon name="mic" size={20} />
					</button>
				{/if}
				<HoldSkip {station} disabled={!cur} />
			</div>
		</div>

		<ReactionBar {station} />

		{#if viz.mode === 'bars'}<Visualizer mode="bars" />{/if}
		<ModeSwitch />
	</div>
</section>

<style>
	.now {
		position: relative;
		isolation: isolate;
		display: flex;
		flex-direction: column;
	}

	/* ---------- the art: the one big thing on the page ---------- */
	.stage {
		position: relative; /* containing block for the visualizers; NOT a stacking context */
		width: min(100%, 372px);
		margin: 6px auto 0;
	}
	.stage :global(canvas),
	.stage :global(.glow),
	.stage :global(.css),
	.disc {
		z-index: -1;
	}
	/* A flat tinted disc behind the art that swells with the music: a shape, not a glow. */
	.disc {
		position: absolute;
		inset: -8%;
		border-radius: 50%;
		background: var(--track-tint);
		opacity: 0.4;
		transform: scale(calc(0.9 + var(--energy) * 0.35));
		will-change: transform;
		pointer-events: none;
	}
	.halo .disc {
		display: none; /* Halo draws its own disc */
	}
	.disc:not(.on) {
		opacity: 0.25;
		transform: scale(0.9);
		transition:
			opacity 1.2s var(--ease-out),
			transform 1.2s var(--ease-out);
	}

	.art-layer {
		position: relative;
		z-index: 3;
		scale: var(--viz-pulse, 1); /* art breathes on beats */
	}
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
		border-radius: 24px;
		overflow: hidden;
		rotate: -2.5deg;
		/* The sticker: white die-cut, hard drop onto the deck. */
		box-shadow:
			0 0 0 6px var(--die),
			0 10px 0 6px var(--shade);
		transition:
			border-radius 480ms var(--ease-in-out),
			scale 480ms var(--ease-in-out),
			rotate 480ms var(--ease-in-out);
	}
	.halo .art-wrap {
		border-radius: 50%;
		scale: 0.66; /* = 2 * ART in Halo.svelte */
		rotate: 0deg;
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

	/* On-air sticker, slapped on the art's corner. */
	.air {
		position: absolute;
		z-index: 4;
		top: -6px;
		left: -10px;
		rotate: -9deg;
		padding: 4px 12px 5px;
		border-radius: var(--r-sm);
		font-family: var(--font-display);
		font-size: 1.05rem;
		font-weight: 700;
		line-height: 1.1;
		background: var(--surface-2);
		color: var(--ink);
		box-shadow:
			0 0 0 3px var(--die),
			0 4px 0 3px var(--shade);
		pointer-events: none;
		white-space: nowrap;
	}
	.air.on {
		background: #b8361f; /* blush, deepened so cream text clears 4.5:1 */
		color: #fbf4e2;
	}
	.air.lost {
		background: var(--berry-300);
		color: #1d2916;
	}
	.halo .air {
		top: 8%;
		left: 8%;
	}

	/* ---------- the deck: cream sticker the art overlaps ---------- */
	.deck {
		margin-top: -52px;
		padding: 76px 28px 22px;
		display: flex;
		flex-direction: column;
	}
	.meta {
		text-align: center;
		min-height: 6.4rem;
	}
	/* Display-size, confident; second only to the art. */
	.title {
		font-size: clamp(1.9rem, 1.3rem + 1.4vw, 2.6rem);
		font-weight: 700;
		line-height: 1.06;
		letter-spacing: -0.025em;
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
		margin-top: 16px;
		padding: 10px 0;
		cursor: pointer;
	}
	.progress.disabled {
		cursor: default;
	}
	.rail {
		height: 10px;
		border-radius: 99px;
		background: var(--surface-2);
		overflow: hidden;
		transition: height 160ms var(--ease-out);
	}
	@media (hover: hover) {
		.progress:not(.disabled):hover .rail {
			height: 14px;
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
		font-family: var(--font-display);
		font-size: 0.95rem;
		font-weight: 600;
		color: var(--ink-2);
	}

	.controls {
		display: grid;
		grid-template-columns: 1fr auto 1fr;
		align-items: center;
		margin: 14px 0 18px;
	}
	.play {
		width: 80px;
		height: 80px;
		border-radius: 50%;
		display: grid;
		place-items: center;
		background: var(--accent);
		color: var(--on-accent);
		box-shadow:
			var(--lip),
			0 0 0 4px var(--die),
			0 6px 0 4px var(--shade-2);
		transition:
			background-color 160ms var(--ease-out),
			box-shadow 120ms var(--ease-out),
			translate 120ms var(--ease-out);
	}
	@media (hover: hover) {
		.play:hover:not(:disabled) {
			background: var(--accent-hover);
		}
	}
	/* Pressed: the drop collapses and the button sinks into it. */
	.play:active:not(:disabled) {
		translate: 0 5px;
		box-shadow:
			inset 0 3px 0 rgb(110 45 0 / 0.3),
			0 0 0 4px var(--die),
			0 1px 0 4px var(--shade-2);
	}
	.play:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}
	/* The triangle's visual centre sits left of its box. */
	.glyph.is-play {
		translate: 2px 0;
	}
	.right {
		display: flex;
		justify-content: flex-end;
		align-items: center;
		gap: 8px;
	}

	.vol {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--ink-2);
		max-width: 136px;
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
		height: 8px;
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
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background: var(--ink);
		border: 3px solid var(--surface);
		box-shadow: 0 2px 0 var(--shade-2);
		transition: scale 120ms var(--ease-out);
	}
	.vol :global(.vol-thumb:active) {
		scale: 1.15;
	}

	/* Phone: the art perches on the deck's corner beside the title; transport in the thumb zone. */
	@media (max-width: 560px) {
		.stage {
			position: absolute;
			top: 0;
			left: 14px;
			width: 108px;
			margin: 0;
		}
		.stage :global(.berry) {
			scale: 0.5;
		}
		.art-wrap {
			border-radius: 16px;
			rotate: -4deg;
			box-shadow:
				0 0 0 4px var(--die),
				0 6px 0 4px var(--shade);
		}
		.air {
			font-size: 0.8rem;
			padding: 2px 8px 3px;
			top: -10px;
			left: -6px;
		}
		.deck {
			margin-top: 18px;
			padding: 14px 16px 16px;
		}
		.meta {
			margin-left: 118px;
			min-height: 92px;
			text-align: left;
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
			margin-top: 6px;
		}
		.times {
			margin-bottom: 14px;
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
			padding: 12px 20px max(12px, env(safe-area-inset-bottom));
			background: var(--surface);
			box-shadow: 0 -4px 0 var(--die);
		}
		.play {
			width: 64px;
			height: 64px;
		}
	}
</style>
