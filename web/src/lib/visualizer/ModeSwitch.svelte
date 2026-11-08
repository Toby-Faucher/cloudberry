<script lang="ts">
	import { MODES, setLean, setMode, viz } from './state.svelte.ts';

	const labels = { halo: 'Halo', bloom: 'Bloom', bars: 'Bars' };
	const idx = $derived(MODES.indexOf(viz.mode));
</script>

<div class="row">
	<div class="seg" role="group" aria-label="Visualizer style">
		<span class="thumb" aria-hidden="true" style:transform="translateX({idx * 100}%)"></span>
		{#each MODES as m (m)}
			<button aria-pressed={viz.mode === m} onclick={() => setMode(m)}>{labels[m]}</button>
		{/each}
	</div>
	<button class="lean" onclick={() => setLean(true)} aria-label="Lean back (F)" title="Lean back (F)">
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
			<path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7" />
		</svg>
	</button>
</div>

<style>
	.row {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
	}
	.seg {
		position: relative;
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		padding: 3px;
		border-radius: 99px;
		background: var(--surface-2);
		font-size: 0.78rem;
	}
	.thumb {
		position: absolute;
		top: 3px;
		bottom: 3px;
		left: 3px;
		width: calc((100% - 6px) / 3);
		border-radius: 99px;
		background: var(--surface);
		box-shadow: 0 1px 3px rgb(60 30 10 / 0.12);
		transition: transform 220ms var(--ease-out);
	}
	.seg button {
		position: relative;
		min-width: 58px;
		padding: 4px 12px;
		border-radius: 99px;
		color: var(--ink-3);
		font-weight: 500;
		transition: color 160ms ease;
	}
	.seg button[aria-pressed='true'] {
		color: var(--ink);
	}
	@media (hover: hover) and (pointer: fine) {
		.seg button:hover {
			color: var(--ink-2);
		}
		.lean:hover {
			color: var(--ink);
			background: var(--surface-2);
		}
	}
	.lean {
		width: 30px;
		height: 30px;
		display: grid;
		place-items: center;
		border-radius: 50%;
		color: var(--ink-3);
		transition:
			color 160ms ease,
			background-color 160ms ease,
			transform 120ms var(--ease-out);
	}
	.lean:active {
		transform: scale(0.94);
	}
	@media (prefers-reduced-motion: reduce) {
		.thumb {
			transition: none;
		}
	}
</style>
