<script lang="ts">
	import type { Station } from '#lib/station.svelte.ts';
	import { REACTIONS } from '#lib/types.ts';

	let { station }: { station: Station } = $props();
	const labels: Record<string, string> = {
		'🔥': 'Fire',
		'😂': 'Laughing',
		'❤️': 'Love',
		'👏': 'Applause',
		'🎉': 'Party',
		'😴': 'Sleepy'
	};
</script>

<div class="bar" role="group" aria-label="React for everyone">
	{#each REACTIONS as e (e)}
		<button class="r" aria-label="React {labels[e]}" title={labels[e]} onclick={() => station.react(e)}>{e}</button>
	{/each}
</div>

<style>
	.bar {
		display: flex;
		justify-content: center;
		gap: 4px;
		margin: -4px 0 18px;
	}
	.r {
		width: 46px;
		height: 46px;
		border-radius: 14px;
		font-size: 1.35rem;
		line-height: 1;
		background: var(--surface-2);
		transition:
			background-color 160ms var(--ease-out),
			translate 140ms var(--ease-out);
	}
	@media (hover: hover) {
		.r:hover {
			background: var(--accent-soft);
			translate: 0 -3px;
		}
	}
</style>
