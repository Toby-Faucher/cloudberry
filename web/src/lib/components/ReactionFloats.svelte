<script lang="ts">
	import { REACTION_MS, type Station } from '#lib/station.svelte.ts';
	import { reducedMotion } from '#lib/util.ts';
	import { smearAnimate } from '#lib/motion/motion.ts';

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
			<span class="badge">{latest.emoji} <span>{latest.mine ? 'You' : latest.name}</span></span>
		{/if}
	{:else}
		{#each station.reactions as r (r.key)}
			<span class="float" style:--x={r.x}>
				<span class="fly" use:rise={r.drift}>
					<span class="emoji">{r.emoji}</span>
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
		z-index: 2;
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
	.emoji {
		font-size: 2.2rem;
		line-height: 1;
		filter: drop-shadow(0 4px 10px rgb(0 0 0 / 0.25));
	}
	.who {
		font-size: 0.7rem;
		font-weight: 650;
		padding: 2px 8px;
		border-radius: 99px;
		background: rgb(20 14 10 / 0.7);
		color: #fff;
		white-space: nowrap;
	}
	.badge {
		position: absolute;
		left: 12px;
		bottom: 12px;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 4px 10px 4px 8px;
		border-radius: 99px;
		background: rgb(20 14 10 / 0.6);
		color: #fff;
		font-size: 0.8rem;
		font-weight: 600;
	}
</style>
