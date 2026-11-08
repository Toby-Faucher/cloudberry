<script lang="ts">
	import { onMount } from 'svelte';
	import { follow, frames, reducedMotion, type Frame } from './analysis.ts';
	import { createRunner, type Runner } from './runner.ts';

	/**
	 * A shader from the pack on a canvas. `fill` covers its box (lean-back); `aura` is
	 * Bloom as a die-cut blob bleeding around its box (behind the now-playing art).
	 * `still` (e.g. the track id): under reduced motion, one fixed frame per value.
	 * Without WebGL2, a static dot grid with an orange level line that stretches with `energy`.
	 */
	let {
		shader = 'bloom',
		variant = 'fill',
		still = null
	}: { shader?: string; variant?: 'aura' | 'fill'; still?: unknown } = $props();

	let canvas = $state<HTMLCanvasElement>();
	let fallback = $state<HTMLDivElement>();
	let gl2 = $state(true);
	let runner = $state<Runner | null>(null);
	let shared: Frame | null = null;

	const redraw = () => shared && runner?.draw(shared, true);

	onMount(() => {
		// The aura is small: render it sharp so its die-cut edge doesn't go soft.
		const r = createRunner(canvas!, variant === 'aura' ? { aura: true, scale: 1 } : {});
		if (!r) {
			gl2 = false;
			return runFallback();
		}
		r.use(shader, true);
		runner = r;
		if (reducedMotion) {
			// Reduced motion: no loop, just a frame whenever the box, shader or track changes.
			const unsub = frames.subscribe((f) => (shared ??= f));
			const ro = new ResizeObserver(() => requestAnimationFrame(redraw));
			ro.observe(canvas!);
			return () => {
				unsub();
				ro.disconnect();
				r.destroy();
			};
		}
		const unsub = frames.subscribe((f) => r.draw(f));
		return () => {
			unsub();
			r.destroy();
		};
	});

	$effect(() => {
		runner?.use(shader);
		if (reducedMotion) requestAnimationFrame(redraw);
	});
	$effect(() => {
		if (!reducedMotion || !runner) return;
		// A stable "time" per track so each one gets its own still.
		const key = String(still ?? '');
		let h = 0;
		for (let i = 0; i < key.length; i++) h = (h * 31 + key.charCodeAt(i)) % 997;
		runner.setTime(20 + h);
		requestAnimationFrame(redraw);
	});

	/** No WebGL2: the level line follows energy; the grid stays put. */
	function runFallback() {
		let energy = 0;
		return frames.subscribe((f) => {
			energy = follow(energy, f.energy + 0.3 * f.beat, f.dt, 0.1, 0.5);
			fallback?.style.setProperty('--e', energy.toFixed(3));
		});
	}
</script>

{#if gl2}
	<canvas bind:this={canvas} class={variant} aria-hidden="true"></canvas>
{:else}
	<div bind:this={fallback} class="css {variant}" aria-hidden="true"></div>
{/if}

<style>
	canvas,
	.css {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}
	.aura {
		left: -30%;
		top: -30%;
		width: 160%;
		height: 160%;
	}
	/* Phone: the art is small, so the blob needs more room to read as a shape, not a smudge. */
	@media (max-width: 560px) {
		canvas.aura {
			left: -50%;
			top: -50%;
			width: 200%;
			height: 200%;
		}
	}
	/* Instrument face: a dot grid on the well, one orange level line across the middle
	   (it runs out past the art on both sides) that stretches with energy. */
	.css {
		--e: 0;
		background: radial-gradient(circle, var(--line-2) 0 1px, transparent 1.5px) center / 16px 16px var(--surface-2);
	}
	.css::after {
		content: '';
		position: absolute;
		left: 0;
		right: 0;
		top: calc(50% - 1px);
		height: 2px;
		background: var(--accent);
		transform: scaleX(calc(0.6 + 0.4 * var(--e)));
	}
	.css.aura {
		border-radius: 50%;
		overflow: hidden;
		box-shadow: inset 0 0 0 1px var(--line);
	}
</style>
