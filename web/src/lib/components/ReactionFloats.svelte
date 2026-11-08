<script lang="ts">
	import { REACTION_MS, type Station } from '#lib/station.svelte.ts';
	import { reducedMotion } from '#lib/util.ts';
	import { smearAnimate } from '#lib/motion/motion.ts';
	import { stickerFor } from '#lib/reactions.ts';
	import Icon from '#lib/icons/Icon.svelte';

	/** Overlay for the screen: incoming reactions rise fast and fade, like readouts scrolling off. */
	let { station }: { station: Station } = $props();
	const latest = $derived(station.reactions.at(-1));
	const glyph = (emoji: string) => stickerFor(emoji)?.id ?? 'heart';

	/** A quick, straight rise (smeared), then a fade. */
	function rise(el: HTMLElement, drift: number) {
		smearAnimate(el, {
			y: [24, -170],
			x: [0, drift * 14],
			opacity: [0, 1, 1, 1, 0],
			duration: Math.min(REACTION_MS, 1600),
			ease: 'outQuart',
			clear: false
		});
	}
</script>

<div class="layer" aria-hidden="true">
	{#if reducedMotion}
		{#if latest}
			<span class="tag"><span class="chip"><Icon name={glyph(latest.emoji)} size={14} /></span><span class="who">{latest.mine ? 'You' : latest.name}</span></span>
		{/if}
	{:else}
		{#each station.reactions as r (r.key)}
			<span class="float" style:--x={r.x}>
				<span class="tag fly" use:rise={r.drift}>
					<span class="chip"><Icon name={glyph(r.emoji)} size={18} /></span>
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
		bottom: 15%;
		left: calc(14% + var(--x) * 64%);
	}
	.fly {
		opacity: 0;
		will-change: translate, opacity;
	}
	/* An orange key-cap with the pictogram, and a printed name strip beside it. */
	.tag {
		display: inline-flex;
		align-items: stretch;
		border-radius: var(--r-sm);
		overflow: hidden;
		box-shadow: 0 0 0 1px rgb(0 0 0 / 0.12);
	}
	.chip {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		background: var(--accent);
		color: #151515;
	}
	.who {
		display: grid;
		align-items: center;
		padding: 0 7px;
		background: var(--ink);
		color: var(--surface);
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.625rem;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		white-space: nowrap;
	}
	.layer > .tag {
		position: absolute;
		left: 12px;
		bottom: 56px; /* above the screen's soft keys */
	}
	.layer > .tag .chip {
		width: 24px;
		height: 24px;
	}
</style>
