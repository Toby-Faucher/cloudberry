<script lang="ts">
	import { tick } from 'svelte';
	import { fade, fly } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { ArrowDown, SendHorizontal, X } from '@lucide/svelte';
	import { CHAT_MAX, type Station } from '#lib/station.svelte.ts';
	import type { ChatMessage } from '#lib/types.ts';
	import { collapsePresence } from '#lib/chat.ts';
	import { ui } from '#lib/ui.svelte.ts';
	import { dur } from '#lib/util.ts';
	import Avatar from './Avatar.svelte';

	let { station, onclose }: { station: Station; onclose: () => void } = $props();

	const msgs = $derived(station.chat);
	// Reconnect blips hidden, runs of joins/leaves folded into one quiet line.
	const items = $derived(collapsePresence(msgs));
	let list = $state<HTMLElement>();
	let draft = $state('');
	/** Unseen messages that arrived while scrolled up. */
	let pending = $state(0);
	let atBottom = true;

	const clock = new Intl.DateTimeFormat(undefined, { hour: 'numeric', minute: '2-digit' });
	const time = (at: number) => clock.format(at);

	/** Same sender within 5 min: no repeated avatar/name. */
	const continues = (m: ChatMessage, prev: ChatMessage | undefined) =>
		m.kind === 'user' && prev?.kind === 'user' && prev.name === m.name && m.at - prev.at < 5 * 60_000;

	function nearBottom() {
		return !list || list.scrollHeight - list.scrollTop - list.clientHeight < 48;
	}
	function onscroll() {
		atBottom = nearBottom();
		if (atBottom) pending = 0;
	}
	function toBottom(smooth = false) {
		list?.scrollTo({ top: list.scrollHeight, behavior: smooth && dur(1) ? 'smooth' : 'instant' });
		pending = 0;
	}

	// Autoscroll unless the user scrolled up; then count what they're missing.
	let seenLen = 0;
	$effect.pre(() => {
		const n = msgs.length;
		const mine = msgs.at(-1)?.name === station.name && msgs.at(-1)?.kind === 'user';
		const wasAtBottom = atBottom;
		const added = Math.max(0, n - seenLen);
		const first = seenLen === 0;
		seenLen = n;
		tick().then(() => {
			if (first || wasAtBottom || mine) toBottom(!first);
			else pending += added;
		});
	});

	// Whatever is rendered here counts as seen (drives the unread badge).
	$effect(() => {
		const last = msgs.at(-1);
		if (last && last.id > ui.seenId) ui.seenId = last.id;
	});

	function submit(e: SubmitEvent) {
		e.preventDefault();
		if (station.sendChat(draft)) draft = '';
	}
	function keydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && !draft) (e.currentTarget as HTMLElement).blur();
	}
</script>

