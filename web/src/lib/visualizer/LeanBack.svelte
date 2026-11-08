<script lang="ts">
	import { fade } from 'svelte/transition';
	import Icon from '#lib/icons/Icon.svelte';
	import type { Station } from '#lib/station.svelte.ts';
	import { dur } from '#lib/util.ts';
	import { lineAt } from '#lib/lyrics.ts';
	import { ui } from '#lib/ui.svelte.ts';
	import Art from '#lib/components/Art.svelte';
	import Berry from '#lib/components/Berry.svelte';
	import Bloom from './Bloom.svelte';
	import { follow, frames, reducedMotion } from './analysis.ts';
	import { setLean, viz } from './state.svelte.ts';
	import { motion, smearAnimate } from '#lib/motion/motion.ts';

	/** Enter: the art flies up out of the dark (smeared), the words land after it. */
	function arrive(el: HTMLElement) {
		smearAnimate(el.querySelector<HTMLElement>('.art'), { y: [260, 0], scale: [0.6, 1], rotate: [-9, -2], opacity: [0, 1], duration: 720, ease: 'outBack(1.3)' });
		el.querySelectorAll<HTMLElement>('.meta, .lyric').forEach((m, i) =>
			motion(m, { y: [40, 0], opacity: [0, 1], duration: 600, delay: 220 + i * 90 })
		);
	}

	/** Fullscreen "lean-back" view: big art, title and the current lyric line over Bloom. */
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
		transition:fade={{ duration: dur(240) }}
	>
		<Bloom variant="fill" scale={0.5} />

		<div class="content" use:arrive>
			<div class="art" bind:this={artEl}>
				{#key cur?.queue_id}
					<div class="art-in" in:fade={{ duration: dur(500) }}>
						{#if cur}
							<Art track={cur.track} size="100%" />
						{:else}
							<div class="idle-art"><Berry bead={56} opacity={0.5} /></div>
						{/if}
					</div>
				{/key}
			</div>
			{#key cur?.queue_id}
				<div class="meta" in:fade={{ duration: dur(500), delay: dur(120) }}>
					<h1>{cur?.track.title ?? 'Dead air'}</h1>
					{#if cur}<p>added by {cur.added_by}</p>{/if}
				</div>
			{/key}
			{#if lines}
				<div class="lyric" aria-live="polite">
					{#key idx}
						<p in:fade={{ duration: dur(360) }}>{lines[idx]?.text || '♪'}</p>
					{/key}
					<p class="next" aria-hidden="true">{lines[idx + 1]?.text ?? ''}</p>
				</div>
			{/if}
		</div>

		<button class="exit" onclick={() => setLean(false)} aria-label="Exit lean-back (Esc)" title="Exit (Esc)">
			<Icon name="shrink" size={20} />
		</button>
	</div>
{/if}

<style>
	.lean {
		position: fixed;
		inset: 0;
		z-index: 100;
		background: var(--bg);
		display: grid;
		place-items: center;
		overflow: hidden;
		outline: none;
	}
	.lean.idle {
		cursor: none;
	}
	/* Content sits above the full-bleed Bloom canvas (z 0) in the dialog's own stacking context. */
	.content {
		position: relative;
		z-index: 1;
		display: grid;
		justify-items: center;
		gap: clamp(20px, 4vh, 40px);
		padding: 32px;
		text-align: center;
		max-width: min(92vw, 900px);
	}
	.art {
		width: min(52vh, 72vw);
		aspect-ratio: 1;
		display: grid;
		border-radius: 28px;
		overflow: hidden;
		rotate: -2deg;
		box-shadow:
			0 0 0 7px var(--die),
			0 12px 0 7px var(--shade);
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
		background: var(--surface-2);
	}
	/* Words ride on cream plates so they read over any part of the field. */
	.meta,
	.lyric {
		padding: 14px 26px 16px;
		border-radius: 22px;
		background: var(--surface);
		color: var(--ink);
		box-shadow: var(--sticker);
	}
	.meta {
		rotate: 1deg;
	}
	.meta h1 {
		font-size: clamp(1.8rem, 1.2rem + 2.6vw, 3.4rem);
		line-height: 1.08;
		letter-spacing: -0.03em;
		text-wrap: balance;
	}
	.meta p {
		margin: 6px 0 0;
		color: var(--ink-2);
		font-size: clamp(0.95rem, 0.8rem + 0.5vw, 1.15rem);
	}
	.lyric {
		display: grid;
		gap: 6px;
		min-height: 4.2em;
	}
	.lyric p {
		margin: 0;
		font-family: var(--font-display);
		font-size: clamp(1.2rem, 0.9rem + 1.2vw, 1.9rem);
		font-weight: 600;
		letter-spacing: -0.02em;
		text-wrap: balance;
	}
	.lyric .next {
		font-size: clamp(0.95rem, 0.8rem + 0.6vw, 1.25rem);
		color: var(--ink-2);
		font-weight: 500;
	}
	.exit {
		position: absolute;
		top: max(18px, env(safe-area-inset-top));
		right: max(18px, env(safe-area-inset-right));
		width: 42px;
		height: 42px;
		display: grid;
		place-items: center;
		z-index: 2;
		border-radius: 12px;
		color: var(--ink);
		background: var(--surface);
		box-shadow: 0 4px 0 var(--shade);
		transition:
			opacity 400ms ease,
			transform 120ms var(--ease-out);
	}
	.idle .exit {
		opacity: 0;
	}
</style>
