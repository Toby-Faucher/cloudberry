/**
 * The one rAF loop behind every visualizer. It runs while anything subscribes to
 * `frames`, does one analysis per frame (wasm ABI from the spec, JS fallback if the
 * wasm can't load) and hands every subscriber the same mutable `Frame`.
 */

export const FFT_SIZE = 2048;
export const BARS = 64;
export const WAVE_POINTS = 256;

export type Rgb = [number, number, number];

export interface Frame {
	bars: Float32Array; // BARS values 0..1, log-spaced 30 Hz-16 kHz
	peaks: Float32Array; // BARS falling peak-hold values 0..1
	waveform: Float32Array; // WAVE_POINTS samples -1..1
	energy: number;
	bass: number;
	mid: number;
	treble: number;
	beat: number; // 1 on an onset, falls to 0 over ~250 ms
	tint: Rgb; // eased --track-tint, 0..255
	time: number; // seconds of animation time (frozen under reduced motion)
	dt: number;
}

type Values = Omit<Frame, 'tint' | 'time' | 'dt'>;

interface Analyzer {
	kind: 'wasm' | 'js';
	frame(analyser: AnalyserNode, dt: number): Values;
}

interface VizExports {
	memory: WebAssembly.Memory;
	input_ptr(): number;
	output_ptr(): number;
	peaks_ptr(): number;
	waveform_ptr(): number;
	energy(): number;
	bass(): number;
	mid(): number;
	treble(): number;
	beat(): number;
	process(sampleRate: number, dt: number): void;
}

async function instantiate(): Promise<WebAssembly.Instance> {
	const res = await fetch('/visualizer.wasm');
	if (!res.ok) throw new Error(`HTTP ${res.status}`);
	try {
		return (await WebAssembly.instantiateStreaming(res.clone(), {})).instance;
	} catch {
		// Wrong MIME type etc.: fall back to the buffered path.
		return (await WebAssembly.instantiate(await res.arrayBuffer(), {})).instance;
	}
}

function wasmAnalyzer(x: VizExports): Analyzer {
	if (typeof x.beat !== 'function') throw new Error('stale visualizer.wasm (no beat export)');
	const view = (ptr: number, len: number) => new Float32Array(x.memory.buffer, ptr, len);
	return {
		kind: 'wasm',
		frame(analyser, dt) {
			// Views are recreated every frame: memory.grow() detaches old buffers.
			analyser.getFloatTimeDomainData(view(x.input_ptr(), FFT_SIZE));
			x.process(analyser.context.sampleRate, dt);
			return {
				bars: view(x.output_ptr(), BARS),
				peaks: view(x.peaks_ptr(), BARS),
				waveform: view(x.waveform_ptr(), WAVE_POINTS),
				energy: x.energy(),
				bass: x.bass(),
				mid: x.mid(),
				treble: x.treble(),
				beat: x.beat()
			};
		}
	};
}

const edge = (i: number) => 30 * Math.pow(16000 / 30, i / BARS);
const centre = (i: number) => Math.sqrt(edge(i) * edge(i + 1));
const bandsIn = (lo: number, hi: number) =>
	[...Array(BARS).keys()].filter((i) => centre(i) >= lo && centre(i) < hi);
const BASS = bandsIn(30, 150);
const MID = bandsIn(150, 2000);
const TREBLE = bandsIn(2000, 16000);
/** Bars entirely below 500 Hz feed the beat detector, like the Rust version. */
const FLUX_BANDS = [...Array(BARS).keys()].filter((i) => edge(i + 1) <= 500).length;

/** Spectral-flux onset detector; same constants and logic as `BeatDetector` in Rust. */
export class BeatTracker {
	value = 0;
	private prev = new Float32Array(BARS);
	private hist: { flux: number; dt: number }[] = [];
	private since = Infinity;

	update(levels: ArrayLike<number>, dt: number): boolean {
		let rise = 0;
		for (let i = 0; i < FLUX_BANDS; i++) rise += Math.max(0, levels[i] - this.prev[i]);
		const flux = rise / FLUX_BANDS;
		for (let i = 0; i < BARS; i++) this.prev[i] = levels[i];

		let sum = 0, sq = 0, t = 0, n = 0;
		for (let k = this.hist.length - 1; k >= 0 && t < 1; k--) {
			const h = this.hist[k];
			sum += h.flux;
			sq += h.flux * h.flux;
			t += h.dt;
			n++;
		}
		const mean = n ? sum / n : 0;
		const sd = n ? Math.sqrt(Math.max(0, sq / n - mean * mean)) : 0;

		this.since += dt;
		this.value = Math.max(0, this.value - dt / 0.25);
		const onset = flux > 0.03 && flux > mean + 1.5 * sd && this.since >= 0.2;
		if (onset) {
			this.value = 1;
			this.since = 0;
		}
		this.hist.push({ flux, dt });
		if (this.hist.length > 128) this.hist.shift();
		return onset;
	}
}

