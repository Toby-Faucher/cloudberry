<script lang="ts">
	import type { Snippet } from 'svelte';
	import { fade } from 'svelte/transition';
	import type { Station } from '#lib/station.svelte.ts';
	import { dur } from '#lib/util.ts';
	import Art from '#lib/components/Art.svelte';
	import Berry from '#lib/components/Berry.svelte';
	import Bloom from './Bloom.svelte';
	import { follow, frames, reducedMotion } from './analysis.ts';
	import { setLean, viz } from './state.svelte.ts';

	/** Fullscreen "lean-back" view: big art + title over Bloom. `children` renders under the title (lyrics). */
	let { station, children }: { station: Station; children?: Snippet } = $props();

	const cur = $derived(station.state?.current ?? null);
	let idle = $state(false);
	let root = $state<HTMLDivElement>();
	let artEl = $state<HTMLDivElement>();
	let idleTimer = 0;
	let wasFullscreen = false;

	const typing = (t: EventTarget | null) =>
		t instanceof HTMLElement && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName));

	function onKey(e: KeyboardEvent) {
		if (e.ctrlKey || e.metaKey || e.altKey || typing(e.target)) return;
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
		<div class="vignette" aria-hidden="true"></div>

		<div class="content">
			<div class="art" bind:this={artEl}>
				{#key cur?.queue_id}
					<div class="art-in" in:fade={{ duration: dur(500) }}>
						{#if cur}
							<Art track={cur.track} size="100%" />
						{:else}
							<div class="idle-art"><Berry bead={56} opacity={0.35} /></div>
						{/if}
					</div>
				{/key}
			</div>
			{#key cur?.queue_id}
				<div class="meta" in:fade={{ duration: dur(500), delay: dur(120) }}>
					<h1>{cur?.track.title ?? 'The air is quiet'}</h1>
					{#if cur}<p>added by {cur.added_by}</p>{/if}
				</div>
			{/key}
			{@render children?.()}
		</div>

		<button class="exit" onclick={() => setLean(false)} aria-label="Exit lean-back (Esc)" title="Exit (Esc)">
			<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
				<path d="M4 14h6v6M20 10h-6V4M14 10l7-7M3 21l7-7" />
			</svg>
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
	.vignette {
		position: absolute;
		inset: 0;
		background: radial-gradient(80% 70% at 50% 45%, transparent 40%, color-mix(in oklab, var(--bg) 70%, transparent));
		pointer-events: none;
	}
	.content {
		position: relative;
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
		box-shadow:
			0 40px 90px -30px rgb(60 20 0 / 0.55),
			0 0 0 1px rgb(255 255 255 / 0.06);
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
	.meta h1 {
		font-size: clamp(1.8rem, 1.2rem + 2.6vw, 3.4rem);
		line-height: 1.08;
		letter-spacing: -0.035em;
		text-wrap: balance;
		text-shadow: 0 2px 30px color-mix(in oklab, var(--bg) 60%, transparent);
	}
	.meta p {
		margin: 10px 0 0;
		color: var(--ink-2);
		font-size: clamp(0.95rem, 0.8rem + 0.5vw, 1.15rem);
	}
	.exit {
		position: absolute;
		top: max(18px, env(safe-area-inset-top));
		right: max(18px, env(safe-area-inset-right));
		width: 42px;
		height: 42px;
		display: grid;
		place-items: center;
		border-radius: 50%;
		color: var(--ink);
		background: color-mix(in oklab, var(--surface) 55%, transparent);
		backdrop-filter: blur(12px);
		transition:
			opacity 400ms ease,
			transform 120ms var(--ease-out);
	}
	.exit:active {
		transform: scale(0.94);
	}
	.idle .exit {
		opacity: 0;
	}
</style>
