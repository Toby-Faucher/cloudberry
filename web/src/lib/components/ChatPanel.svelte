<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { CHAT_MAX, type Station } from '#lib/station.svelte.ts';
	import type { ChatMessage } from '#lib/types.ts';
	import { collapsePresence, type ChatItem, type PendingChat } from '#lib/chat.ts';
	import { ui } from '#lib/ui.svelte.ts';
	import { dur } from '#lib/util.ts';
	import { motion, pop, smearAnimate } from '#lib/motion/motion.ts';
	import Avatar from './Avatar.svelte';
	import Icon from '#lib/icons/Icon.svelte';

	let { station, onclose }: { station: Station; onclose: () => void } = $props();

	const msgs = $derived(station.chat);
	// Reconnect blips hidden, runs of joins/leaves folded into one quiet line.
	const items = $derived(collapsePresence(msgs));
	const pendingMsgs = $derived(station.pendingChat);
	let list = $state<HTMLElement>();
	let compose = $state<HTMLFormElement>();
	let input = $state<HTMLInputElement>();
	let draft = $state('');
	/** Unseen messages that arrived while scrolled up. */
	let unseen = $state(0);
	let atBottom = true;

	const clock = new Intl.DateTimeFormat(undefined, { hour: 'numeric', minute: '2-digit' });
	const time = (at: number) => clock.format(at);
	const isMine = (m: ChatMessage) => m.kind === 'user' && m.name === station.name;

	/** Same sender within 5 min: no repeated avatar/name. */
	const continues = (m: ChatMessage, prev: ChatMessage | undefined) =>
		m.kind === 'user' && prev?.kind === 'user' && prev.name === m.name && m.at - prev.at < 5 * 60_000;

	/**
	 * One keyed list for history and our pending sends. An echoed message takes over its pending
	 * bubble's key, so it is the SAME element: the send pop plays out and the bubble just settles.
	 */
	type Row =
		| { key: string; item: ChatItem; mine: boolean; text: string; pend: null; echoed: boolean }
		| { key: string; item: null; mine: true; text: string; pend: PendingChat; echoed: false };
	const rows = $derived<Row[]>([
		...items.map((item) => {
			const m = item.kind === 'msg' ? item.m : null;
			const pk = m ? station.settled.get(m.id) : undefined;
			return {
				key: pk !== undefined ? `p${pk}` : `m${item.key}`,
				item,
				mine: !!m && isMine(m),
				text: m?.text ?? '',
				pend: null,
				echoed: pk !== undefined
			};
		}),
		...pendingMsgs.map((p) => ({ key: `p${p.key}`, item: null, mine: true as const, text: p.text, pend: p, echoed: false as const }))
	]);

	function nearBottom() {
		return !list || list.scrollHeight - list.scrollTop - list.clientHeight < 48;
	}
	function onscroll() {
		atBottom = nearBottom();
		if (atBottom) unseen = 0;
	}
	function toBottom(smooth = false) {
		list?.scrollTo({ top: list.scrollHeight, behavior: smooth && dur(1) ? 'smooth' : 'instant' });
		unseen = 0;
	}

	// Autoscroll unless the user scrolled up; then count what they're missing.
	let seenLen = 0;
	let seenPending = 0;
	$effect.pre(() => {
		const n = msgs.length;
		const p = pendingMsgs.length;
		const sent = p > seenPending; // we just sent: always follow it down
		const mine = msgs.at(-1)?.name === station.name && msgs.at(-1)?.kind === 'user';
		const wasAtBottom = atBottom;
		const added = Math.max(0, n - seenLen);
		const first = seenLen === 0;
		seenLen = n;
		seenPending = p;
		tick().then(() => {
			if (first || wasAtBottom || mine || sent) toBottom(!first);
			else unseen += added;
		});
	});

	// Whatever is rendered here counts as seen (drives the unread badge).
	$effect(() => {
		const last = msgs.at(-1);
		if (last && last.id > ui.seenId) ui.seenId = last.id;
	});

	// ---------- motion ----------
	// History on open just appears; only what arrives afterwards moves.
	let ready = false;
	onMount(() => void tick().then(() => (ready = true)));

	// Messages landing in the same frame (a burst) stagger instead of all moving at once.
	let burst = 0;
	function nextDelay() {
		if (burst === 0) requestAnimationFrame(() => (burst = 0));
		return Math.min(burst++, 6) * 70;
	}

	/** A message arrives from the sender's side with a little overshoot. */
	function arrive(el: HTMLElement, mine: boolean) {
		if (!ready) return;
		const from = mine ? 56 : -56;
		motion(el, { x: [from, 0], scale: [0.92, 1], opacity: [0, 1], duration: 520, delay: nextDelay(), ease: 'outBack(1.7)' });
	}
	/** Ours: a fresh send pops out of the input; one from another tab of ours arrives normally. */
	function mineIn(el: HTMLElement, sending: boolean) {
		if (sending) sendPop(el);
		else arrive(el, true);
	}
	/** Event lines stay quiet: a fade and a small wiggle, nothing that pulls the eye. */
	function quiet(el: HTMLElement) {
		if (!ready) return;
		motion(el, { opacity: [0, 1], rotate: [0, -1.5, 1.2, -0.6, 0], duration: 520, delay: nextDelay(), ease: 'outQuad' });
	}
	/** Sending: the bubble pops out of the input, squashed and smeared, into the list. */
	function sendPop(el: HTMLElement) {
		const from = input?.getBoundingClientRect();
		const to = el.getBoundingClientRect();
		const dx = from ? from.left + from.width / 2 - (to.left + to.width / 2) : 0;
		const dy = from ? from.top + from.height / 2 - (to.top + to.height / 2) : 40;
		smearAnimate(el, {
			x: [dx * 0.6, 0],
			y: [dy, 0],
			scaleX: [0.55, 1.1, 1],
			scaleY: [1.35, 0.92, 1],
			opacity: [0, 1],
			duration: 560,
			ease: 'outBack(1.4)'
		});
	}
	/** "N new" pill: bounces in, then hops each time the count goes up. */
	function pillIn(el: HTMLElement) {
		motion(el, { y: [28, 0], scale: [0.5, 1], opacity: [0, 1], duration: 520, ease: 'outBack(2.6)' });
	}
	let pill = $state<HTMLElement>();
	let lastUnseen = 0;
	$effect(() => {
		if (unseen > lastUnseen && lastUnseen > 0) pop(pill, 0.18);
		lastUnseen = unseen;
	});

	function submit(e: SubmitEvent) {
		e.preventDefault();
		if (!station.sendChat(draft)) return;
		draft = '';
		// The input gives a little bounce as the message leaves it.
		motion(compose, { scaleX: [1, 0.96, 1.015, 1], scaleY: [1, 1.08, 0.98, 1], duration: 440, ease: 'outQuad' });
	}
	function keydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && !draft) (e.currentTarget as HTMLElement).blur();
	}
