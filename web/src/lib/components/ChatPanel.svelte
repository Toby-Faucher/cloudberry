<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { CHAT_MAX, type Station } from '#lib/station.svelte.ts';
	import type { ChatMessage } from '#lib/types.ts';
	import { collapsePresence, type ChatItem, type PendingChat } from '#lib/chat.ts';
	import { ui } from '#lib/ui.svelte.ts';
	import { dur } from '#lib/util.ts';
	import { motion, pop, smearAnimate } from '#lib/motion/motion.ts';
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

	const clock = new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit', hourCycle: 'h23' }); // a log reads in 24 h
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

	// Messages landing in the same frame (a burst) step in one after another.
	let burst = 0;
	function nextDelay() {
		if (burst === 0) requestAnimationFrame(() => (burst = 0));
		return Math.min(burst++, 6) * 45;
	}

	/** A line prints in from the sender's side: short, straight, no overshoot. */
	function arrive(el: HTMLElement, mine: boolean) {
		if (!ready) return;
		motion(el, { x: [mine ? 14 : -14, 0], opacity: [0, 1], duration: 200, delay: nextDelay(), ease: 'outQuart' });
	}
	/** Ours: a fresh send slides up out of the input; one from another tab of ours arrives normally. */
	function mineIn(el: HTMLElement, sending: boolean) {
		if (sending) sendPop(el);
		else arrive(el, true);
	}
	/** Event lines stay quiet: they just print. */
	function quiet(el: HTMLElement) {
		if (!ready) return;
		motion(el, { opacity: [0, 1], duration: 160, delay: nextDelay(), ease: 'linear' });
	}
	/** Sending: the line travels up from the input into the log, smeared on the way. */
	function sendPop(el: HTMLElement) {
		const from = input?.getBoundingClientRect();
		const to = el.getBoundingClientRect();
		const dy = from ? from.top + from.height / 2 - (to.top + to.height / 2) : 40;
		smearAnimate(el, { y: [dy, 0], opacity: [0.4, 1], duration: 260, ease: 'outQuart' });
	}
	/** "N new" strip: slides up, then ticks each time the count goes up. */
	function pillIn(el: HTMLElement) {
		motion(el, { y: [12, 0], opacity: [0, 1], duration: 180, ease: 'outQuart' });
	}
	let pill = $state<HTMLElement>();
	let lastUnseen = 0;
	$effect(() => {
		if (unseen > lastUnseen && lastUnseen > 0) pop(pill, 0.06);
		lastUnseen = unseen;
	});

	function submit(e: SubmitEvent) {
		e.preventDefault();
		if (!station.sendChat(draft)) return;
		draft = '';
		// The field dips like a key as the line leaves it.
		motion(compose, { y: [2, 0], duration: 140, ease: 'outQuart' });
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
				{@const cont = !!prev?.mine && (!m || !prevMsg || continues(m, prevMsg))}
				<div
					class="line mine"
					class:cont
					class:pending={row.pend && !row.pend.failed}
					class:failed={row.pend?.failed}
					class:echoed={row.echoed}
					use:mineIn={!!row.pend}
				>
					{#if m && !cont}<time datetime={new Date(m.at).toISOString()}>{time(m.at)}</time>{:else}<span class="t"></span>{/if}
					<span class="who"><span class="tick" aria-hidden="true"></span>{cont ? '' : 'You'}</span>
					<p class="text">
						{row.text}
						{#if row.pend?.failed}
							<button class="retry" onclick={() => row.pend && station.retryChat(row.pend.key)}>Didn't send. Try again</button>
						{:else if row.pend}
							<span class="sr-only">Sending</span>
						{/if}
					</p>
				</div>
			{:else if row.item?.kind === 'presence'}
				{@const item = row.item}
				<p class="line event" use:quiet>
					<time datetime={new Date(item.at).toISOString()}>{time(item.at)}</time>
					<span class="text">{item.text}</span>
				</p>
			{:else if row.item?.m.kind === 'event'}
				{@const m = row.item.m}
				<p class="line event" use:quiet>
					<time datetime={new Date(m.at).toISOString()}>{time(m.at)}</time>
					<span class="text">{m.name} {m.text}</span>
				</p>
			{:else if row.item && continues(row.item.m, prevMsg)}
				{@const m = row.item.m}
				<div class="line theirs cont" use:arrive={false}>
					<span class="t"></span><span class="who"></span>
					<p class="text">{m.text}</p>
				</div>
			{:else if row.item}
				{@const m = row.item.m}
				<div class="line theirs" use:arrive={false}>
					<time datetime={new Date(m.at).toISOString()}>{time(m.at)}</time>
					<strong class="who" title={m.name}>{m.name}</strong>
					<p class="text">{m.text}</p>
				</div>
			{/if}
		{/each}
	</div>

	{#if unseen}
		<button class="pill" bind:this={pill} use:pillIn onclick={() => toBottom(true)}>
			<Icon name="down" size={14} />
			<span class="num">{unseen}</span> new
		</button>
	{/if}

	<form class="compose" bind:this={compose} onsubmit={submit}>
		<label for="chat-input" class="sr-only">Message</label>
		<span class="prompt" aria-hidden="true">&gt;</span>
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
		<button type="submit" class="send" aria-label="Send" disabled={!draft.trim()}><Icon name="send" size={18} /></button>
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
		height: 64px;
		padding: 0 10px 0 16px;
		border-bottom: 1px solid var(--line);
	}
	header .section-title {
		margin: 0;
		padding: 0;
		border: 0;
	}
	.scroll {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		overflow-x: hidden;
		overscroll-behavior: contain;
		padding: 12px 16px;
		display: flex;
		flex-direction: column;
		scrollbar-color: var(--line-2) transparent;
	}
	/* Push a short conversation to the bottom, next to the input. */
	.scroll > :first-child {
		margin-top: auto;
	}
	p {
		margin: 0;
	}
	.hello {
		padding: 24px 0;
		color: var(--ink-2);
		font-size: 0.9375rem;
	}

	/* A log line: time | who | text, on a fixed grid so everything aligns down the page. */
	.line {
		display: grid;
		grid-template-columns: 2.9rem 4.6rem minmax(0, 1fr);
		column-gap: 8px;
		align-items: baseline;
		padding: 3px 0;
	}
	.line:not(.cont) {
		margin-top: 8px;
	}
	time,
	.t {
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.625rem;
		color: var(--ink-3);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	.who {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 0.8125rem;
		font-weight: 600;
	}
	.text {
		min-width: 0;
		font-size: 0.875rem;
		line-height: 1.4;
		overflow-wrap: anywhere;
		white-space: pre-wrap;
	}

	/* Events: one quiet mono line under the same time column. */
	.event {
		grid-template-columns: 2.9rem minmax(0, 1fr);
		margin-top: 6px;
	}
	.event .text {
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.6875rem;
		line-height: 1.5;
		color: var(--ink-2);
		white-space: normal;
	}

	/* You: an orange tick in the who column. Pending: the tick blinks hollow until the echo. */
	.tick {
		flex: none;
		width: 6px;
		height: 6px;
		background: var(--accent);
		box-shadow: inset 0 0 0 1px var(--led-rim);
	}
	.mine .who {
		color: var(--accent-ink);
	}
	.pending .tick {
		background: transparent;
		box-shadow: inset 0 0 0 1.5px var(--accent);
		animation: tx 0.5s steps(1, end) infinite;
	}
	.pending .text {
		color: var(--ink-2);
	}
	@keyframes tx {
		50% {
			box-shadow: inset 0 0 0 1.5px transparent;
		}
	}
	/* Echoed back by the server: the tick latches solid with one blink. */
	.echoed .tick {
		animation: latch 240ms steps(1, end);
	}
	@keyframes latch {
		33% {
			background: transparent;
		}
		66% {
			background: var(--accent);
		}
	}
	.failed .tick {
		background: var(--danger);
	}
	.failed .text {
		color: var(--ink-2);
		text-decoration: line-through;
		text-decoration-color: var(--danger);
	}
	.retry {
		display: block;
		margin-top: 2px;
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.6875rem;
		color: var(--danger);
		text-decoration: underline;
		text-underline-offset: 3px;
	}

	.pill {
		position: absolute;
		left: 50%;
		bottom: 76px;
		translate: -50% 0;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 28px;
		padding: 0 10px;
		border-radius: var(--r-sm);
		background: var(--ink);
		color: var(--surface);
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.6875rem;
		z-index: 1;
	}
	.compose {
		display: flex;
		align-items: center;
		gap: 4px;
		margin: 0;
		padding: 10px 10px 10px 16px;
		border-top: 1px solid var(--line);
	}
	.prompt {
		font-family: var(--font-mono);
		font-size: 0.8125rem;
		color: var(--accent-ink);
	}
	.compose input {
		flex: 1;
		min-width: 0;
		height: 40px;
		border: 0;
		background: transparent;
		padding: 0 6px;
		outline: none;
		font-size: 0.875rem;
	}
	@media (pointer: coarse) {
		.compose input {
			font-size: 16px; /* below this iOS zooms on focus */
		}
	}
	.compose input::placeholder {
		color: var(--ink-3);
	}
	.compose:focus-within {
		box-shadow: inset 0 2px 0 var(--ink);
	}
	.send {
		display: grid;
		place-items: center;
		width: 40px;
		height: 40px;
		border-radius: var(--r-md);
		background: var(--ink);
		color: var(--surface);
		transition: opacity 120ms;
	}
	.send:disabled {
		opacity: 0;
		cursor: default;
	}
</style>
