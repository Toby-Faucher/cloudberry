<script lang="ts">
	import Icon from '#lib/icons/Icon.svelte';
	import type { Station } from '#lib/station.svelte.ts';
	import { pressSkip, releaseSkip, skipHold } from '#lib/hold.svelte.ts';
	import { wiggle } from '#lib/motion/motion.ts';

	/** Skip for everyone, but only after a ~600 ms hold: the key fills from the bottom, berry → blush. */
	let { station, disabled = false }: { station: Station; disabled?: boolean } = $props();

	let btn = $state<HTMLButtonElement>();
	let hinting = $state(false);
	let hintTimer = 0;

	// A tap (button or N) wiggles and says how it works.
	let lastHint = skipHold.hint;
	$effect(() => {
		if (skipHold.hint === lastHint) return;
		lastHint = skipHold.hint;
		wiggle(btn);
		hinting = true;
		clearTimeout(hintTimer);
		hintTimer = window.setTimeout(() => (hinting = false), 1600);
	});

	const down = (e: PointerEvent) => {
		if (disabled || e.button !== 0) return;
		btn?.setPointerCapture(e.pointerId);
		pressSkip(station);
	};
	const key = (e: KeyboardEvent) => {
		if (e.key !== 'Enter' && e.key !== ' ') return;
		e.preventDefault();
		e.stopPropagation(); // Space must not also reach the global play/pause shortcut
		if (!e.repeat && !disabled) pressSkip(station);
	};
	const keyUp = (e: KeyboardEvent) => {
		if (e.key === 'Enter' || e.key === ' ') releaseSkip();
	};
</script>

<div class="wrap">
	<button
		bind:this={btn}
		class="hold key"
		class:hot={skipHold.p > 0.7}
		class:holding={skipHold.p > 0}
		style:--p={skipHold.p}
		aria-label="Skip for everyone (hold)"
		aria-describedby="skip-hint"
		{disabled}
		data-no-press
		onpointerdown={down}
		onpointerup={releaseSkip}
		onpointercancel={releaseSkip}
		onkeydown={key}
		onkeyup={keyUp}
		onblur={releaseSkip}
		oncontextmenu={(e) => e.preventDefault()}
	>
		<span class="fill" aria-hidden="true"></span>
		<span class="glyph"><Icon name="skip" size={20} /></span>
	</button>
	<span id="skip-hint" class="hint" class:show={hinting}>Hold to skip</span>
</div>

<style>
	.wrap {
		position: relative;
		display: grid;
		place-items: center;
	}
	/* A key that charges while held: the fill rises from the bottom edge (no easing: it is the timer). */
	.hold {
		width: 56px;
		height: 56px;
		overflow: hidden;
		touch-action: none;
		user-select: none;
		-webkit-user-select: none;
		-webkit-touch-callout: none;
	}
	.holding {
		translate: 0 1px;
		box-shadow:
			inset 0 0 0 1px var(--line),
			0 0 0 var(--key-edge);
	}
	.fill {
		position: absolute;
		inset: 0;
		transform-origin: bottom;
		transform: scaleY(var(--p));
		background: color-mix(in oklab, var(--blush) calc(var(--p) * 100%), var(--accent));
	}
	.hold:not(.holding) .fill {
		visibility: hidden;
	}
	.glyph {
		position: relative;
		display: grid;
	}
	.hot .glyph {
		color: #151515;
	}
	.hint {
		position: absolute;
		bottom: calc(100% + 8px);
		left: 50%;
		translate: -50% 2px;
		padding: 6px 8px;
		border-radius: var(--r-sm);
		background: var(--ink);
		color: var(--surface);
		font-family: var(--font-mono);
		font-stretch: 87.5%;
		font-size: 0.6875rem;
		white-space: nowrap;
		opacity: 0;
		pointer-events: none;
		transition:
			opacity 120ms var(--ease-out),
			translate 120ms var(--ease-out);
	}
	.hint.show {
		opacity: 1;
		translate: -50% 0;
	}
</style>
