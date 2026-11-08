<script lang="ts">
	import { untrack } from 'svelte';
	import { fade } from 'svelte/transition';
	import Icon from '#lib/icons/Icon.svelte';
	import type { Station } from '#lib/station.svelte.ts';
	import { dur } from '#lib/util.ts';
	import { lineAt } from '#lib/lyrics.ts';
	import { ui } from '#lib/ui.svelte.ts';
	import Art from '#lib/components/Art.svelte';
	import ShaderCanvas from './ShaderCanvas.svelte';
	import ShaderPicker from './ShaderPicker.svelte';
	import { follow, frames, reducedMotion } from './analysis.ts';
	import { leanKey, setLean, trackChanged, viz } from './state.svelte.ts';
	import { motion } from '#lib/motion/motion.ts';

	/** Enter: the art slides up into place, the readouts land after it. Short and mechanical. */
	function arrive(el: HTMLElement) {
		motion(el.querySelector<HTMLElement>('.art'), { y: [24, 0], opacity: [0, 1], duration: 320 });
		el.querySelectorAll<HTMLElement>('.meta, .lyric').forEach((m, i) =>
			motion(m, { y: [12, 0], opacity: [0, 1], duration: 260, delay: 120 + i * 60 })
		);
	}

	/** Fullscreen "lean-back" view: big art, title and the current lyric line over a shader from the pack. */
	let { station }: { station: Station } = $props();

	const cur = $derived(station.state?.current ?? null);
	let idle = $state(false);
	let root = $state<HTMLDivElement>();
	let artEl = $state<HTMLDivElement>();
	let idleTimer = 0;
	let wasFullscreen = false;

	const typing = (t: EventTarget | null) =>
		t instanceof HTMLElement && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName));

	function onKey(e: KeyboardEvent) {
		if (e.ctrlKey || e.metaKey || e.altKey || typing(e.target) || ui.help || ui.drawer) return;
		if (e.key === 'f' || e.key === 'F') {
			e.preventDefault();
			setLean(!viz.lean);
		} else if (e.key === 'Escape' && viz.lean) {
			setLean(false);
		} else if (viz.lean && leanKey(e.key)) {
			e.preventDefault();
			wake(); // show the picker so the change is visible
		}
	}

	// Leaving browser fullscreen (Esc is eaten by the browser there) also leaves lean-back.
	function onFullscreen() {
		if (document.fullscreenElement) wasFullscreen = true;
		else if (wasFullscreen && viz.lean) {
			wasFullscreen = false;
			viz.lean = false;
		}
	}

	function wake() {
		idle = false;
		clearTimeout(idleTimer);
		idleTimer = window.setTimeout(() => (idle = true), 2500);
	}

	// Auto-cycle: a new track moves to the next shader.
	let lastTrack: unknown = Symbol();
	$effect(() => {
		const q = cur?.queue_id;
		if (typeof lastTrack !== 'symbol' && q !== lastTrack) untrack(trackChanged);
		lastTrack = q;
	});

	// Current lyric line (lyrics are fetched by <Lyrics/> into ui.lines).
	let idx = $state(-1);
	const lines = $derived(viz.lean && ui.lyrics ? ui.lines : null);
	$effect(() => {
		if (!lines) return;
		let raf = 0;
		const tick = () => {
			idx = lineAt(lines, station.positionMs());
			raf = requestAnimationFrame(tick);
		};
		tick();
		return () => cancelAnimationFrame(raf);
	});

	$effect(() => {
		if (!viz.lean) return;
		wake();
		root?.focus({ preventScroll: true });
		let pulse = 0;
		const unsub = reducedMotion
			? () => {}
			: frames.subscribe((f) => {
					pulse = follow(pulse, f.beat * f.beat, f.dt, 0.035, 0.12);
					artEl?.style.setProperty('scale', (1 + 0.02 * pulse).toFixed(4));
				});
		return () => {
			unsub();
			clearTimeout(idleTimer);
		};
	});
</script>

<svelte:window onkeydown={onKey} />
<svelte:document onfullscreenchange={onFullscreen} />

