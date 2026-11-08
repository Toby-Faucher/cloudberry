<script lang="ts">
	import { fade, fly } from 'svelte/transition';
	import { quartOut } from 'svelte/easing';
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
	import Icon from '#lib/icons/Icon.svelte';
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

	/** Tune-in → app: modules power on in reading order, a short rise each, no overshoot. */
	function entrance(root: HTMLElement) {
		root.querySelectorAll<HTMLElement>('[data-enter]').forEach((el, i) =>
			motion(el, { y: [10, 0], opacity: [0, 1], duration: 360, delay: 40 + i * 45, ease: 'outQuart' })
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
		<div class="plate">
			<div class="plate-top">
				<span class="brand"><Berry bead={7} /><span class="wordmark">cloudberry</span></span>
				<span class="label state"><span class="led" class:on={!!name} class:blink={!!name}></span>{name ? 'Ready' : 'Standby'}</span>
			</div>

			{#if !name}
				<form class="name-form" onsubmit={saveName} in:fly={{ y: 6, duration: dur(240), easing: quartOut }}>
					<label for="name">What should the room call you?</label>
					<div class="field">
						<!-- svelte-ignore a11y_autofocus -->
						<input id="name" bind:value={draft} maxlength="32" autocomplete="nickname" placeholder="Name or nickname" autofocus />
						<button type="submit" class="key go" disabled={!validName(draft)}>That's me</button>
					</div>
				</form>
			{:else}
				<div class="tune" in:fly={{ y: 6, duration: dur(240), easing: quartOut }}>
					<p class="hello"><span class="label">Hey</span><span class="who">{name}</span></p>
					<button class="tune-btn" onclick={tuneIn}>
						<span class="tune-led" aria-hidden="true"></span>
						<Icon name="play" size={20} />
						Tune in
					</button>
					<button class="link" onclick={changeName}>Not you?</button>
				</div>
			{/if}
		</div>
		<p class="foot">A tiny shared radio. Everyone hears the same second.</p>
	</main>
{:else}
	<LeanBack {station} />
	<div class="shell" class:docked={ui.wide && ui.dock} in:fade={{ duration: dur(240) }}>
		<div class="app" use:entrance>
			<header>
				<div class="brand" data-enter>
					<Berry bead={7} />
					<span class="wordmark">cloudberry</span>
				</div>
				<div class="hud-slot" data-enter><Hud {station} /></div>
				<div class="head-right" data-enter>
					<Listeners {station} max={ui.wide ? 6 : ui.phone ? 2 : 4} />
					<span class="rule" aria-hidden="true"></span>
					<HeaderTools {station} />
					<button class="me-btn" onclick={changeName} aria-label="You are {name}. Change name">
						<span class="me-name">{name}</span><Icon name="pencil" size={14} />
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
	/* ---------- gate: the device on standby ---------- */
	.gate {
		min-height: 100dvh;
		display: grid;
		place-content: center;
		justify-items: center;
		gap: 16px;
		padding: 24px 16px;
	}
	.plate {
		width: min(100vw - 32px, 400px);
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: var(--r-lg);
	}
	.plate-top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px 16px;
		border-bottom: 1px solid var(--line);
	}
	.brand {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.wordmark {
		font-family: var(--font-ui);
		font-weight: 650;
		font-size: 1.0625rem;
		letter-spacing: -0.025em;
		line-height: 1;
		color: var(--ink);
	}
	.state {
		display: inline-flex;
		align-items: center;
		gap: 7px;
	}

	.name-form {
		display: grid;
		gap: 10px;
		padding: 24px 16px 16px;
	}
	.name-form label {
		font-size: 0.875rem;
		font-weight: 550;
		color: var(--ink-2);
	}
	.field {
		display: flex;
		gap: 8px;
	}
	.field input {
		flex: 1;
		min-width: 0;
		height: 44px;
		padding: 0 12px;
		border: 1px solid var(--line-2);
		border-radius: var(--r-md);
		background: var(--key);
		outline: none;
		font-size: 1rem;
		transition: border-color 120ms var(--ease-out);
	}
	.field input:focus {
		border-color: var(--ink);
	}
	.field input::placeholder {
		color: var(--ink-3);
	}
	.go {
		height: 44px;
		padding: 0 16px;
		font-weight: 600;
	}

	.tune {
		display: grid;
		gap: 16px;
		padding: 20px 16px 16px;
	}
	.hello {
		margin: 0;
		display: grid;
		gap: 10px;
	}
	/* The name on a dot-matrix display. */
	.hello .who {
		font-family: var(--font-dot);
		font-weight: 800;
		font-size: 2.75rem;
		line-height: 0.95;
		text-transform: uppercase;
		overflow-wrap: anywhere;
		color: var(--ink);
	}
	.tune-btn {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 10px;
		height: 72px;
		border-radius: var(--r-md);
		background: var(--accent);
		color: var(--on-accent);
		font-size: 1.25rem;
		font-weight: 650;
		letter-spacing: -0.02em;
		box-shadow: 0 2px 0 color-mix(in oklab, var(--accent) 55%, #000);
		transition:
			background-color 120ms var(--ease-out),
			translate 60ms linear,
			box-shadow 60ms linear;
	}
	@media (hover: hover) {
		.tune-btn:hover {
			background: var(--accent-hover);
		}
	}
	.tune-btn:active {
		translate: 0 2px;
		box-shadow: 0 0 0 color-mix(in oklab, var(--accent) 55%, #000);
	}
	/* The one idle motion we keep: a status lamp on the key, blinking until you press it. */
	.tune-led {
		position: absolute;
		top: 10px;
		right: 10px;
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--on-accent);
		animation: lamp 1.2s steps(1, end) infinite;
	}
	@keyframes lamp {
		50% {
			opacity: 0.15;
		}
	}
	.link {
		justify-self: start;
		font-size: 0.875rem;
		color: var(--ink-2);
		text-decoration: underline;
		text-decoration-thickness: 1px;
		text-underline-offset: 4px;
	}
	.link:hover {
		color: var(--ink);
	}
	.foot {
		margin: 0;
		max-width: 400px;
		text-align: center;
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.6875rem;
		color: var(--ink-3);
	}

	/* ---------- app ---------- */
	.shell {
		position: relative;
		overflow-x: clip; /* the visualizer may bleed sideways; never a horizontal scroll */
	}
	.shell.docked {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 360px;
		align-items: start;
	}
	.app {
		max-width: 1240px;
		min-width: 0;
		margin: 0 auto;
		padding: 0 32px 64px;
	}
	header {
		position: relative;
		z-index: 4;
		display: flex;
		align-items: center;
		gap: 20px;
		height: 64px;
		margin-bottom: 24px;
		border-bottom: 1px solid var(--line);
	}
	.hud-slot {
		flex: 1;
		min-width: 0;
		display: flex;
		overflow: hidden; /* the countdown truncates; it never pushes the layout wider */
	}
	.head-right {
		display: flex;
		align-items: center;
		gap: 12px;
		font-size: 0.85rem;
	}
	.rule {
		width: 1px;
		height: 24px;
		background: var(--line);
	}
	.me-btn {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		height: 36px;
		padding: 0 10px;
		border-radius: var(--r-md);
		color: var(--ink);
		font-size: 0.875rem;
		font-weight: 550;
		transition: background-color 120ms var(--ease-out);
	}
	.me-btn :global(.cb-icon) {
		color: var(--ink-3);
	}
	@media (hover: hover) {
		.me-btn:hover {
			background: var(--surface-2);
		}
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

	/* Asymmetric: the deck (one raised object) on the left, ruled lists on the chassis to its right. */
	.grid {
		display: grid;
		grid-template-columns: minmax(340px, 400px) minmax(0, 1fr);
		gap: 40px;
		align-items: start;
	}
	.left {
		position: sticky;
		top: 16px;
	}
	.right {
		position: relative;
		z-index: 1;
		display: grid;
		gap: 40px;
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
		header {
			flex-wrap: wrap;
			height: auto;
			padding: 12px 0;
			row-gap: 10px;
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
		.app {
			padding: 0 16px calc(104px + env(safe-area-inset-bottom)); /* room for the transport bar */
		}
		header {
			gap: 10px;
			margin-bottom: 16px;
		}
		.head-right {
			gap: 6px;
		}
		.rule {
			display: none;
		}
		.me-name {
			display: none; /* icon-only here; the label still names it */
		}
		.me-btn {
			width: 36px;
			justify-content: center;
			padding: 0;
		}
		.grid {
			gap: 28px;
		}
		.right {
			gap: 32px;
		}
	}
</style>
