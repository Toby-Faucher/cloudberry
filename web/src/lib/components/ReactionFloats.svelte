<script lang="ts">
	import { REACTION_MS, type Station } from '#lib/station.svelte.ts';
	import { reducedMotion } from '#lib/util.ts';
	import { smearAnimate } from '#lib/motion/motion.ts';
	import Sticker from '#lib/icons/Sticker.svelte';

	/** Overlay for the now-playing art: incoming reactions drift up and fade. */
	let { station }: { station: Station } = $props();
	const latest = $derived(station.reactions.at(-1));

	/** Pops up fast (smeared), then drifts and fades. */
	function rise(el: HTMLElement, drift: number) {
		smearAnimate(el, {
			y: [18, -250],
			x: [0, drift * 46],
			scale: [0.4, 1.15, 1, 1, 0.9],
			opacity: [0, 1, 1, 1, 1, 0],
			duration: REACTION_MS,
			ease: 'outExpo',
			clear: false
		});
	}
</script>

<div class="layer" aria-hidden="true" style:--ms="{REACTION_MS}ms">
	{#if reducedMotion}
		{#if latest}
			<span class="badge"><Sticker emoji={latest.emoji} size={26} /> <span>{latest.mine ? 'You' : latest.name}</span></span>
		{/if}
	{:else}
		{#each station.reactions as r (r.key)}
			<span class="float" style:--x={r.x}>
				<span class="fly" use:rise={r.drift}>
					<Sticker emoji={r.emoji} size={52} />
					<span class="who">{r.mine ? 'You' : r.name}</span>
				</span>
			</span>
		{/each}
	{/if}
</div>

<style>
	.layer {
		position: absolute;
		inset: 0;
		pointer-events: none;
		z-index: 5; /* above the art (3) in the now-playing stacking context */
	}
	.float {
		position: absolute;
		bottom: 8%;
		left: calc(12% + var(--x) * 76%);
		translate: -50% 0;
	}
	.fly {
		display: grid;
		justify-items: center;
		gap: 2px;
		opacity: 0;
		will-change: translate, scale, opacity;
	}
	/* Name tag under the sticker: a tiny ink label with the same white cut. */
	.who {
		font-family: var(--font-display);
		font-size: 0.74rem;
		font-weight: 650;
		padding: 1px 8px;
		border-radius: var(--r-sm);
		background: var(--ink);
		color: var(--surface);
		box-shadow: 0 0 0 2px var(--die);
		white-space: nowrap;
	}
	.badge {
		position: absolute;
		left: 12px;
		bottom: 12px;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 3px 10px 3px 6px;
		border-radius: var(--r-sm);
		background: var(--ink);
		color: var(--surface);
		box-shadow: 0 0 0 2px var(--die);
		font-size: 0.8rem;
		font-weight: 600;
	}
</style>
