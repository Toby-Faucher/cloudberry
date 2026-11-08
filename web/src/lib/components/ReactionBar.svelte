<script lang="ts">
	import type { Station } from '#lib/station.svelte.ts';
	import { REACTIONS } from '#lib/types.ts';
	import { STICKERS } from '#lib/reactions.ts';
	import Icon from '#lib/icons/Icon.svelte';

	let { station }: { station: Station } = $props();

	// The key flashes orange for a beat when pressed: the press answers back even before the float.
	let flash = $state<string | null>(null);
	let timer = 0;
	function react(e: (typeof REACTIONS)[number]) {
		station.react(e);
		flash = e;
		clearTimeout(timer);
		timer = window.setTimeout(() => (flash = null), 160);
	}
</script>

<div class="pad" role="group" aria-label="React for everyone">
	{#each REACTIONS as e (e)}
		<button class="r key" class:flash={flash === e} aria-label="React {STICKERS[e].label}" title={STICKERS[e].label} onclick={() => react(e)}>
			<Icon name={STICKERS[e].id} size={20} />
		</button>
	{/each}
</div>

<style>
	/* Six small keys in one row, edge to edge: a pad, not a sticker sheet. */
	.pad {
		display: grid;
		grid-template-columns: repeat(6, minmax(0, 1fr));
		gap: 6px;
		margin: 16px 4px 0;
	}
	.r {
		height: 44px;
		color: var(--ink-2);
	}
	@media (hover: hover) {
		.r:hover {
			color: var(--ink);
		}
	}
	.r.flash {
		background: var(--accent);
		color: var(--on-accent);
		transition: none;
	}
</style>
