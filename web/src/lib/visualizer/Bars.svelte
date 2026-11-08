<script lang="ts">
	import { onMount } from 'svelte';
	import { BARS, fit, follow, frames, type Frame } from './analysis.ts';

	/** Bottom spectrum: rounded-top bars with falling peak caps. */
	let canvas: HTMLCanvasElement;

	onMount(() => {
		const g = canvas.getContext('2d')!;
		const ro = new ResizeObserver(() => fit(canvas));
		ro.observe(canvas);
		let pulse = 0;

		const draw = (f: Frame) => {
			const { width: w, height: h } = canvas;
			g.clearRect(0, 0, w, h);
			const [r, gr, b] = f.tint.map(Math.round);
			const group = canvas.clientWidth < 360 ? 2 : 1; // chunkier columns on narrow screens
			const cols = BARS / group;
			const step = w / cols;
			const bw = step * 0.62;
			const cap = Math.max(2, bw * 0.32);
			const top = cap * 3; // headroom for peak caps
			pulse = follow(pulse, f.beat, f.dt, 0.03, 0.16);

			const fill = g.createLinearGradient(0, h, 0, top);
			fill.addColorStop(0, `rgb(${r} ${gr} ${b} / 0.28)`);
			fill.addColorStop(1, `rgb(${r} ${gr} ${b} / ${(0.85 + 0.15 * pulse).toFixed(3)})`);
			g.fillStyle = fill;
			g.beginPath();
			const caps: number[] = [];
			for (let c = 0; c < cols; c++) {
				let v = 0;
				let p = 0;
				for (let k = 0; k < group; k++) {
					v = Math.max(v, f.bars[c * group + k]);
					p = Math.max(p, f.peaks[c * group + k]);
				}
				const x = step * c + (step - bw) / 2;
				// Never shorter than its own width: at rest each bar is a little bead.
				const bh = Math.max(bw, (h - top) * v);
				g.roundRect(x, h - bh, bw, bh, [bw / 2, bw / 2, bw / 4, bw / 4]);
				if (p > 0.03) caps.push(x, h - Math.max(bw, (h - top) * p) - cap * 1.8);
			}
			g.fill();

			g.fillStyle = `rgb(${r} ${gr} ${b} / 0.9)`;
			g.beginPath();
			for (let i = 0; i < caps.length; i += 2) g.roundRect(caps[i], caps[i + 1], bw, cap, cap / 2);
			g.fill();
		};

		const unsub = frames.subscribe(draw);
		return () => {
			unsub();
			ro.disconnect();
		};
	});
</script>

<canvas bind:this={canvas} aria-hidden="true"></canvas>

<style>
	canvas {
		display: block;
		width: 100%;
		height: 112px;
		margin-bottom: 16px;
	}
</style>
