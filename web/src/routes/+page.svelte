<script lang="ts">
	import { fade, fly } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { Station } from '#lib/station.svelte.ts';
	import { dur } from '#lib/util.ts';
	import NowPlaying from '#lib/components/NowPlaying.svelte';
	import AddLink from '#lib/components/AddLink.svelte';
	import Queue from '#lib/components/Queue.svelte';
	import Library from '#lib/components/Library.svelte';
	import Listeners from '#lib/components/Listeners.svelte';
	import Berry from '#lib/components/Berry.svelte';
	import History from '#lib/components/History.svelte';
	import HeaderTools from '#lib/components/HeaderTools.svelte';
	import ChatDock from '#lib/components/ChatDock.svelte';
	import Toast from '#lib/components/Toast.svelte';
	import Shortcuts from '#lib/components/Shortcuts.svelte';
	import Integrations from '#lib/components/Integrations.svelte';
	import { ui } from '#lib/ui.svelte.ts';

	const NAME_KEY = 'cloudberry.name';
	let name = $state(localStorage.getItem(NAME_KEY) ?? '');
	let draft = $state(localStorage.getItem(NAME_KEY) ?? '');
	let station = $state<Station | null>(null);

	const validName = (v: string) => v.trim().length >= 1 && v.trim().length <= 32;

	function saveName(e: SubmitEvent) {
		e.preventDefault();
		if (!validName(draft)) return;
		name = draft.trim();
		localStorage.setItem(NAME_KEY, name);
	}

	// Must run inside the click handler: browsers only unlock audio on a user gesture.
	function tuneIn() {
		station = new Station(name);
		// Handy in devtools: cb.player.lastDrift, cb.clock.offset
		if (import.meta.env.DEV) Object.assign(window, { cb: station });
	}

	function changeName() {
		station?.close();
		station = null;
		draft = name;
		name = '';
	}

	const statusLabel = { connecting: 'Connecting', live: 'Live', reconnecting: 'Reconnecting' };
</script>

