<script lang="ts">
	import Icon from '#lib/icons/Icon.svelte';
	import { MODES, setLean, setMode, viz } from './state.svelte.ts';

	const labels = { halo: 'Halo', bloom: 'Bloom', bars: 'Bars' };
</script>

<div class="row">
	<div class="tabs" role="group" aria-label="Visualizer style">
		{#each MODES as m (m)}
			<button class="tab" aria-pressed={viz.mode === m} onclick={() => setMode(m)}>
				<!-- A tiny preview of what each mode draws. -->
				<svg viewBox="0 0 20 20" aria-hidden="true">
					{#if m === 'halo'}
						<circle cx="10" cy="10" r="4.5" fill="currentColor" />
						<path d="M10 1.5v2.5M10 16v2.5M1.5 10H4M16 10h2.5M4 4l1.8 1.8M14.2 14.2 16 16M4 16l1.8-1.8M14.2 5.8 16 4" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" />
					{:else if m === 'bloom'}
						<path d="M10 2.5c3 0 4 2.5 5.8 3.3 2 1 2.6 3.6 1.4 5.6-1 1.8.2 4-2 5.3-2.2 1.2-3.4-.2-5.2.3-2.3.6-4.6-.6-5.3-2.8-.6-1.8-2.6-2.7-1.9-5C3.5 6.8 5 6 6 4.4 7 2.9 8.3 2.5 10 2.5Z" fill="currentColor" />
					{:else}
						<rect x="2" y="9" width="3.4" height="9" rx="1.7" fill="currentColor" />
						<rect x="8.3" y="3" width="3.4" height="15" rx="1.7" fill="currentColor" />
						<rect x="14.6" y="7" width="3.4" height="11" rx="1.7" fill="currentColor" />
					{/if}
				</svg>
				{labels[m]}
			</button>
		{/each}
	</div>
	<button class="lean" onclick={() => setLean(true)} aria-label="Lean back (F)" title="Lean back (F)">
		<Icon name="expand" size={20} />
	</button>
</div>

<style>
	.row {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 10px;
	}
	.tabs {
		display: flex;
		gap: 6px;
	}
	/* Game tabs: raised tiles on a hard drop; the chosen one is berry and pressed down. */
	.tab {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 40px;
		padding: 0 12px 0 10px;
		border-radius: 12px;
		background: var(--surface-2);
		color: var(--ink-2);
		font-family: var(--font-display);
		font-size: 0.95rem;
		font-weight: 650;
		box-shadow: 0 4px 0 var(--shade-2);
		transition:
			translate 120ms var(--ease-out),
			box-shadow 120ms var(--ease-out),
			background-color 160ms var(--ease-out),
			color 160ms var(--ease-out);
	}
	.tab svg {
		width: 18px;
		height: 18px;
		flex: none;
	}
	.tab[aria-pressed='true'] {
		background: var(--accent);
		color: var(--on-accent);
		translate: 0 3px;
		box-shadow:
			inset 0 -3px 0 rgb(110 45 0 / 0.28),
			0 1px 0 var(--shade-2);
	}
	@media (hover: hover) and (pointer: fine) {
		.tab:not([aria-pressed='true']):hover {
			color: var(--ink);
			translate: 0 -1px;
			box-shadow: 0 5px 0 var(--shade-2);
		}
		.lean:hover {
			color: var(--ink);
			background: var(--surface-2);
		}
	}
	.lean {
		width: 40px;
		height: 40px;
		display: grid;
		place-items: center;
		border-radius: 12px;
		color: var(--ink-2);
		transition:
			color 160ms ease,
			background-color 160ms ease;
	}
</style>
