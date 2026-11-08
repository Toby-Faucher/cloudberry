<script lang="ts">
	import { SkipForward } from '@lucide/svelte';
	import type { Station } from '#lib/station.svelte.ts';
	import { pressSkip, releaseSkip, skipHold } from '#lib/hold.svelte.ts';
	import { wiggle } from '#lib/motion/motion.ts';

	/** Skip for everyone, but only after a ~600 ms hold: a ring fills berry → blush. */
	let { station, disabled = false }: { station: Station; disabled?: boolean } = $props();

	let btn = $state<HTMLButtonElement>();
	let hinting = $state(false);
	let hintTimer = 0;
	const R = 25;
	const C = 2 * Math.PI * R;

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
		class="hold"
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
		<svg viewBox="0 0 56 56" aria-hidden="true">
			<circle class="track" cx="28" cy="28" r={R} />
			<circle class="fill" cx="28" cy="28" r={R} stroke-dasharray={C} stroke-dashoffset={C * (1 - skipHold.p)} />
		</svg>
		<SkipForward size={20} fill="currentColor" />
	</button>
	<span id="skip-hint" class="hint" class:show={hinting}>Hold to skip</span>
</div>

<style>
	.wrap {
		position: relative;
		display: grid;
		place-items: center;
	}
	.hold {
		position: relative;
		width: 56px;
		height: 56px;
		border-radius: 50%;
		display: grid;
		place-items: center;
		color: var(--ink);
		background: var(--surface-2);
		touch-action: none;
		user-select: none;
		-webkit-user-select: none;
		-webkit-touch-callout: none;
		transition:
			background-color 160ms var(--ease-out),
			color 160ms var(--ease-out);
		scale: calc(1 - var(--p) * 0.08);
	}
	@media (hover: hover) {
		.hold:hover:not(:disabled) {
			background: color-mix(in oklab, var(--accent) 22%, var(--surface-2));
		}
	}
	.hold:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.hold.hot {
		color: var(--blush);
	}
	svg {
		position: absolute;
		inset: 0;
		rotate: -90deg;
		pointer-events: none;
	}
	circle {
		fill: none;
		stroke-width: 4;
	}
	.track {
		stroke: transparent;
	}
	.holding .track {
		stroke: color-mix(in oklab, var(--ink-3) 30%, transparent);
	}
	.fill {
		stroke: color-mix(in oklab, var(--blush) calc(var(--p) * 100%), var(--accent-line));
		stroke-linecap: round;
	}
	.hold:not(.holding) .fill {
		stroke: transparent;
	}
	.hint {
		position: absolute;
		bottom: calc(100% + 8px);
		left: 50%;
		translate: -50% 4px;
		padding: 5px 10px;
		border-radius: 8px;
		background: var(--ink);
		color: var(--bg);
		font-size: 0.78rem;
		font-weight: 600;
		white-space: nowrap;
		opacity: 0;
		pointer-events: none;
		transition:
			opacity 160ms var(--ease-out),
			translate 160ms var(--ease-out);
	}
	.hint.show {
		opacity: 1;
		translate: -50% 0;
	}
</style>
