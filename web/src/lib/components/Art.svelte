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
		<Berry bead="9cqi" opacity={0.55} />
	{:else}
		<span aria-hidden="true">{track.title.trim()[0]?.toUpperCase() ?? '♪'}</span>
	{/if}
</div>

<style>
	.art {
		width: var(--size);
		aspect-ratio: 1;
		flex: none;
		border-radius: var(--r-sm);
		overflow: hidden;
		background: var(--surface-2);
		box-shadow: inset 0 0 0 1px var(--line);
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
	/* No cover: the title's initial, printed in mono. */
	span {
		font-family: var(--font-mono);
		font-weight: 500;
		font-size: 42cqi;
		color: var(--ink-2);
	}
</style>
