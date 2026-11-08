<script lang="ts">
	import { fade } from 'svelte/transition';
	import { Hand, Undo2 } from '@lucide/svelte';
	import { smearAnimate, wiggle } from '#lib/motion/motion.ts';
	import type { Station } from '#lib/station.svelte.ts';
	import { dur } from '#lib/util.ts';

	let { station }: { station: Station } = $props();
	const t = $derived(station.toast);

	/** Toasts fling up from the bottom edge (smeared); a poke wiggles on arrival. */
	function arrive(el: HTMLElement, kind: string | undefined) {
		smearAnimate(el, { y: [90, 0], scale: [0.85, 1], opacity: [0, 1], duration: 520, ease: 'outBack(1.6)' }).then(
			() => void (kind === 'poke' && wiggle(el))
		);
	}
</script>

<div class="region" role="status" aria-live="polite">
	{#if t}
		{#key t.id}
			<div class="toast" class:poke={t.kind === 'poke'} use:arrive={t.kind} out:fade={{ duration: dur(160) }}>
				{#if t.kind === 'poke'}<Hand size={18} />{/if}
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
		pointer-events: none;
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
		font-size: 0.92rem;
		font-weight: 550;
		box-shadow: var(--shadow-pop);
		pointer-events: auto;
	}
	.toast.poke {
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 650;
	}
	/* Above the phone's bottom transport bar. */
	@media (max-width: 560px) {
		.region {
			bottom: calc(96px + env(safe-area-inset-bottom));
		}
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
		/* The toast is inverted (ink on light theme), so the accent text flips too. */
		color: var(--berry-400);
		background: color-mix(in oklab, var(--bg) 10%, transparent);
		transition:
			background-color 160ms var(--ease-out),
			transform 120ms var(--ease-out);
	}
	@media (prefers-color-scheme: dark) {
		/* Toast is light here, so the orange has to be the deep one. */
		.undo {
			color: var(--berry-700);
		}
	}
	.undo:hover {
		background: color-mix(in oklab, var(--bg) 18%, transparent);
	}
</style>
