<script lang="ts">
	import { SHADERS } from './shaders/index.ts';
	import { setAuto, setShader, viz } from './state.svelte.ts';

	/** Lean-back's shader tabs (game tiles, like the mode switch) plus the auto-cycle toggle. */
</script>

<div class="picker">
	<div class="tabs" role="group" aria-label="Lean-back shader (← →)">
		{#each SHADERS as s (s.id)}
			<button class="tab" aria-pressed={viz.shader === s.id} onclick={() => setShader(s.id)} title={s.label}>
				<svg viewBox="0 0 20 20" aria-hidden="true">
					{#if s.id === 'bloom'}
						<path d="M3 15c2-6 5-1 7-5s5-6 7-3M3 10.5c2.5-4 4.5-1 6.5-4S14 2.5 17 4" fill="none" stroke="currentColor" stroke-width="1.5" />
					{:else if s.id === 'scope'}
						<path d="M1.5 10h17M10 2v16" stroke="currentColor" stroke-width="1" opacity="0.5" />
						<path d="M1.5 10c1.6-6 3.4-6 5 0s3.4 6 5 0 3.4-6 5 0" fill="none" stroke="currentColor" stroke-width="1.6" />
					{:else if s.id === 'aurora'}
						<path d="M1.5 8c4-4 7 2 10-1s5-2 7-1M1.5 11c4-4 7 2 10-1s5-2 7-1M1.5 14c4-4 7 2 10-1s5-2 7-1" fill="none" stroke="currentColor" stroke-width="1.4" />
					{:else if s.id === 'drupelets'}
						<circle cx="6.6" cy="7.4" r="3.6" fill="none" stroke="currentColor" stroke-width="1.4" />
						<circle cx="13.4" cy="7.4" r="3.6" fill="none" stroke="currentColor" stroke-width="1.4" />
						<circle cx="10" cy="13.4" r="3.6" fill="none" stroke="currentColor" stroke-width="1.4" />
					{:else if s.id === 'tunnel'}
						<circle cx="10" cy="10" r="8" fill="none" stroke="currentColor" stroke-width="1.4" />
						<circle cx="10" cy="10" r="4.6" fill="none" stroke="currentColor" stroke-width="1.4" />
						<circle cx="10" cy="10" r="1.8" fill="currentColor" />
					{:else}
						<circle cx="4" cy="4" r="2.4" fill="currentColor" />
						<circle cx="10" cy="4" r="1.6" fill="currentColor" />
						<circle cx="16" cy="4" r="0.9" fill="currentColor" />
						<circle cx="4" cy="10" r="1.6" fill="currentColor" />
						<circle cx="10" cy="10" r="2.6" fill="currentColor" />
						<circle cx="16" cy="10" r="1.6" fill="currentColor" />
						<circle cx="4" cy="16" r="0.9" fill="currentColor" />
						<circle cx="10" cy="16" r="1.6" fill="currentColor" />
						<circle cx="16" cy="16" r="2.4" fill="currentColor" />
					{/if}
				</svg>
				<span class="label">{s.label}</span>
				<span class="led" aria-hidden="true"></span>
			</button>
		{/each}
	</div>
	<button class="tab auto" aria-pressed={viz.auto} onclick={() => setAuto(!viz.auto)} title="Next shader on every new track (A)">
		<svg viewBox="0 0 20 20" aria-hidden="true">
			<path d="M16 8.5A6.3 6.3 0 0 0 4.4 6.6M4 11.5a6.3 6.3 0 0 0 11.6 1.9" fill="none" stroke="currentColor" stroke-width="1.5" />
			<path d="M3.2 2.8v4.6h4.6M16.8 17.2v-4.6h-4.6" fill="none" stroke="currentColor" stroke-width="1.5" />
		</svg>
		<span class="label">Auto</span>
		<span class="led" aria-hidden="true"></span>
	</button>
</div>

<style>
	/* Instrument key strip: hairline-ruled keys, mono small-caps labels, an orange LED on the
	   active one. Colours come from LeanBack's --lb-* tokens. */
	.picker {
		display: flex;
		align-items: stretch;
		gap: 8px;
	}
	.tabs {
		display: flex;
		border: 1px solid var(--lb-line);
		background: var(--lb-paper);
	}
	.tab {
		position: relative;
		display: inline-flex;
		align-items: center;
		gap: 7px;
		height: 38px;
		padding: 0 12px 0 10px;
		border-radius: 0;
		background: var(--lb-paper);
		color: var(--lb-ink-2);
		font: 500 0.7rem/1 var(--lb-mono);
		letter-spacing: 0.12em;
		text-transform: uppercase;
		transition:
			background-color 120ms linear,
			color 120ms linear;
	}
	.tabs .tab + .tab {
		border-left: 1px solid var(--lb-line);
	}
	.auto {
		border: 1px solid var(--lb-line);
	}
	.tab svg {
		width: 16px;
		height: 16px;
		flex: none;
	}
	.led {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		box-shadow: inset 0 0 0 1px var(--lb-ink-2);
		flex: none;
	}
	.tab[aria-pressed='true'] {
		color: var(--lb-ink);
		background: var(--lb-panel);
	}
	.tab[aria-pressed='true'] .led {
		background: var(--lb-orange);
		box-shadow: none;
	}
	.tab:active {
		translate: 0 1px;
	}
	@media (hover: hover) and (pointer: fine) {
		.tab:not([aria-pressed='true']):hover {
			color: var(--lb-ink);
		}
	}
	/* Phones: icon-only keys so all seven fit; the LED moves to the corner. */
	@media (max-width: 640px) {
		.tab {
			width: 42px;
			height: 42px;
			padding: 0;
			justify-content: center;
		}
		.tab .led {
			position: absolute;
			top: 5px;
			right: 5px;
			width: 5px;
			height: 5px;
		}
		.label {
			position: absolute;
			width: 1px;
			height: 1px;
			overflow: hidden;
			clip-path: inset(50%);
		}
	}
</style>