{#if !station}
	<main class="gate">
		<Berry />
		<h1 class="wordmark">cloudberry</h1>

		{#if !name}
			<form class="name-form" onsubmit={saveName} in:fly={{ y: 8, duration: dur(320), easing: cubicOut }}>
				<label for="name">What should the room call you?</label>
				<div class="field">
					<!-- svelte-ignore a11y_autofocus -->
					<input id="name" bind:value={draft} maxlength="32" autocomplete="nickname" placeholder="Your name" autofocus />
					<button type="submit" disabled={!validName(draft)}>Continue</button>
				</div>
			</form>
		{:else}
			<div class="tune" in:fly={{ y: 8, duration: dur(320), easing: cubicOut }}>
				<button class="tune-btn" onclick={tuneIn}>
					<span class="ring" aria-hidden="true"></span>
					Tune in
				</button>
				<p class="as">
					Listening as <strong>{name}</strong> · <button class="link" onclick={changeName}>change</button>
				</p>
			</div>
		{/if}
	</main>
{:else}
	<div class="shell" class:docked={ui.wide && ui.dock} in:fade={{ duration: dur(400) }}>
		<div class="app">
			<header>
				<div class="brand">
					<span class="dot" aria-hidden="true"></span>
					<span class="wordmark small">cloudberry</span>
				</div>
				<div class="head-right">
					<Listeners {station} />
					<HeaderTools {station} />
					<span class="status {station.status}" role="status" title="Connection: {statusLabel[station.status]}">
						<span class="pip"></span><span class="status-text">{statusLabel[station.status]}</span>
					</span>
					<button class="link" onclick={changeName}>{name}</button>
				</div>
			</header>

			<main class="grid">
				<div class="left">
					<NowPlaying {station} />
				</div>
				<div class="right">
					<AddLink {station} />
					<Queue {station} />
					<History {station} />
					<Library {station} />
				</div>
			</main>
		</div>
		<ChatDock {station} />
		<Toast {station} />
		<Shortcuts {station} />
		<Integrations {station} />
	</div>
{/if}

<style>
	/* ---------- gate ---------- */
	.gate {
		min-height: 100dvh;
		display: grid;
		place-content: center;
		justify-items: center;
		gap: 18px;
		padding: 24px;
		text-align: center;
	}
	.wordmark {
		font-family: var(--font-display);
		font-weight: 700;
		font-size: clamp(2.6rem, 2rem + 3vw, 4rem);
		letter-spacing: -0.045em;
		line-height: 1;
	}
	.wordmark.small {
		font-size: 1.35rem;
		letter-spacing: -0.035em;
	}

	.name-form {
		display: grid;
		gap: 12px;
		margin-top: 18px;
		width: min(100%, 360px);
	}
	.name-form label {
		color: var(--ink-2);
	}
	.field {
		display: flex;
		gap: 6px;
		padding: 6px;
		border-radius: 18px;
		background: var(--surface);
		border: 1px solid var(--line);
		box-shadow: var(--shadow);
	}
	.field:focus-within {
		border-color: color-mix(in oklab, var(--accent) 55%, var(--line));
	}
	.field input {
		flex: 1;
		min-width: 0;
		border: 0;
		background: transparent;
		padding: 10px 12px;
		outline: none;
	}
	.field button {
		padding: 0 16px;
		border-radius: 12px;
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 600;
		transition: opacity 160ms, transform 120ms var(--ease-out);
	}
	.field button:disabled {
		opacity: 0.35;
		cursor: default;
	}
	.field button:active:not(:disabled) {
		transform: scale(0.96);
	}

	.tune {
		display: grid;
		justify-items: center;
		gap: 22px;
		margin-top: 28px;
	}
	.tune-btn {
		position: relative;
		width: 168px;
		height: 168px;
		border-radius: 50%;
		background: var(--accent);
		color: var(--on-accent);
		font-family: var(--font-display);
		font-size: 1.4rem;
		font-weight: 600;
		letter-spacing: -0.02em;
		box-shadow: 0 24px 60px -20px var(--accent);
		transition: transform 160ms var(--ease-out);
	}
	.tune-btn:active {
		transform: scale(0.96);
	}
	.ring {
		position: absolute;
		inset: 0;
		border-radius: 50%;
		border: 2px solid var(--accent);
		animation: ripple 2.6s var(--ease-out) infinite;
		pointer-events: none;
	}
	@keyframes ripple {
		from {
			transform: scale(1);
			opacity: 0.6;
		}
		to {
			transform: scale(1.55);
			opacity: 0;
		}
	}
	.as {
		margin: 0;
		color: var(--ink-2);
		font-size: 0.9rem;
	}
	.link {
		color: var(--ink-2);
		text-decoration: underline;
		text-decoration-color: var(--line);
		text-underline-offset: 3px;
	}
	.link:hover {
		color: var(--ink);
	}

	/* ---------- app ---------- */
	.shell.docked {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 340px;
		align-items: start;
	}
	.app {
		max-width: 1180px;
		margin: 0 auto;
		padding: 20px 32px 64px;
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		padding: 4px 0 28px;
	}
	.brand {
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.brand .dot {
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: var(--accent);
		box-shadow: 0 0 calc(4px + var(--energy) * 18px) var(--accent);
	}
	.head-right {
		display: flex;
		align-items: center;
		gap: 18px;
		font-size: 0.85rem;
	}
	.status {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		color: var(--ink-2);
	}
	.pip {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--ink-3);
	}
	.status.live .pip {
		background: #22a35a;
	}
	.status.reconnecting .pip {
		background: var(--accent);
		animation: blink 1s ease-in-out infinite;
	}
	@keyframes blink {
		50% {
			opacity: 0.25;
		}
	}

	.grid {
		display: grid;
		grid-template-columns: minmax(340px, 420px) 1fr;
		gap: 40px;
		align-items: start;
	}
	.left {
		position: sticky;
		top: 20px;
	}
	.right {
		display: grid;
		gap: 36px;
		min-width: 0;
	}

	@media (max-width: 880px) {
		.grid {
			grid-template-columns: 1fr;
			gap: 32px;
		}
		.left {
			position: static;
			max-width: 520px;
			width: 100%;
			margin: 0 auto;
		}
	}
	@media (max-width: 560px) {
		.app {
			padding: 14px 16px 48px;
		}
		header {
			padding-bottom: 18px;
		}
		.head-right {
			gap: 12px;
		}
		.status-text {
			display: none;
		}
	}
</style>
