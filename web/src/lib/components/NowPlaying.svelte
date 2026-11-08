<script lang="ts">
	import { onMount } from 'svelte';
	import { Slider } from 'bits-ui';
	import type { Station } from '#lib/station.svelte.ts';
	import { expectedPositionMs } from '#lib/sync.ts';
	import { dur, fmtTime } from '#lib/util.ts';
	import Art from './Art.svelte';
	import Icon from './Icon.svelte';
	import { ModeSwitch, Visualizer, startAnalysis, stopAnalysis, viz } from '#lib/visualizer/index.ts';
	import Berry from './Berry.svelte';

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
			const st = station.state;
			pos = st?.current ? Math.max(0, expectedPositionMs(st, station.clock.serverNow())) : 0;
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

	function trackIn(node: Element) {
		return {
			duration: dur(520),
			delay: dur(80),
			css: (t: number) => {
				const e = 1 - Math.pow(1 - t, 4);
				return `opacity:${e};transform:translateY(${(1 - e) * 14}px) scale(${0.97 + 0.03 * e});filter:blur(${(1 - e) * 6}px)`;
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
				<div class="art-wrap" in:trackIn out:trackOut>
					{#if cur}
						<Art track={cur.track} size="100%" />
					{:else}
						<div class="idle-art"><Berry bead={44} opacity={0.3} /></div>
					{/if}
				</div>
			{/key}
		</div>
	</div>

	<div class="stack meta">
		{#key cur?.queue_id}
			<div in:trackIn out:trackOut>
				{#if cur}
					<h1 class="title" title={cur.track.title}>{cur.track.title}</h1>
					<p class="by">added by {cur.added_by}</p>
				{:else}
					<h1 class="title">The air is quiet</h1>
					<p class="by">
						{s?.queue.length ? 'Press play to start the next track.' : 'Paste a YouTube link to put something on.'}
					</p>
				{/if}
			</div>
		{/key}
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
		<div class="rail"><div class="fill" style:transform="scaleX({frac})"></div></div>
	</div>
	<div class="times num">
		<span>{fmtTime(shown)}</span>
		<span>{cur ? fmtTime(durationMs) : '0:00'}</span>
	</div>

	<div class="controls">
		<div class="vol">
			<Icon name="volume" size={16} />
			<Slider.Root type="single" bind:value={volume} min={0} max={1} step={0.01} class="vol-slider" aria-label="Volume (only for you)">
				<span class="vol-rail"><Slider.Range class="vol-range" /></span>
				<Slider.Thumb index={0} class="vol-thumb" />
			</Slider.Root>
		</div>

		<button
			class="play"
			aria-label={s?.playing ? 'Pause for everyone' : 'Play for everyone'}
			disabled={!cur && !s?.queue.length}
			onclick={() => station.send({ type: s?.playing ? 'pause' : 'play' })}
		>
			<Icon name={s?.playing ? 'pause' : 'play'} size={26} />
		</button>

		<div class="right">
			<button
				class="icon-btn skip"
				aria-label="Skip for everyone"
				title="Skip"
				disabled={!cur}
				onclick={() => station.send({ type: 'skip' })}
			>
				<Icon name="skip" size={20} />
			</button>
		</div>
	</div>

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
		margin: 8px auto 28px;
	}
	.glow {
		position: absolute;
		inset: -24%;
		border-radius: 50%;
		background: radial-gradient(closest-side, var(--accent), transparent);
		filter: blur(40px);
		opacity: calc(0.3 + var(--energy) * 1.4);
		transform: scale(calc(0.8 + var(--energy) * 0.6));
		will-change: transform, opacity;
		pointer-events: none;
	}
	.glow:not(.on) {
		opacity: 0.14;
		transform: scale(0.85);
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
		box-shadow: 0 24px 48px -24px rgb(120 50 0 / 0.45);
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
		min-height: 4.6rem;
	}
	.title {
		font-size: clamp(1.3rem, 1.1rem + 0.8vw, 1.65rem);
		line-height: 1.2;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		text-wrap: balance;
	}
	.by {
		margin: 6px 0 0;
		color: var(--ink-2);
		font-size: 0.9rem;
	}

	.progress {
		margin-top: 18px;
		padding: 10px 0;
		cursor: pointer;
	}
	.progress.disabled {
		cursor: default;
	}
	.rail {
		height: 5px;
		border-radius: 99px;
		background: var(--surface-2);
		overflow: hidden;
		transition: height 160ms var(--ease-out);
	}
	@media (hover: hover) {
		.progress:not(.disabled):hover .rail {
			height: 8px;
		}
	}
	.fill {
		height: 100%;
		background: var(--accent);
		transform-origin: left;
		border-radius: inherit;
	}
	.times {
		display: flex;
		justify-content: space-between;
		font-size: 0.78rem;
		color: var(--ink-3);
	}

	.controls {
		display: grid;
		grid-template-columns: 1fr auto 1fr;
		align-items: center;
		margin: 14px 0 22px;
	}
	.play {
		width: 64px;
		height: 64px;
		border-radius: 50%;
		display: grid;
		place-items: center;
		background: var(--accent);
		color: var(--on-accent);
		box-shadow: 0 10px 24px -10px var(--accent);
		transition:
			transform 140ms var(--ease-out),
			box-shadow 200ms var(--ease-out);
	}
	.play:active:not(:disabled) {
		transform: scale(0.94);
	}
	.play:disabled {
		opacity: 0.35;
		cursor: default;
		box-shadow: none;
	}
	.right {
		display: flex;
		justify-content: flex-end;
	}
	.skip {
		width: 44px;
		height: 44px;
	}

	.vol {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--ink-3);
		max-width: 130px;
	}
	.vol :global(.vol-slider) {
		position: relative;
		flex: 1;
		display: flex;
		align-items: center;
		height: 24px;
		touch-action: none;
		cursor: pointer;
	}
	.vol-rail {
		position: relative;
		flex: 1;
		height: 4px;
		border-radius: 99px;
		background: var(--surface-2);
		overflow: hidden;
	}
	.vol :global(.vol-range) {
		position: absolute;
		height: 100%;
		background: var(--ink-3);
	}
	.vol :global(.vol-thumb) {
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: var(--surface);
		border: 1px solid var(--line);
		box-shadow: 0 1px 4px rgb(0 0 0 / 0.18);
		transition: transform 120ms var(--ease-out);
	}
	.vol :global(.vol-thumb:active) {
		transform: scale(1.15);
	}

	@media (max-width: 480px) {
		.now {
			padding: 20px;
		}
	}
</style>
