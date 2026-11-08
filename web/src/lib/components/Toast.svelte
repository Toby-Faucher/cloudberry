<script lang="ts">
	import { fade } from 'svelte/transition';
	import Icon from '#lib/icons/Icon.svelte';
	import { smearAnimate, wiggle } from '#lib/motion/motion.ts';
	import type { Station } from '#lib/station.svelte.ts';
	import { dur } from '#lib/util.ts';

	let { station }: { station: Station } = $props();
	const t = $derived(station.toast);

	/** Toasts slide up from the bottom edge (smeared); a poke buzzes on arrival. */
	function arrive(el: HTMLElement, kind: string | undefined) {
		smearAnimate(el, { y: [48, 0], opacity: [0, 1], duration: 240, ease: 'outQuart' }).then(
			() => void (kind === 'poke' && wiggle(el))
		);
	}
</script>

<div class="region" role="status" aria-live="polite">
	{#if t}
		{#key t.id}
			<div class="toast" class:poke={t.kind === 'poke'} use:arrive={t.kind} out:fade={{ duration: dur(160) }}>
				{#if t.kind === 'poke'}<Icon name="poke" size={18} />{/if}
				<span class="text">{t.text}</span>
				{#if t.undo}
					<button
						class="undo"
						onclick={() => {
							t.undo?.();
							station.dismissToast(t.id);
						}}
					>
						<Icon name="undo" size={16} /> Undo
					</button>
				{/if}
			</div>
		{/key}
	{/if}
</div>

<style>
	.region {
		position: fixed;
		left: 16px;
		right: 16px;
		bottom: max(24px, env(safe-area-inset-bottom));
		pointer-events: none;
		z-index: 70;
		display: grid;
		justify-items: center;
	}
	.toast {
		grid-area: 1 / 1;
		min-width: 0;
		max-width: 100%;
		display: flex;
		align-items: center;
		gap: 12px;
		min-height: 48px;
		padding: 6px 6px 6px 16px;
		border-radius: var(--r-md);
		background: var(--ink);
		color: var(--surface);
		font-size: 0.875rem;
		font-weight: 500;
		box-shadow: var(--shadow-lift);
		pointer-events: auto;
	}
	.toast.poke {
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 600;
	}
	/* Above the phone's bottom transport bar. */
	@media (max-width: 560px) {
		.region {
			bottom: calc(92px + env(safe-area-inset-bottom));
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
		height: 36px;
		padding: 0 12px;
		border-radius: var(--r-sm);
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.6875rem;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--ink);
		background: var(--accent);
		transition: background-color 120ms var(--ease-out);
	}
	@media (hover: hover) {
		.undo:hover {
			background: var(--accent-hover);
		}
	}
</style>