/** Same idea as the Rust version (log-spaced bars, attack/decay, falling peaks), less exact. */
function jsAnalyzer(): Analyzer {
	const bins = new Uint8Array(FFT_SIZE / 2);
	const time = new Float32Array(FFT_SIZE);
	const levels = new Float32Array(BARS);
	const out: Values = {
		bars: new Float32Array(BARS),
		peaks: new Float32Array(BARS),
		waveform: new Float32Array(WAVE_POINTS),
		energy: 0, bass: 0, mid: 0, treble: 0, beat: 0
	};
	const beat = new BeatTracker();
	const mean = (idx: number[]) => idx.reduce((s, i) => s + levels[i], 0) / idx.length;
	return {
		kind: 'js',
		frame(analyser, dt) {
			analyser.getByteFrequencyData(bins);
			analyser.getFloatTimeDomainData(time);
			const hzPerBin = analyser.context.sampleRate / FFT_SIZE;
			let sum = 0;
			for (let i = 0; i < BARS; i++) {
				const lo = Math.max(1, Math.floor(edge(i) / hzPerBin));
				const hi = Math.max(lo + 1, Math.floor(edge(i + 1) / hzPerBin));
				let m = 0;
				for (let b = lo; b < hi && b < bins.length; b++) m = Math.max(m, bins[b]);
				levels[i] = m / 255;
				out.bars[i] = follow(out.bars[i], levels[i], dt, 0.03, 0.25);
				out.peaks[i] = Math.max(out.bars[i], out.peaks[i] - dt * 0.6);
				sum += out.bars[i];
			}
			out.energy += (sum / BARS - out.energy) * (1 - Math.exp(-dt / 0.15));
			out.bass = follow(out.bass, mean(BASS), dt, 0.03, 0.25);
			out.mid = follow(out.mid, mean(MID), dt, 0.03, 0.25);
			out.treble = follow(out.treble, mean(TREBLE), dt, 0.03, 0.25);
			beat.update(levels, dt);
			out.beat = beat.value;
			const step = FFT_SIZE / WAVE_POINTS;
			for (let i = 0; i < WAVE_POINTS; i++) {
				let s = 0;
				for (let k = 0; k < step; k++) s += time[i * step + k];
				out.waveform[i] = s / step;
			}
			return out;
		}
	};
}

let cached: Promise<Analyzer> | null = null;

function loadAnalyzer(): Promise<Analyzer> {
	return (cached ??= instantiate()
		.then((inst) => wasmAnalyzer(inst.exports as unknown as VizExports))
		.catch((e) => {
			console.info('[visualizer] wasm unavailable, using JS fallback:', e?.message ?? e);
			return jsAnalyzer();
		}));
}

// ---- the loop ----------------------------------------------------------------

export const reducedMotion =
	typeof matchMedia !== 'undefined' && matchMedia('(prefers-reduced-motion: reduce)').matches;
/** Under reduced motion we only take a still snapshot this often (no animation in between). */
const SNAPSHOT_S = 6;
const BRAND: Rgb = [240, 145, 43]; // --berry-500

const frame: Frame = {
	bars: new Float32Array(BARS),
	peaks: new Float32Array(BARS),
	waveform: new Float32Array(WAVE_POINTS),
	energy: 0, bass: 0, mid: 0, treble: 0, beat: 0,
	tint: [...BRAND],
	time: 0,
	dt: 0
};

type Sub = (f: Frame) => void;
const subs = new Set<Sub>();
let analyser: AnalyserNode | null = null;
let analyzer: Analyzer | null = null;
let raf = 0;
let last = 0;
let sinceSnapshot = Infinity;
let tintTarget: Rgb = [...BRAND];
let sinceTint = Infinity;
let probe: HTMLElement | null = null;
let parse: CanvasRenderingContext2D | null = null;
let lastEnergy = -1;

/** Any CSS colour string -> rgb, via a 1px canvas (handles hex, rgb(), color-mix, oklch...). */
export function cssColor(css: string, fallback: Rgb = BRAND): Rgb {
	parse ??= document.createElement('canvas').getContext('2d', { willReadFrequently: true });
	if (!parse) return fallback;
	parse.clearRect(0, 0, 1, 1);
	parse.fillStyle = `rgb(${fallback.join(' ')})`;
	parse.fillStyle = css;
	parse.fillRect(0, 0, 1, 1);
	const [r, g, b] = parse.getImageData(0, 0, 1, 1).data;
	return [r, g, b];
}

