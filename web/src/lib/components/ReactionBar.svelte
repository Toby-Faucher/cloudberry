<script lang="ts">
	import type { Station } from '#lib/station.svelte.ts';
	import { REACTIONS } from '#lib/types.ts';
	import { STICKERS } from '#lib/reactions.ts';
	import Sticker from '#lib/icons/Sticker.svelte';

	let { station }: { station: Station } = $props();
	// A sticker sheet, not a toolbar: each one sits a little crooked.
	const tilt = [-6, 4, -2, 5, -4, 3];
</script>

<div class="sheet" role="group" aria-label="React for everyone">
	{#each REACTIONS as e, i (e)}
		<button class="r" style:--tilt="{tilt[i]}deg" aria-label="React {STICKERS[e].label}" title={STICKERS[e].label} onclick={() => station.react(e)}>
			<Sticker emoji={e} size={40} />
		</button>
	{/each}
</div>

<style>
	.sheet {
		display: flex;
		justify-content: center;
		flex-wrap: wrap;
		gap: 2px;
		margin: 0 0 18px;
	}
	.r {
		display: grid;
		place-items: center;
		width: 50px;
		height: 50px;
		border-radius: var(--r-md);
		rotate: var(--tilt);
		transition:
			rotate 180ms var(--ease-out),
			translate 180ms var(--ease-out);
	}
	@media (hover: hover) {
		.r:hover {
			rotate: 0deg;
			translate: 0 -4px;
		}
	}
</style>
