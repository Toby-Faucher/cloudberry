<script lang="ts">
	import { onMount } from 'svelte';
	import { follow, frames, reducedMotion, type Frame } from './analysis.ts';
	import { createRunner, type Runner } from './runner.ts';

	/**
	 * A shader from the pack on a canvas. `fill` covers its box (lean-back); `aura` is
	 * Bloom as a die-cut blob bleeding around its box (behind the now-playing art).
	 * `still` (e.g. the track id): under reduced motion, one fixed frame per value.
	 * Without WebGL2, a flat CSS gradient breathes with `energy` instead.
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

	/** No WebGL2: a flat tinted shape that swells with energy. */
	function runFallback() {
		let energy = 0;
		return frames.subscribe((f) => {
			const el = fallback;
			if (!el) return;
			energy = follow(energy, f.energy + 0.3 * f.beat, f.dt, 0.1, 0.5);
			el.style.setProperty('--tint', `rgb(${f.tint.map(Math.round).join(' ')})`);
			el.style.opacity = (0.45 + 0.55 * energy).toFixed(3);
			el.style.transform = `scale(${(0.92 + 0.14 * energy).toFixed(3)})`;
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
	.css {
		--tint: var(--berry-500);
		/* Flat tone bands, hard stops: same posterised look as the shader. */
		background:
			radial-gradient(40% 34% at 35% 40%, var(--berry-300) 0 99%, transparent 100%),
			radial-gradient(70% 60% at 65% 65%, var(--tint) 0 99%, transparent 100%);
		will-change: transform, opacity;
	}
	.css.aura {
		border-radius: 50%;
		background: radial-gradient(closest-side, var(--tint) 0 60%, transparent 61%);
	}
</style>
