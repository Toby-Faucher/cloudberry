<script lang="ts">
	import type { Track } from '#lib/types.ts';
	import Berry from './Berry.svelte';

	let { track, size = 44 }: { track: Track; size?: number | string } = $props();
	let failed = $state(false);
	const css = $derived(typeof size === 'number' ? `${size}px` : size);
	// Small tiles get the initial; big ones (now playing, lean-back) get the berry mark.
	const big = $derived(typeof size === 'string' || size >= 96);
</script>

<div class="art" style:--size={css}>
	{#if track.thumbnail && !failed}
		<img src="/art/{encodeURIComponent(track.id)}.jpg" alt="" loading="lazy" onerror={() => (failed = true)} />
	{:else if big}
		<Berry bead="19cqi" />
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
		background: var(--berry-300);
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
		font-size: 46cqi;
		color: var(--berry-700);
	}
</style>
