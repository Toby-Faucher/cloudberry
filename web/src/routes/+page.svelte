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
					<input id="name" bind:value={draft} maxlength="32" autocomplete="nickname" placeholder="Name or nickname" autofocus />
					<button type="submit" disabled={!validName(draft)}>That's me</button>
				</div>
			</form>
		{:else}
			<div class="tune" in:fly={{ y: 8, duration: dur(320), easing: cubicOut }}>
				<button class="tune-btn" onclick={tuneIn}>
					<span class="ring" aria-hidden="true"></span>
					Tune in
				</button>
				<p class="as">
					<span>Hey <strong>{name}</strong>.</span>
					<button class="link" onclick={changeName}>Not you?</button>
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
		color: var(--on-bg);
	}
	/* The wordmark as a sticker: berry letters inside a fat white die-cut, like the logo. */
	.wordmark {
		font-family: var(--font-display);
		font-weight: 700;
		font-size: clamp(3rem, 2.2rem + 3.6vw, 4.8rem);
		letter-spacing: -0.02em;
		line-height: 1;
		color: var(--berry-500);
		-webkit-text-stroke: 0.18em var(--die);
		paint-order: stroke fill;
		text-shadow: 0 0.12em 0 var(--shade);
	}
	.wordmark.small {
		font-size: 1.7rem;
		letter-spacing: -0.015em;
	}

	.name-form {
		display: grid;
		gap: 14px;
		margin-top: 18px;
		width: min(100%, 380px);
	}
	.name-form label {
		font-family: var(--font-display);
		font-size: 1.2rem;
		font-weight: 600;
		color: var(--on-bg);
	}
	.field {
		display: flex;
		gap: 6px;
		padding: 6px;
		border-radius: 18px;
		background: var(--surface);
		color: var(--ink);
		box-shadow: var(--sticker);
	}
	.field input {
		flex: 1;
		min-width: 0;
		border: 0;
		background: transparent;
		padding: 10px 12px;
		outline: none;
	}
	.field input::placeholder {
		color: var(--ink-3);
	}
	.field button {
		padding: 0 18px;
		min-height: 46px;
		border-radius: 12px;
		background: var(--accent);
		color: var(--on-accent);
		font-family: var(--font-display);
		font-size: 1.05rem;
		font-weight: 650;
		box-shadow: var(--lip);
		transition:
			opacity 160ms,
			background-color 160ms var(--ease-out);
	}
	.field button:hover:not(:disabled) {
		background: var(--accent-hover);
	}
	.field button:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.tune {
		display: grid;
		justify-items: center;
		gap: 26px;
		margin-top: 28px;
	}
	.tune-btn {
		position: relative;
		width: 172px;
		height: 172px;
		border-radius: 50%;
		background: var(--accent);
		color: var(--on-accent);
		font-family: var(--font-display);
		font-size: 1.6rem;
		font-weight: 700;
		letter-spacing: -0.02em;
		rotate: -4deg;
		box-shadow:
			var(--lip),
			0 0 0 6px var(--die),
			0 10px 0 6px var(--shade);
		transition:
			background-color 160ms var(--ease-out),
			translate 120ms var(--ease-out),
			box-shadow 120ms var(--ease-out);
	}
	.tune-btn:hover {
		background: var(--accent-hover);
	}
	.tune-btn:active {
		translate: 0 7px;
		box-shadow:
			var(--lip),
			0 0 0 6px var(--die),
			0 3px 0 6px var(--shade);
	}
	/* The one idle motion we keep: the gate asking to be pressed. */
	.ring {
		position: absolute;
		inset: -6px;
		border-radius: 50%;
		border: 4px solid var(--die);
		animation: ripple 2.6s var(--ease-out) infinite;
		pointer-events: none;
	}
	@keyframes ripple {
		from {
			transform: scale(1);
			opacity: 0.7;
		}
		to {
			transform: scale(1.45);
			opacity: 0;
		}
	}
	.as {
		margin: 0;
		display: flex;
		align-items: baseline;
		gap: 12px;
		color: var(--on-bg);
		font-size: 1rem;
	}
	.link {
		color: var(--on-bg-2);
		text-decoration: underline;
		text-decoration-thickness: 2px;
		text-decoration-color: color-mix(in oklab, var(--on-bg-2) 50%, transparent);
		text-underline-offset: 4px;
	}
	.link:hover {
		color: var(--on-bg);
	}

	/* ---------- app ---------- */
	.shell {
		position: relative;
		overflow-x: clip; /* the now-playing aura may bleed sideways; never a horizontal scroll */
	}
	.shell.docked {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 360px;
		align-items: start;
	}
	.app {
		max-width: 1220px;
		min-width: 0; /* never let the header widen the column */
		margin: 0 auto;
		padding: 20px 36px 72px;
	}
	header {
		position: relative;
		z-index: 4; /* above the now-playing column and its aura, which may bleed up */
		display: flex;
		align-items: center;
		gap: 18px;
		padding: 4px 0 30px;
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
		overflow: hidden; /* the countdown truncates; it never pushes the layout wider */
		padding: 8px 6px 12px; /* room for die-cuts, drops and the countdown pulse */
		margin: -8px 0 -12px;
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
		height: 40px;
		padding: 0 12px;
		border-radius: 12px;
		color: var(--on-bg);
		font-family: var(--font-display);
		font-size: 1rem;
		font-weight: 600;
		transition:
			background-color 160ms var(--ease-out),
			color 160ms var(--ease-out);
	}
	.me-btn:hover {
		background: color-mix(in oklab, var(--shade) 55%, transparent);
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

	/* Not three equal panels: one big sticker on the left, loose sections on the bog to its right. */
	.grid {
		display: grid;
		grid-template-columns: minmax(360px, 440px) 1fr;
		gap: 48px;
		align-items: start;
	}
	.left {
		position: sticky;
		top: 20px;
	}
	.right {
		position: relative;
		z-index: 1; /* above the aura's sideways bleed */
		display: grid;
		gap: 40px;
		min-width: 0;
		padding-top: 6px;
	}

	@media (max-width: 880px) {
		.grid {
			grid-template-columns: 1fr;
			gap: 36px;
		}
		.left {
			position: static;
			max-width: 520px;
			width: 100%;
			margin: 0 auto;
		}
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
		/* The fixed transport bar lives inside .now's stacking context: lift the whole column. */
		.left {
			position: relative;
			z-index: 3;
		}
	}
	@media (max-width: 560px) {
		.app {
			padding: 14px 16px calc(116px + env(safe-area-inset-bottom)); /* room for the transport bar */
		}
		header {
			gap: 12px;
			padding-bottom: 14px;
		}
		.head-right {
			gap: 8px;
		}
		.me-name {
			display: none; /* icon-only here; the label still names it */
		}
		.me-btn {
			width: 40px;
			justify-content: center;
			padding: 0;
		}
		.wordmark.small {
			font-size: 1.45rem;
		}
		.grid {
			gap: 28px;
		}
	}
</style>
