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
		gap: 2px;
		margin: -6px 0 18px;
	}
	.r {
		width: 40px;
		height: 36px;
		border-radius: 12px;
		font-size: 1.15rem;
		line-height: 1;
		filter: saturate(0.85);
		transition:
			background-color 160ms var(--ease-out),
			transform 140ms var(--ease-out),
			filter 160ms;
	}
	@media (hover: hover) {
		.r:hover {
			background: var(--surface-2);
			filter: none;
			transform: translateY(-2px);
		}
	}
	.r:active {
		transform: scale(0.88);
		transition-duration: 80ms;
	}
</style>
