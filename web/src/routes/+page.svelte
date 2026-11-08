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
	import { PHONE_QUERY, ui } from '#lib/ui.svelte.ts';
	import { LeanBack } from '#lib/visualizer/index.ts';
	import Hud from '#lib/components/Hud.svelte';
	import TitleCard from '#lib/components/TitleCard.svelte';
	import { installPress, motion } from '#lib/motion/motion.ts';
	import { Pencil } from '@lucide/svelte';
	import { onMount } from 'svelte';

	onMount(() => {
		const phone = matchMedia(PHONE_QUERY);
		const onPhone = () => (ui.phone = phone.matches);
		phone.addEventListener('change', onPhone);
		const uninstall = installPress();
		return () => {
			phone.removeEventListener('change', onPhone);
			uninstall();
		};
	});

	/** Tune-in → app: the HUD drops in, then the card, then the lists, staggered. */
	function entrance(root: HTMLElement) {
		root.querySelectorAll<HTMLElement>('[data-enter]').forEach((el, i) =>
			motion(el, { y: [28, 0], scale: [0.97, 1], opacity: [0, 1], duration: 700, delay: 60 + i * 70 })
		);
	}

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

</script>

{#if !station}
	<main class="gate">
		<Berry bead={34} />
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
	<LeanBack {station} />
	<div class="shell" class:docked={ui.wide && ui.dock} in:fade={{ duration: dur(400) }}>
		<div class="app" use:entrance>
			<header>
				<div class="brand" data-enter>
					<Berry bead={10} />
					<span class="wordmark small">cloudberry</span>
				</div>
				<div class="hud-slot" data-enter><Hud {station} /></div>
				<div class="head-right" data-enter>
					<Listeners {station} max={ui.wide ? 6 : ui.phone ? 2 : 4} />
					<HeaderTools {station} />
					<button class="me-btn" onclick={changeName} aria-label="You are {name}. Change name">
						<span class="me-name">{name}</span><Pencil size={14} />
					</button>
				</div>
			</header>

			<main class="grid">
				<div class="left" data-enter>
					<NowPlaying {station} />
				</div>
				<div class="right">
					<div data-enter><AddLink {station} /></div>
					<div data-enter><Queue {station} /></div>
					<div data-enter><History {station} /></div>
					<div data-enter><Library {station} /></div>
				</div>
			</main>
		</div>
		<ChatDock {station} />
		<Toast {station} />
		<Shortcuts {station} />
		<Integrations {station} />
		<TitleCard {station} />
	</div>
{/if}
<canvas class="rain" id="berry-rain" aria-hidden="true"></canvas>

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
		font-size: clamp(3rem, 2.2rem + 3.6vw, 4.8rem);
		letter-spacing: -0.02em;
		line-height: 1;
	}
	.wordmark.small {
		font-size: 1.6rem;
		letter-spacing: -0.015em;
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
		padding: 0 18px;
		min-height: 44px;
		border-radius: 12px;
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 650;
		transition:
			opacity 160ms,
			background-color 160ms var(--ease-out);
	}
	.field button:hover:not(:disabled) {
		background: var(--accent-hover);
	}
	.field button:disabled {
		opacity: 0.4;
		cursor: not-allowed;
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
		font-size: 1.5rem;
		font-weight: 700;
		letter-spacing: -0.02em;
		box-shadow: var(--lip), var(--die-cut), var(--shadow-pop);
		transition: background-color 160ms var(--ease-out);
	}
	.tune-btn:hover {
		background: var(--accent-hover);
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
	.shell {
		position: relative;
	}
	.shell.docked {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 340px;
		align-items: start;
	}
	.app {
		max-width: 1180px;
		min-width: 0; /* never let the header chips widen the column */
		margin: 0 auto;
		padding: 20px 32px 64px;
	}
	header {
		display: flex;
		align-items: center;
		gap: 16px;
		padding: 4px 0 28px;
	}
	.brand {
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.hud-slot {
		flex: 1;
		min-width: 0;
		display: flex;
		overflow: hidden; /* chips truncate; they never push the layout wider */
		padding: 4px 0; /* room for the countdown pulse */
	}
	.head-right {
		display: flex;
		align-items: center;
		gap: 14px;
		font-size: 0.85rem;
	}
	.me-btn {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 36px;
		padding: 0 12px;
		border-radius: 999px;
		color: var(--ink-2);
		font-weight: 600;
		transition:
			background-color 160ms var(--ease-out),
			color 160ms var(--ease-out);
	}
	.me-btn:hover {
		background: var(--surface-2);
		color: var(--ink);
	}
	.me-name {
		max-width: 12ch;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.rain {
		position: fixed;
		inset: 0;
		width: 100%;
		height: 100%;
		z-index: 95;
		pointer-events: none;
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
	@media (max-width: 880px) {
		header {
			flex-wrap: wrap;
		}
		.hud-slot {
			order: 3;
			flex-basis: 100%;
		}
		.head-right {
			margin-left: auto;
		}
	}
	@media (max-width: 560px) {
		.app {
			padding: 14px 16px calc(112px + env(safe-area-inset-bottom)); /* room for the transport bar */
		}
		header {
			gap: 12px;
			padding-bottom: 16px;
		}
		.head-right {
			gap: 6px;
		}
		.me-name {
			display: none; /* icon-only here; the label still names it */
		}
		.me-btn {
			width: 36px;
			justify-content: center;
			padding: 0;
		}
		.wordmark.small {
			font-size: 1.4rem;
		}
		header :global(.chat-btn) {
			width: 36px;
		}
		.grid {
			gap: 24px;
		}
	}
</style>