{#if viz.lean}
	<div
		class="lean"
		class:idle
		role="dialog"
		aria-modal="true"
		aria-label="Lean-back view"
		tabindex="-1"
		bind:this={root}
		onpointermove={wake}
		onpointerdown={wake}
		transition:fade={{ duration: dur(240) }}
	>
		<ShaderCanvas shader={viz.shader} still={cur?.queue_id} />

		<div class="content" use:arrive>
			<div class="art" bind:this={artEl}>
				{#key cur?.queue_id}
					<div class="art-in" in:fade={{ duration: dur(300) }}>
						{#if cur}
							<Art track={cur.track} size="100%" />
						{:else}
							<div class="idle-art">NO SIGNAL</div>
						{/if}
					</div>
				{/key}
			</div>
			{#key cur?.queue_id}
				<div class="meta plate" in:fade={{ duration: dur(300), delay: dur(80) }}>
					<p class="label"><span class="led" class:on={!!cur}></span>{cur ? 'Now playing' : 'Idle'}</p>
					<h1>{cur?.track.title ?? 'Dead air'}</h1>
					{#if cur}<p class="by">Added by {cur.added_by}</p>{/if}
				</div>
			{/key}
			{#if lines}
				<div class="lyric plate" aria-live="polite">
					{#key idx}
						<p in:fade={{ duration: dur(200) }}>{lines[idx]?.text || '♪'}</p>
					{/key}
					<p class="next" aria-hidden="true">{lines[idx + 1]?.text ?? ''}</p>
				</div>
			{/if}
		</div>

		<div class="pick">
			<ShaderPicker />
		</div>

		<button class="exit" onclick={() => setLean(false)} aria-label="Exit lean-back (Esc)" title="Exit (Esc)">
			<Icon name="shrink" size={18} />
		</button>
	</div>
{/if}

<style>
	/* Instrument display. Self-contained tokens: the app shell's names may change. */
	.lean {
		--lb-paper: #ecebe8;
		--lb-panel: #dad9d6;
		--lb-ink: #1e1f21;
		--lb-ink-2: #5d5f63;
		--lb-line: #1e1f21;
		--lb-orange: #f0912b;
		--lb-mono: 'JetBrains Mono', 'IBM Plex Mono', ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
		--lb-sans: 'Inter Variable', 'Inter', 'Helvetica Neue', Arial, system-ui, sans-serif;
		position: fixed;
		inset: 0;
		z-index: 100;
		background: var(--lb-paper);
		color: var(--lb-ink);
		display: grid;
		place-items: center;
		overflow: hidden;
		outline: none;
	}
	@media (prefers-color-scheme: dark) {
		.lean {
			--lb-paper: #121315;
			--lb-panel: #1f2022;
			--lb-ink: #e6e5e1;
			--lb-ink-2: #9a9b9e;
			--lb-line: #e6e5e1;
		}
	}
	.lean.idle {
		cursor: none;
	}
	/* Content sits above the full-bleed shader canvas (z 0) in the dialog's own stacking context. */
	.content {
		position: relative;
		z-index: 1;
		display: grid;
		justify-items: center;
		gap: clamp(14px, 3vh, 28px);
		/* Bottom room for the shader picker. */
		padding: 32px 32px 96px;
		text-align: center;
		max-width: min(92vw, 900px);
	}
	.art {
		width: min(50vh, 72vw);
		aspect-ratio: 1;
		display: grid;
		overflow: hidden;
		border: 1px solid var(--lb-line);
		outline: 6px solid var(--lb-paper);
		will-change: scale;
	}
	.art-in {
		grid-area: 1 / 1;
	}
	.art :global(.art) {
		border-radius: 0;
	}
	.idle-art {
		width: 100%;
		height: 100%;
		display: grid;
		place-items: center;
		background: var(--lb-panel);
		color: var(--lb-ink-2);
		font: 500 0.8rem/1 var(--lb-mono);
		letter-spacing: 0.2em;
	}
	/* Readout plates: paper, one hairline, mono label. Text always reads over any shader. */
	.plate {
		padding: 10px 18px 12px;
		background: var(--lb-paper);
		color: var(--lb-ink);
		border: 1px solid var(--lb-line);
	}
	.label,
	.by {
		margin: 0;
		font: 500 0.72rem/1.2 var(--lb-mono);
		letter-spacing: 0.14em;
		text-transform: uppercase;
		color: var(--lb-ink-2);
	}
	.label {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		margin-bottom: 6px;
	}
	.led {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--lb-panel);
		box-shadow: inset 0 0 0 1px var(--lb-ink-2);
	}
	.led.on {
		background: var(--lb-orange);
		box-shadow: none;
	}
	.meta h1 {
		margin: 0;
		font-family: var(--lb-sans);
		font-size: clamp(1.5rem, 1rem + 2.2vw, 2.8rem);
		font-weight: 600;
		line-height: 1.08;
		letter-spacing: -0.035em;
		text-wrap: balance;
	}
	.by {
		margin-top: 8px;
	}
	.lyric {
		display: grid;
		gap: 4px;
		min-height: 4.2em;
	}
	.lyric p {
		margin: 0;
		font-family: var(--lb-sans);
		font-size: clamp(1.1rem, 0.9rem + 1vw, 1.7rem);
		font-weight: 600;
		letter-spacing: -0.02em;
		text-wrap: balance;
	}
	.lyric .next {
		font: 400 clamp(0.8rem, 0.7rem + 0.4vw, 1rem) / 1.3 var(--lb-mono);
		color: var(--lb-ink-2);
	}
	.exit {
		position: absolute;
		top: max(18px, env(safe-area-inset-top));
		right: max(18px, env(safe-area-inset-right));
		width: 40px;
		height: 40px;
		display: grid;
		place-items: center;
		z-index: 2;
		border-radius: 0;
		color: var(--lb-ink);
		background: var(--lb-paper);
		border: 1px solid var(--lb-line);
		transition: opacity 200ms linear;
	}
	.exit:active {
		background: var(--lb-panel);
	}
	.pick {
		position: absolute;
		z-index: 2;
		left: 0;
		right: 0;
		bottom: max(20px, env(safe-area-inset-bottom));
		display: flex;
		justify-content: center;
		transition:
			opacity 200ms linear,
			translate 200ms cubic-bezier(0.2, 0, 0, 1);
	}
	.idle .exit,
	.idle .pick {
		opacity: 0;
		pointer-events: none;
	}
	.idle .pick {
		translate: 0 8px;
	}
	/* Keyboard users keep the picker while they're in it. */
	.idle .pick:has(:global(:focus-visible)) {
		opacity: 1;
		pointer-events: auto;
		translate: 0 0;
	}
</style>
