<script lang="ts">
	import { fade, fly } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { Undo2 } from '@lucide/svelte';
	import type { Station } from '#lib/station.svelte.ts';
	import { dur } from '#lib/util.ts';

	let { station }: { station: Station } = $props();
	const t = $derived(station.toast);
</script>

<div class="region" role="status" aria-live="polite">
	{#if t}
		{#key t.id}
			<div class="toast" in:fly={{ y: 16, duration: dur(260), easing: cubicOut }} out:fade={{ duration: dur(160) }}>
				<span class="text">{t.text}</span>
				{#if t.undo}
					<button
						class="undo"
						onclick={() => {
							t.undo?.();
							station.dismissToast(t.id);
						}}
					>
						<Undo2 size={15} /> Undo
					</button>
				{/if}
			</div>
		{/key}
	{/if}
</div>

<style>
	.region {
		position: fixed;
		left: 50%;
		bottom: max(24px, env(safe-area-inset-bottom));
		translate: -50% 0;
		width: max-content;
		max-width: calc(100% - 32px);
		z-index: 70;
		display: grid;
	}
	.toast {
		grid-area: 1 / 1;
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 10px 10px 10px 16px;
		border-radius: 14px;
		background: var(--ink);
		color: var(--bg);
		font-size: 0.88rem;
		box-shadow: var(--shadow);
	}
	.toast:not(:has(.undo)) {
		padding-right: 16px;
	}
	.text {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.undo {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		flex: none;
		padding: 5px 10px;
		border-radius: 9px;
		font-weight: 600;
		color: #fb8a3c;
		background: color-mix(in oklab, var(--bg) 10%, transparent);
		transition:
			background-color 160ms var(--ease-out),
			transform 120ms var(--ease-out);
	}
	@media (prefers-color-scheme: dark) {
		/* Toast is light here, so the orange has to be the deep one. */
		.undo {
			color: #b84d06;
		}
	}
	.undo:hover {
		background: color-mix(in oklab, var(--bg) 18%, transparent);
	}
	.undo:active {
		transform: scale(0.95);
	}
</style>
