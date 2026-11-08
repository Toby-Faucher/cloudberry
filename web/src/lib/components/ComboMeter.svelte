<script lang="ts">
	import { tick, untrack } from 'svelte';
	import type { Station } from '#lib/station.svelte.ts';
	import { comboHeat } from '#lib/game.ts';
	import { motion, reduced, smearAnimate } from '#lib/motion/motion.ts';
	import Sticker from '#lib/icons/Sticker.svelte';

	/** "[flame] ×5 combo!" over the art: pops and shakes on every hit, heats berry → blush. */
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
				if (fresh) smearAnimate(el, { y: [-60, 0], scale: [1.6, 1], opacity: [0, 1], duration: 460, ease: 'outBack(2)' });
				else {
					const h = comboHeat(c.count);
					motion(el, {
						scale: [1.25 + h * 0.2, 1],
						rotate: [0, -6 - h * 6, 5 + h * 5, -3, 0],
						x: [0, -4 - h * 4, 4 + h * 4, 0],
						y: 0,
						opacity: 1, // in case a fade-out was interrupted
						duration: 420,
						ease: 'outQuad'
					});
				}
			});
		} else if (had) {
			motion(el, { y: [0, -24], opacity: [1, 0], duration: 260, ease: 'inQuad', clear: false }).then(() => {
				if (!station.combo) shown = null;
			});
			if (reduced) shown = null;
		}
	});
</script>

{#if shown}
	<div class="combo" bind:this={el} style:--heat={comboHeat(shown.count)} aria-live="polite">
		<span class="e"><Sticker emoji={shown.emoji} size={34} /></span>
		<span class="n num">×{shown.count}</span>
		<span class="w">combo!</span>
	</div>
{/if}

<style>
	.combo {
		position: absolute;
		z-index: 6;
		top: 6%;
		left: 50%;
		margin-left: -80px;
		width: 160px;
		display: flex;
		align-items: baseline;
		justify-content: center;
		gap: 6px;
		padding: 6px 14px;
		border-radius: 999px;
		pointer-events: none;
		/* Dark ink on berry → blush: 7.2:1 → 4.5:1, and it's large bold text. */
		background: color-mix(in oklab, var(--blush-500) calc(var(--heat) * 100%), var(--berry-500));
		color: #211a16;
		box-shadow: var(--lip), var(--sticker);
		font-family: var(--font-display);
		font-weight: 700;
		white-space: nowrap;
	}
	.e {
		align-self: center;
		margin: -10px 0 -10px -8px;
		rotate: -10deg;
	}
	.n {
		font-size: calc(1.4rem + var(--heat) * 0.5rem);
		letter-spacing: -0.02em;
	}
	.w {
		font-size: 0.95rem;
	}
</style>
