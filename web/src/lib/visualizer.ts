/**
 * Spectrum analysis for the visualizer. Prefers /visualizer.wasm (ABI in the spec),
 * falls back to a small JS version built on AnalyserNode.getByteFrequencyData.
 */

export const FFT_SIZE = 2048;
export const BARS = 64;

export interface Frame {
	bars: Float32Array; // BARS values 0..1
	peaks: Float32Array; // BARS falling peak-hold values 0..1
	energy: number; // 0..1
}

export interface Analyzer {
	kind: 'wasm' | 'js';
	frame(analyser: AnalyserNode, dt: number): Frame;
}

interface VizExports {
	memory: WebAssembly.Memory;
	input_ptr(): number;
	output_ptr(): number;
	peaks_ptr(): number;
	energy(): number;
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
				energy: x.energy()
			};
		}
	};
}

/** Same idea as the Rust version (log-spaced 30 Hz–16 kHz, attack/decay, falling peaks), less exact. */
function jsAnalyzer(): Analyzer {
	const bins = new Uint8Array(FFT_SIZE / 2);
	const bars = new Float32Array(BARS);
	const peaks = new Float32Array(BARS);
	let energy = 0;
	return {
		kind: 'js',
		frame(analyser, dt) {
			analyser.getByteFrequencyData(bins);
			const hzPerBin = analyser.context.sampleRate / FFT_SIZE;
			let sum = 0;
			for (let i = 0; i < BARS; i++) {
				const lo = Math.max(1, Math.floor((30 * Math.pow(16000 / 30, i / BARS)) / hzPerBin));
				const hi = Math.max(lo + 1, Math.floor((30 * Math.pow(16000 / 30, (i + 1) / BARS)) / hzPerBin));
				let m = 0;
				for (let b = lo; b < hi && b < bins.length; b++) m = Math.max(m, bins[b]);
				const v = m / 255;
				const k = v > bars[i] ? 1 - Math.exp(-dt / 0.03) : 1 - Math.exp(-dt / 0.25);
				bars[i] += (v - bars[i]) * k;
				peaks[i] = Math.max(bars[i], peaks[i] - dt * 0.6);
				sum += bars[i];
			}
			energy += (sum / BARS - energy) * (1 - Math.exp(-dt / 0.15));
			return { bars, peaks, energy };
		}
	};
}

let cached: Promise<Analyzer> | null = null;

export function loadAnalyzer(): Promise<Analyzer> {
	return (cached ??= instantiate()
		.then((inst) => wasmAnalyzer(inst.exports as unknown as VizExports))
		.catch((e) => {
			console.info('[visualizer] wasm unavailable, using JS fallback:', e?.message ?? e);
			return jsAnalyzer();
		}));
}
