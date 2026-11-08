<script lang="ts">
	import { Dialog } from 'bits-ui';
	import { fly } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import type { Station } from '#lib/station.svelte.ts';
	import { store, ui, WIDE_QUERY } from '#lib/ui.svelte.ts';
	import { dur } from '#lib/util.ts';
	import ChatPanel from './ChatPanel.svelte';

	/** Side panel on wide screens, bottom drawer on narrow ones. Same ChatPanel inside. */
	let { station }: { station: Station } = $props();

	$effect(() => {
		const mq = matchMedia(WIDE_QUERY);
		const set = () => {
			ui.wide = mq.matches;
			if (ui.wide) ui.drawer = false;
		};
		mq.addEventListener('change', set);
		return () => mq.removeEventListener('change', set);
	});
</script>

{#if ui.wide}
	{#if ui.dock}
		<aside class="dock" aria-label="Chat" transition:fly={{ x: 24, duration: dur(260), easing: cubicOut }}>
			<ChatPanel {station} onclose={() => store('cloudberry.chat', (ui.dock = false))} />
		</aside>
	{/if}
{:else}
	<Dialog.Root bind:open={ui.drawer}>
		<Dialog.Portal>
			<Dialog.Overlay class="chat-scrim" />
			<Dialog.Content class="chat-drawer" aria-label="Chat">
				<Dialog.Title class="sr-only">Chat</Dialog.Title>
				<ChatPanel {station} onclose={() => (ui.drawer = false)} />
			</Dialog.Content>
		</Dialog.Portal>
	</Dialog.Root>
{/if}

<style>
	/* A tall cream sticker stuck to the right edge of the bog. */
	.dock {
		position: sticky;
		top: 16px;
		height: calc(100dvh - 36px);
		margin: 16px 16px 20px 0;
		border-radius: var(--r-lg);
		background: var(--surface);
		box-shadow: var(--sticker);
		overflow: hidden;
	}
	:global(.chat-scrim) {
		position: fixed;
		inset: 0;
		z-index: 60;
		background: color-mix(in oklab, var(--shade) 55%, transparent);
		animation: scrim-in 200ms var(--ease-out);
	}
	:global(.chat-drawer) {
		position: fixed;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 61;
		height: min(78dvh, 640px);
		padding-bottom: env(safe-area-inset-bottom);
		border-radius: 24px 24px 0 0;
		background: var(--surface);
		box-shadow: 0 -4px 0 var(--die);
		animation: drawer-in 320ms var(--ease-out);
		outline: none;
	}
	@keyframes scrim-in {
		from {
			opacity: 0;
		}
	}
	@keyframes drawer-in {
		from {
			transform: translateY(100%);
		}
	}
	@media (min-width: 640px) {
		/* Tablets: a side sheet reads better than a tall bottom sheet. */
		:global(.chat-drawer) {
			left: auto;
			top: 0;
			width: 380px;
			height: 100dvh;
			border-radius: 24px 0 0 24px;
			box-shadow: -4px 0 0 var(--die);
			animation-name: sheet-in;
		}
	}
	@keyframes sheet-in {
		from {
			transform: translateX(100%);
		}
	}
</style>
