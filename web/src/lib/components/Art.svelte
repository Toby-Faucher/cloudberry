<script lang="ts">
	import type { Track } from '#lib/types.ts';

	let { track, size = 44 }: { track: Track; size?: number | string } = $props();
	let failed = $state(false);
	const css = $derived(typeof size === 'number' ? `${size}px` : size);
</script>

<div class="art" style:--size={css}>
	{#if track.thumbnail && !failed}
		<img src="/art/{encodeURIComponent(track.id)}.jpg" alt="" loading="lazy" onerror={() => (failed = true)} />
	{:else}
		<span aria-hidden="true">{track.title.trim()[0]?.toUpperCase() ?? '♪'}</span>
	{/if}
</div>

<style>
	.art {
		width: var(--size);
		aspect-ratio: 1;
		flex: none;
		border-radius: calc(var(--size) * 0.18);
		overflow: hidden;
		background:
			radial-gradient(circle at 30% 25%, #ffd2ad 0 18%, transparent 46%),
			radial-gradient(circle at 70% 75%, #f97316 0 30%, transparent 70%),
			#fb9a55;
		display: grid;
		place-items: center;
		container-type: inline-size;
	}
	img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	span {
		font-family: var(--font-display);
		font-weight: 700;
		font-size: 42cqi;
		color: rgb(255 255 255 / 0.9);
	}
</style>
