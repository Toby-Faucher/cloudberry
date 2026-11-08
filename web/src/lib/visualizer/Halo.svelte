<script lang="ts">
	import { onMount } from 'svelte';
	import { BARS, WAVE_POINTS, fit, follow, frames, reducedMotion, type Frame } from './analysis.ts';

	/**
	 * Ring of mirrored radial bars around a round art disc. Lives inside the art's
	 * container (positioned) and bleeds HALO_BLEED past it on every side. Writes
	 * `--viz-pulse` (1..~1.03) on that container so the art can breathe on beats.
	 */
	let canvas: HTMLCanvasElement;
	let glowEl: HTMLDivElement;

	/** Art disc radius as a fraction of the container width; keep in sync with the CSS using it. */
	const ART = 0.33;
	const BLEED = 0.14;
	const R0 = ART / (1 + 2 * BLEED); // in canvas units (canvas side = 1)
	const GAP = 0.02;
	const LEN = 0.13;
	const TAU = Math.PI * 2;

	onMount(() => {
		const g = canvas.getContext('2d')!;
		const host = canvas.parentElement!;
		const ro = new ResizeObserver(() => fit(canvas));
		ro.observe(canvas);

		const smooth = new Float32Array(BARS);
		let rot = 0;
		let pulse = 0;
		let lastPulse = '';
		let lastTint = '';

		const draw = (f: Frame) => {
			const { width: W, height: H } = canvas;
			const C = Math.min(W, H);
			const cx = W / 2;
			const cy = H / 2;
			const dt = f.dt;
			const [r, gr, b] = f.tint.map(Math.round);
			const rgba = (a: number) => `rgb(${r} ${gr} ${b} / ${a.toFixed(3)})`;

			pulse = follow(pulse, f.beat * f.beat, dt, 0.035, 0.11);
			rot += dt * (0.06 + 0.22 * f.energy);
			const p = (1 + 0.028 * pulse).toFixed(4);
			if (p !== lastPulse && !reducedMotion) host.style.setProperty('--viz-pulse', (lastPulse = p));

			// Neighbour-blur the spectrum so the ring reads as one organic shape.
			for (let i = 0; i < BARS; i++) {
				const a = f.bars[Math.max(0, i - 1)];
				const c = f.bars[Math.min(BARS - 1, i + 1)];
				smooth[i] = 0.25 * a + 0.5 * f.bars[i] + 0.25 * c;
			}

			g.clearRect(0, 0, W, H);
			const base = C * R0 * (1 + 0.028 * pulse);
			const inner = base + C * GAP;

			// Tinted bloom behind the ring, breathing with energy and beats. A CSS layer so
			// per-frame changes are compositor-only opacity, not a full-canvas gradient fill.
			const tint = `${r} ${gr} ${b}`;
			if (tint !== lastTint) {
				lastTint = tint;
				glowEl.style.background = `radial-gradient(closest-side, rgb(${tint}) ${R0 * 170}%, rgb(${tint} / 0.3) 55%, transparent)`;
			}
			glowEl.style.opacity = (0.08 + 0.26 * f.energy + 0.12 * pulse).toFixed(3);

			// Spokes: bass at the top, mirrored down both sides, slowly turning.
			const lw = Math.max(1.5, ((TAU * inner) / (BARS * 2)) * 0.46);
			const spokes = g.createRadialGradient(cx, cy, inner, cx, cy, inner + C * LEN);
			spokes.addColorStop(0, rgba(0.95));
			spokes.addColorStop(1, rgba(0.3));
			g.strokeStyle = spokes;
			g.lineWidth = lw;
			g.lineCap = 'round';
			g.beginPath();
			for (let i = 0; i < BARS; i++) {
				const len = C * LEN * Math.pow(smooth[i], 1.35);
				for (const side of [1, -1]) {
					const a = rot + (side * Math.PI * (i + 0.5)) / BARS;
					const sx = Math.sin(a);
					const cy_ = -Math.cos(a);
					g.moveTo(cx + sx * inner, cy + cy_ * inner);
					g.lineTo(cx + sx * (inner + len), cy + cy_ * (inner + len));
				}
			}
			g.stroke();

			// Peak beads just past each spoke.
			g.fillStyle = rgba(0.55);
			g.beginPath();
			for (let i = 0; i < BARS; i++) {
				if (f.peaks[i] < 0.04) continue;
				const d = inner + C * LEN * Math.pow(f.peaks[i], 1.35) + lw * 1.6;
				for (const side of [1, -1]) {
					const a = rot + (side * Math.PI * (i + 0.5)) / BARS;
					const x = cx + Math.sin(a) * d;
					const y = cy - Math.cos(a) * d;
					g.moveTo(x + lw * 0.42, y);
					g.arc(x, y, lw * 0.42, 0, TAU);
				}
			}
			g.fill();

			// Thin oscilloscope ring outside the bars, mirrored and tapered so the seams vanish.
			const wr = inner + C * (LEN + 0.018);
			g.strokeStyle = rgba(0.22 + 0.4 * f.energy);
			g.lineWidth = Math.max(1, C / 520);
			g.beginPath();
			for (const side of [1, -1]) {
				for (let i = 0; i <= WAVE_POINTS; i++) {
					const k = i / WAVE_POINTS;
					const taper = Math.sin(Math.PI * k);
					const w = f.waveform[Math.min(WAVE_POINTS - 1, i)] * taper;
					const d = wr + C * 0.028 * Math.max(-1, Math.min(1, w * 1.6));
					const a = rot + side * Math.PI * k;
					const x = cx + Math.sin(a) * d;
					const y = cy - Math.cos(a) * d;
					if (i === 0) g.moveTo(x, y);
					else g.lineTo(x, y);
				}
			}
			g.stroke();
		};

		const unsub = frames.subscribe(draw);
		return () => {
			unsub();
			ro.disconnect();
			host.style.removeProperty('--viz-pulse');
		};
	});
</script>

<div
	class="glow"
	bind:this={glowEl}
	aria-hidden="true"
	style:left="-{BLEED * 100}%"
	style:top="-{BLEED * 100}%"
	style:width="{(1 + 2 * BLEED) * 100}%"
></div>
<canvas
	bind:this={canvas}
	aria-hidden="true"
	style:left="-{BLEED * 100}%"
	style:top="-{BLEED * 100}%"
	style:width="{(1 + 2 * BLEED) * 100}%"
></canvas>

<style>
	canvas,
	.glow {
		position: absolute;
		aspect-ratio: 1;
		pointer-events: none;
	}
	.glow {
		border-radius: 50%;
		will-change: opacity;
	}
</style>