<div class="chat">
	<header>
		<h2 class="section-title">Chat</h2>
		<button class="icon-btn" aria-label="Close chat" title="Close chat" onclick={onclose}><X size={18} /></button>
	</header>

	<div class="scroll" bind:this={list} {onscroll} role="log" aria-live="polite" aria-label="Chat messages">
		{#if msgs.length === 0}
			<p class="hello">Say hi. Everyone tuned in sees it.</p>
		{/if}
		{#each items as item, i (item.key)}
			{@const prev = items[i - 1]}
			{#if item.kind === 'presence'}
				<p class="event presence" in:fade={{ duration: dur(200) }}>
					<span>{item.text}</span>
					<time datetime={new Date(item.at).toISOString()}>{time(item.at)}</time>
				</p>
			{:else if item.m.kind === 'event'}
				{@const m = item.m}
				<p class="event" in:fade={{ duration: dur(200) }}>
					<span>{m.name} {m.text}</span>
					<time datetime={new Date(m.at).toISOString()}>{time(m.at)}</time>
				</p>
			{:else if continues(item.m, prev?.kind === 'msg' ? prev.m : undefined)}
				{@const m = item.m}
				<p class="msg cont" in:fly={{ y: 6, duration: dur(220), easing: cubicOut }}>{m.text}</p>
			{:else}
				{@const m = item.m}
				<div class="msg first" in:fly={{ y: 6, duration: dur(220), easing: cubicOut }}>
					<Avatar name={m.name} size={26} me={m.name === station.name} />
					<div class="body">
						<p class="meta">
							<strong>{m.name}</strong>
							<time datetime={new Date(m.at).toISOString()}>{time(m.at)}</time>
						</p>
						<p class="text">{m.text}</p>
					</div>
				</div>
			{/if}
		{/each}
	</div>

	{#if pending}
		<button class="pill" onclick={() => toBottom(true)} transition:fly={{ y: 8, duration: dur(180), easing: cubicOut }}>
			<ArrowDown size={14} />
			{pending} new {pending === 1 ? 'message' : 'messages'}
		</button>
	{/if}

	<form class="compose" onsubmit={submit}>
		<label for="chat-input" class="sr-only">Message</label>
		<input
			id="chat-input"
			bind:value={draft}
			maxlength={CHAT_MAX}
			autocomplete="off"
			enterkeyhint="send"
			placeholder="Message the room"
			onkeydown={keydown}
		/>
		<button type="submit" class="send" aria-label="Send" disabled={!draft.trim()}><SendHorizontal size={17} /></button>
	</form>
</div>

<style>
	.chat {
		position: relative;
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px 12px 6px 18px;
	}
	header .section-title {
		margin: 0;
	}
	.scroll {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		overscroll-behavior: contain;
		padding: 8px 18px 12px;
		display: flex;
		flex-direction: column;
	}
	/* Push a short conversation to the bottom, next to the input. */
	.scroll > :first-child {
		margin-top: auto;
	}
	p {
		margin: 0;
	}
	.hello {
		color: var(--ink-3);
		font-size: 0.88rem;
		text-align: center;
		padding: 24px 0;
	}
	.event {
		display: flex;
		justify-content: space-between;
		gap: 10px;
		padding: 5px 0 5px 36px;
		color: var(--ink-3);
		font-size: 0.8rem;
		line-height: 1.35;
	}
	.event span {
		min-width: 0;
		overflow-wrap: anywhere;
	}
	time {
		flex: none;
		color: var(--ink-3);
		font-size: 0.72rem;
		font-variant-numeric: tabular-nums;
	}
	.msg.first {
		display: flex;
		gap: 10px;
		margin-top: 10px;
	}
	.body {
		min-width: 0;
		flex: 1;
	}
	.meta {
		display: flex;
		align-items: baseline;
		gap: 8px;
		font-size: 0.82rem;
	}
	.meta strong {
		font-weight: 600;
	}
	.text,
	.cont {
		font-size: 0.92rem;
		line-height: 1.45;
		overflow-wrap: anywhere;
		white-space: pre-wrap;
	}
	.cont {
		padding: 2px 0 0 36px;
	}
	.pill {
		position: absolute;
		left: 50%;
		bottom: 72px;
		translate: -50% 0;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 6px 12px;
		border-radius: 99px;
		background: var(--accent);
		color: var(--on-accent);
		font-size: 0.8rem;
		font-weight: 600;
		box-shadow: var(--shadow);
		z-index: 1;
	}
	.compose {
		display: flex;
		gap: 6px;
		margin: 6px 12px 12px;
		padding: 5px;
		border-radius: 16px;
		background: var(--surface-2);
		border: 3px solid transparent;
		transition: border-color 160ms var(--ease-out);
	}
	.compose:focus-within {
		border-color: var(--accent-line);
		background: var(--surface);
	}
	.compose input {
		flex: 1;
		min-width: 0;
		border: 0;
		background: transparent;
		padding: 8px 10px;
		outline: none;
	}
	@media (pointer: coarse) {
		.compose input {
			font-size: 16px; /* below this iOS zooms on focus */
		}
	}
	.compose input::placeholder {
		color: var(--ink-3);
	}
	.send {
		display: grid;
		place-items: center;
		width: 40px;
		border-radius: 12px;
		background: var(--accent);
		color: var(--on-accent);
		transition:
			opacity 160ms,
			transform 120ms var(--ease-out);
	}
	.send:disabled {
		opacity: 0;
		transform: scale(0.8);
		cursor: default;
	}
</style>