</script>

<div class="chat">
	<header>
		<h2 class="section-title">Chat</h2>
		<button class="icon-btn" aria-label="Close chat" title="Close chat" onclick={onclose}><Icon name="close" size={18} /></button>
	</header>

	<div class="scroll" bind:this={list} {onscroll} role="log" aria-live="polite" aria-label="Chat messages">
		{#if msgs.length === 0 && pendingMsgs.length === 0}
			<p class="hello">Quiet in here. Say hi.</p>
		{/if}
		{#each rows as row, i (row.key)}
			{@const prev = rows[i - 1]}
			{@const prevMsg = prev?.item?.kind === 'msg' ? prev.item.m : undefined}
			{#if row.mine}
				<!-- Pending and echoed are one element (same key, same branch): it settles, never re-enters. -->
				{@const m = row.item?.kind === 'msg' ? row.item.m : null}
				<div
					class="mine"
					class:cont={!!prev?.mine && (!m || !prevMsg || continues(m, prevMsg))}
					class:pending={row.pend && !row.pend.failed}
					class:failed={row.pend?.failed}
					class:echoed={row.echoed}
					use:mineIn={!!row.pend}
				>
					<p class="bubble">{row.text}</p>
					{#if row.pend?.failed}
						<button class="retry" onclick={() => row.pend && station.retryChat(row.pend.key)}>Didn't send. Try again</button>
					{:else if row.pend}
						<span class="sending" aria-label="Sending"><i></i><i></i><i></i></span>
					{:else if m}
						<time datetime={new Date(m.at).toISOString()}>{time(m.at)}</time>
					{/if}
				</div>
			{:else if row.item?.kind === 'presence'}
				{@const item = row.item}
				<p class="event" use:quiet>
					<span>{item.text}</span>
					<time datetime={new Date(item.at).toISOString()}>{time(item.at)}</time>
				</p>
			{:else if row.item?.m.kind === 'event'}
				{@const m = row.item.m}
				<p class="event" use:quiet>
					<span>{m.name} {m.text}</span>
					<time datetime={new Date(m.at).toISOString()}>{time(m.at)}</time>
				</p>
			{:else if row.item && continues(row.item.m, prevMsg)}
				{@const m = row.item.m}
				<div class="theirs cont" use:arrive={false}><p class="bubble">{m.text}</p></div>
			{:else if row.item}
				{@const m = row.item.m}
				<div class="theirs first" use:arrive={false}>
					<Avatar name={m.name} size={30} />
					<div class="body">
						<p class="meta">
							<strong>{m.name}</strong>
							<time datetime={new Date(m.at).toISOString()}>{time(m.at)}</time>
						</p>
						<p class="bubble">{m.text}</p>
					</div>
				</div>
			{/if}
		{/each}
	</div>

	{#if unseen}
		<button class="pill" bind:this={pill} use:pillIn onclick={() => toBottom(true)}>
			<Icon name="down" size={15} />
			{unseen} new
		</button>
	{/if}

	<form class="compose" bind:this={compose} onsubmit={submit}>
		<label for="chat-input" class="sr-only">Message</label>
		<input
			id="chat-input"
			bind:this={input}
			bind:value={draft}
			maxlength={CHAT_MAX}
			autocomplete="off"
			enterkeyhint="send"
			placeholder="Say something to the room"
			onkeydown={keydown}
		/>
		<button type="submit" class="send" aria-label="Send" disabled={!draft.trim()}><Icon name="send" size={19} /></button>
	</form>
</div>

<style>
	.chat {
		position: relative;
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		color: var(--ink);
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px 12px 6px 20px;
	}
	header .section-title {
		margin: 0;
		color: var(--ink);
	}
	.scroll {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		overflow-x: hidden;
		overscroll-behavior: contain;
		padding: 8px 16px 14px;
		display: flex;
		flex-direction: column;
		scrollbar-color: color-mix(in oklab, var(--ink-3) 45%, transparent) transparent;
	}
	/* Push a short conversation to the bottom, next to the input. */
	.scroll > :first-child {
		margin-top: auto;
	}
	p {
		margin: 0;
	}
	.hello {
		color: var(--ink-2);
		font-family: var(--font-display);
		font-size: 1.05rem;
		text-align: center;
		padding: 24px 0;
	}
	.event {
		display: flex;
		justify-content: space-between;
		gap: 10px;
		margin: 6px 0;
		padding: 0 4px 0 40px;
		color: var(--ink-3);
		font-size: 0.8rem;
		line-height: 1.35;
		transform-origin: left center;
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
	.bubble {
		width: fit-content;
		max-width: 100%;
		padding: 7px 12px 8px;
		border-radius: 16px;
		font-size: 0.92rem;
		line-height: 1.4;
		overflow-wrap: anywhere;
		white-space: pre-wrap;
	}

	/* Everyone else: avatar on the left, cream-2 bubbles. */
	.theirs .bubble {
		background: var(--surface-2);
		border-top-left-radius: 6px;
	}
	.theirs {
		transform-origin: left bottom;
	}
	.theirs.first {
		display: flex;
		gap: 10px;
		margin-top: 12px;
	}
	.theirs.cont {
		margin-top: 4px;
		padding-left: 40px;
	}
	.body {
		min-width: 0;
		flex: 1;
	}
	.meta {
		display: flex;
		align-items: baseline;
		gap: 8px;
		margin-bottom: 3px;
		font-size: 0.82rem;
	}
	.meta strong {
		font-family: var(--font-display);
		font-weight: 650;
		font-size: 0.92rem;
	}

	/* You: right side, berry bubbles. */
	.mine {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 2px;
		margin: 12px 0 0 48px;
		transform-origin: right bottom;
	}
	.mine.cont {
		margin-top: 4px;
	}
	.mine.cont time {
		display: none;
	}
	.mine .bubble {
		background: var(--accent);
		color: var(--on-accent);
		border-bottom-right-radius: 6px;
		transition:
			background-color 280ms var(--ease-out),
			color 280ms var(--ease-out);
	}
	/* The server echoed it: the pale pending bubble fills in with a small squash. */
	.mine.echoed .bubble {
		animation: settle 380ms var(--ease-out);
	}
	@keyframes settle {
		40% {
			scale: 1.05 0.94;
		}
		75% {
			scale: 0.98 1.03;
		}
	}
	/* Pending: paler until the server echoes it back. */
	.pending .bubble {
		background: var(--accent-soft);
		color: var(--ink);
		box-shadow: none;
	}
	.failed .bubble {
		background: transparent;
		outline: 2px dashed var(--danger);
		outline-offset: -2px;
	}
	.sending {
		display: flex;
		gap: 3px;
		padding: 2px 4px;
	}
	.sending i {
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: var(--ink-3);
		animation: dot 0.9s ease-in-out infinite;
	}
	.sending i:nth-child(2) {
		animation-delay: 0.15s;
	}
	.sending i:nth-child(3) {
		animation-delay: 0.3s;
	}
	@keyframes dot {
		50% {
			translate: 0 -3px;
		}
	}
	.retry {
		font-size: 0.78rem;
		font-weight: 600;
		color: var(--danger);
		text-decoration: underline;
		text-underline-offset: 3px;
	}

	.pill {
		position: absolute;
		left: 50%;
		bottom: 78px;
		margin-left: -52px;
		width: 104px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		padding: 7px 12px;
		border-radius: 12px;
		background: var(--accent);
		color: var(--on-accent);
		font-family: var(--font-display);
		font-size: 0.9rem;
		font-weight: 650;
		box-shadow:
			var(--lip),
			0 0 0 3px var(--die),
			0 5px 0 3px var(--shade-2);
		z-index: 1;
	}
	.compose {
		display: flex;
		gap: 6px;
		margin: 6px 12px 12px;
		padding: 5px;
		border-radius: 16px;
		background: var(--surface-2);
		box-shadow: inset 0 0 0 3px transparent;
		transition:
			box-shadow 160ms var(--ease-out),
			background-color 160ms var(--ease-out);
	}
	.compose:focus-within {
		box-shadow: inset 0 0 0 3px var(--accent-line);
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
		width: 42px;
		border-radius: 12px;
		background: var(--accent);
		color: var(--on-accent);
		box-shadow: var(--lip);
		transition:
			opacity 160ms,
			scale 160ms var(--ease-out);
	}
	.send:disabled {
		opacity: 0;
		scale: 0.6;
		cursor: default;
	}
</style>