/** `--track-tint` as rgb; the probe element resolves var() and inheritance for us. */
function readTint(): Rgb {
	if (!probe) {
		probe = document.createElement('i');
		probe.style.cssText = 'position:absolute;width:0;height:0;color:var(--track-tint, #f0912b)';
		probe.setAttribute('aria-hidden', 'true');
		document.body.append(probe);
	}
	return cssColor(getComputedStyle(probe).color);
}

/** Size a canvas's backing store to its CSS box (DPR capped at 2) times `scale`. */
export function fit(canvas: HTMLCanvasElement, scale = 1): void {
	const k = Math.min(devicePixelRatio || 1, 2) * scale;
	canvas.width = Math.max(1, Math.round(canvas.clientWidth * k));
	canvas.height = Math.max(1, Math.round(canvas.clientHeight * k));
}

/** One-pole follower with separate rise/fall time constants (s), frame-rate independent. */
export function follow(cur: number, target: number, dt: number, rise: number, fall: number): number {
	return cur + (target - cur) * (1 - Math.exp(-dt / (target > cur ? rise : fall)));
}

function zero(f: Frame, dt: number) {
	// No audio graph yet: let everything relax to rest instead of snapping.
	const k = 1 - Math.exp(-dt / 0.25);
	for (let i = 0; i < BARS; i++) {
		f.bars[i] -= f.bars[i] * k;
		f.peaks[i] = Math.max(f.bars[i], f.peaks[i] - dt * 0.6);
	}
	f.waveform.fill(0);
	f.energy -= f.energy * k;
	f.bass -= f.bass * k;
	f.mid -= f.mid * k;
	f.treble -= f.treble * k;
	f.beat = Math.max(0, f.beat - dt / 0.25);
}

function tick(now: number) {
	raf = requestAnimationFrame(tick);
	const dt = Math.min(0.1, Math.max(0, (now - last) / 1000));
	last = now;

	sinceTint += dt;
	if (sinceTint > 0.5) {
		sinceTint = 0;
		tintTarget = readTint();
	}
	const tk = reducedMotion ? 1 : 1 - Math.exp(-dt / 0.6);
	for (let i = 0; i < 3; i++) frame.tint[i] += (tintTarget[i] - frame.tint[i]) * tk;

	if (reducedMotion) {
		sinceSnapshot += dt;
		if (sinceSnapshot < SNAPSHOT_S) return;
		sinceSnapshot = 0;
	}
	// Analysis uses real dt even for snapshots so wasm smoothing settles (it clamps to 1 s).
	const adt = reducedMotion ? 1 : dt;
	if (analyser && analyzer) {
		const v = analyzer.frame(analyser, adt);
		frame.bars = v.bars;
		frame.peaks = v.peaks;
		frame.waveform = v.waveform;
		frame.energy = v.energy;
		frame.bass = v.bass;
		frame.mid = v.mid;
		frame.treble = v.treble;
		frame.beat = reducedMotion ? 0 : v.beat;
	} else {
		zero(frame, adt);
	}
	frame.dt = dt;
	if (!reducedMotion) frame.time += dt;

	// The ambient glow + brand dot read this. Root vars restyle the whole page, so skip tiny changes.
	if (!reducedMotion && Math.abs(frame.energy - lastEnergy) > 0.004) {
		lastEnergy = frame.energy;
		document.documentElement.style.setProperty('--energy', frame.energy.toFixed(3));
	}
	for (const s of subs) s(frame);
}

function run() {
	if (raf || !subs.size || document.hidden) return;
	last = performance.now();
	raf = requestAnimationFrame(tick);
}

function halt() {
	cancelAnimationFrame(raf);
	raf = 0;
}

function onVisibility() {
	if (document.hidden) halt();
	else run();
}

/**
 * Svelte-store-compatible: `frames.subscribe(fn)` calls `fn(frame)` now and once per
 * animation frame. The Frame object (and its arrays) is shared and mutated in place:
 * read it inside the callback, don't keep references to the arrays.
 */
export const frames = {
	subscribe(fn: Sub): () => void {
		subs.add(fn);
		fn(frame);
		if (subs.size === 1) document.addEventListener('visibilitychange', onVisibility);
		sinceSnapshot = Infinity;
		run();
		return () => {
			subs.delete(fn);
			if (subs.size) return;
			halt();
			document.removeEventListener('visibilitychange', onVisibility);
		};
	}
};

/** Feed the visualizers from this analyser (fftSize must be FFT_SIZE). */
export function startAnalysis(node: AnalyserNode): void {
	analyser = node;
	sinceSnapshot = Infinity;
	loadAnalyzer().then((a) => {
		analyzer = a;
	});
}

export function stopAnalysis(): void {
	analyser = null;
	document.documentElement.style.setProperty('--energy', '0');
	lastEnergy = -1;
}

