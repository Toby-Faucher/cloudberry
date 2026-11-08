<script lang="ts">
	import { onMount } from 'svelte';
	import { BARS, loadAnalyzer } from '#lib/visualizer.ts';
	import { reducedMotion } from '#lib/util.ts';

	let { analyser }: { analyser: AnalyserNode } = $props();
	let canvas: HTMLCanvasElement;

	onMount(() => {
		const g = canvas.getContext('2d')!;
		const root = document.documentElement;
		let raf = 0;
		let last = performance.now();
		let accent = '';
		const readAccent = () => (accent = getComputedStyle(canvas).getPropertyValue('--accent').trim());
		readAccent();
		const scheme = matchMedia('(prefers-color-scheme: dark)');
		scheme.addEventListener('change', readAccent);

		// Keep the backing store at device resolution.
		const resize = () => {
			const dpr = Math.min(devicePixelRatio || 1, 2);
			canvas.width = Math.round(canvas.clientWidth * dpr);
			canvas.height = Math.round(canvas.clientHeight * dpr);
		};
		const ro = new ResizeObserver(resize);
		ro.observe(canvas);

		let stopped = false;
		loadAnalyzer().then((an) => {
			const draw = (now: number) => {
				if (stopped) return;
				const dt = Math.min(0.1, (now - last) / 1000);
				last = now;
				const { bars, peaks, energy } = an.frame(analyser, dt);
				if (!reducedMotion) root.style.setProperty('--energy', energy.toFixed(3));
				paint(g, bars, peaks, accent);
				raf = requestAnimationFrame(draw);
			};
			raf = requestAnimationFrame(draw);
		});

		return () => {
			stopped = true;
			cancelAnimationFrame(raf);
			ro.disconnect();
			scheme.removeEventListener('change', readAccent);
			root.style.setProperty('--energy', '0');
		};
	});

	/**
	 * Columns of round "drupelets" (the little beads a cloudberry is made of)
	 * instead of solid bars; the falling peak is a single solid bead.
	 */
	function paint(g: CanvasRenderingContext2D, bars: Float32Array, peaks: Float32Array, color: string) {
		const { width: w, height: h } = g.canvas;
		g.clearRect(0, 0, w, h);
		const group = w < 700 ? 2 : 1; // fewer, chunkier columns on phones
		const cols = BARS / group;
		const step = w / cols;
		const r = step * 0.32;
		const rows = Math.max(4, Math.floor(h / step));
		const rowStep = h / rows;
		g.fillStyle = color;

		for (let c = 0; c < cols; c++) {
			let v = 0;
			let p = 0;
			for (let k = 0; k < group; k++) {
				v = Math.max(v, bars[c * group + k]);
				p = Math.max(p, peaks[c * group + k]);
			}
			const x = step * (c + 0.5);
			const lit = Math.round(v * rows);
			const peakRow = Math.min(rows - 1, Math.round(p * rows));
			for (let i = 0; i < rows; i++) {
				const y = h - rowStep * (i + 0.5);
				g.globalAlpha = i < lit ? 0.45 + 0.55 * (i / rows) : i === peakRow && p > 0.02 ? 0.9 : 0.09;
				g.beginPath();
				g.arc(x, y, r, 0, Math.PI * 2);
				g.fill();
			}
		}
		g.globalAlpha = 1;
	}
</script>

<canvas bind:this={canvas} aria-hidden="true"></canvas>

<style>
	canvas {
		display: block;
		width: 100%;
		height: 112px;
	}
</style>
