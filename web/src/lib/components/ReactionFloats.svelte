<script lang="ts">
	import { REACTION_MS, type Station } from '#lib/station.svelte.ts';
	import { reducedMotion } from '#lib/util.ts';

	/** Overlay for the now-playing art: incoming reactions drift up and fade. */
	let { station }: { station: Station } = $props();
	const latest = $derived(station.reactions.at(-1));
</script>

<div class="layer" aria-hidden="true" style:--ms="{REACTION_MS}ms">
	{#if reducedMotion}
		{#if latest}
			<span class="badge">{latest.emoji} <span>{latest.mine ? 'You' : latest.name}</span></span>
		{/if}
	{:else}
		{#each station.reactions as r (r.key)}
			<span class="float" style:--x={r.x} style:--drift={r.drift}>
				<span class="emoji">{r.emoji}</span>
				<span class="who">{r.mine ? 'You' : r.name}</span>
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
		display: grid;
		justify-items: center;
		gap: 2px;
		translate: -50% 0;
		animation: rise var(--ms) cubic-bezier(0.2, 0.7, 0.3, 1) forwards;
		will-change: transform, opacity;
	}
	.emoji {
		font-size: 2rem;
		line-height: 1;
		filter: drop-shadow(0 4px 10px rgb(0 0 0 / 0.25));
	}
	.who {
		font-size: 0.68rem;
		font-weight: 600;
		padding: 1px 7px;
		border-radius: 99px;
		background: rgb(20 14 10 / 0.55);
		color: #fff;
		backdrop-filter: blur(6px);
		white-space: nowrap;
	}
	@keyframes rise {
		0% {
			opacity: 0;
			transform: translate(0, 12px) scale(0.6);
		}
		12% {
			opacity: 1;
			transform: translate(calc(var(--drift) * 6px), 0) scale(1.05);
		}
		20% {
			transform: translate(calc(var(--drift) * 10px), -12%) scale(1);
		}
		70% {
			opacity: 1;
		}
		100% {
			opacity: 0;
			transform: translate(calc(var(--drift) * 36px), -380%) scale(0.9);
		}
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
