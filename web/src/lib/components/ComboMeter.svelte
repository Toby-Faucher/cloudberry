<script lang="ts">
	import { tick, untrack } from 'svelte';
	import type { Station } from '#lib/station.svelte.ts';
	import { comboHeat } from '#lib/game.ts';
	import { motion, reduced, smearAnimate } from '#lib/motion/motion.ts';
	import Icon from '#lib/icons/Icon.svelte';
	import { stickerFor } from '#lib/reactions.ts';

	/** "[flame] ×5 combo" readout on the screen: ticks on every hit, its lamp heats berry → blush. */
	let { station }: { station: Station } = $props();

	const combo = $derived(station.combo);
	let el = $state<HTMLElement>();
	let shown = $state<{ emoji: string; count: number } | null>(null);

	$effect(() => {
		const c = combo;
		const had = untrack(() => shown);
		if (c) {
			const fresh = !had;
			shown = { ...c };
			tick().then(() => {
				if (fresh) smearAnimate(el, { y: [-28, 0], opacity: [0, 1], duration: 260, ease: 'outQuart' });
				else {
					const h = comboHeat(c.count);
					motion(el, { scale: [1.08 + h * 0.08, 1], x: [0, -2 - h * 2, 2 + h * 2, 0], y: 0, opacity: 1, duration: 200, ease: 'outQuart' });
				}
			});
		} else if (had) {
			motion(el, { y: [0, -12], opacity: [1, 0], duration: 180, ease: 'inQuad', clear: false }).then(() => {
				if (!station.combo) shown = null;
			});
			if (reduced) shown = null;
		}
	});
</script>

{#if shown}
	<div class="combo" bind:this={el} style:--heat={comboHeat(shown.count)} aria-live="polite">
		<span class="e"><Icon name={stickerFor(shown.emoji)?.id ?? 'flame'} size={16} /></span>
		<span class="n num">×{shown.count}</span>
		<span class="w">combo</span>
	</div>
{/if}

<style>
	.combo {
		position: absolute;
		z-index: 6;
		top: 10px;
		right: 10px;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 10px 0 0;
		height: 34px;
		border-radius: var(--r-sm);
		overflow: hidden;
		pointer-events: none;
		background: var(--ink);
		color: var(--surface);
		white-space: nowrap;
	}
	/* The lamp: berry at ×2, blush at full heat. Dark ink on it clears 4.5:1 across the range. */
	.e {
		display: grid;
		place-items: center;
		align-self: stretch;
		width: 34px;
		background: color-mix(in oklab, var(--blush-500) calc(var(--heat) * 100%), var(--berry-500));
		color: #151515;
	}
	.n {
		font-family: var(--font-dot);
		font-weight: 800;
		font-size: 1.5rem;
		line-height: 1;
	}
	.w {
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.625rem;
		letter-spacing: 0.06em;
		text-transform: uppercase;
	}
</style>
