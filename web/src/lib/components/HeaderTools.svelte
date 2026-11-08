<script lang="ts">
	import { Tooltip } from 'bits-ui';
	import { scale } from 'svelte/transition';
	import { backOut } from 'svelte/easing';
	import { Bell, BellOff, Keyboard, MessageCircle } from '@lucide/svelte';
	import type { Station } from '#lib/station.svelte.ts';
	import { chatVisible, store, toggleChat, ui } from '#lib/ui.svelte.ts';
	import { dur } from '#lib/util.ts';

	/** Header icon buttons: notifications, shortcuts help, chat toggle (with unread badge). */
	let { station }: { station: Station } = $props();

	const canNotify = typeof Notification !== 'undefined';
	const unread = $derived(
		chatVisible() ? 0 : station.chat.filter((m) => m.id > ui.seenId && m.kind === 'user' && m.name !== station.name).length
	);

	// Permission is only ever requested here, on an explicit toggle.
	async function toggleNotify() {
		if (ui.notify) return store('cloudberry.notify', (ui.notify = false));
		let p = Notification.permission;
		if (p === 'default') p = await Notification.requestPermission();
		if (p === 'granted') {
			store('cloudberry.notify', (ui.notify = true));
			station.showToast("We'll ping you when a track you added starts");
		} else station.showToast('Notifications are blocked for this site in your browser settings');
	}
</script>

{#snippet tip(label: string)}
	<Tooltip.Portal>
		<Tooltip.Content sideOffset={8} class="cb-tip">{label}</Tooltip.Content>
	</Tooltip.Portal>
{/snippet}

<div class="tools">
	{#if canNotify}
		<Tooltip.Root>
			<Tooltip.Trigger
				class={['icon-btn', 'desk', ui.notify && 'on']}
				aria-label="Notify me when my tracks start"
				aria-pressed={ui.notify}
				onclick={toggleNotify}
			>
				{#if ui.notify}<Bell size={18} />{:else}<BellOff size={18} />{/if}
			</Tooltip.Trigger>
			{@render tip(ui.notify ? 'Notifying you when your tracks start' : 'Notify me when my tracks start')}
		</Tooltip.Root>
	{/if}
	<Tooltip.Root>
		<Tooltip.Trigger class="icon-btn desk" aria-label="Keyboard shortcuts" onclick={() => (ui.help = true)}>
			<Keyboard size={18} />
		</Tooltip.Trigger>
		{@render tip('Shortcuts (?)')}
	</Tooltip.Root>
	<Tooltip.Root>
		<Tooltip.Trigger
			class={['icon-btn', 'chat-btn', chatVisible() && 'on']}
			aria-label={unread ? `Chat, ${unread} unread` : 'Chat'}
			aria-expanded={chatVisible()}
			onclick={toggleChat}
		>
			<MessageCircle size={18} />
			{#if unread}
				<span class="badge num" in:scale={{ start: 0.4, duration: dur(260), easing: backOut }}>{unread > 9 ? '9+' : unread}</span>
			{/if}
		</Tooltip.Trigger>
		{@render tip(chatVisible() ? 'Hide chat (/)' : 'Chat (/)')}
	</Tooltip.Root>
</div>

<style>
	.tools {
		display: flex;
		align-items: center;
		gap: 2px;
	}
	.tools :global(.icon-btn.on) {
		color: var(--accent-ink);
	}
	.tools :global(.chat-btn) {
		position: relative;
		width: 36px;
		height: 36px;
	}
	.badge {
		position: absolute;
		top: -1px;
		right: -2px;
		min-width: 17px;
		height: 17px;
		padding: 0 4px;
		border-radius: 99px;
		display: grid;
		place-items: center;
		font-size: 0.66rem;
		font-weight: 700;
		background: var(--accent);
		color: var(--on-accent);
		box-shadow: 0 0 0 2px var(--bg);
	}
	/* No keyboard / desktop notifications to speak of on phones (and no room). */
	@media (pointer: coarse), (max-width: 560px) {
		.tools :global(.desk) {
			display: none;
		}
	}
</style>
