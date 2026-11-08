// Smoke test for the raw WASM ABI: node crates/visualizer/scripts/smoke.mjs [path.wasm]
import { readFileSync } from 'node:fs';

const FFT_SIZE = 2048, BARS = 64, SR = 48000, HZ = 1000;
const path = process.argv[2] ?? new URL('../../../target/wasm32-unknown-unknown/release/visualizer.wasm', import.meta.url);
const mod = await WebAssembly.compile(readFileSync(path));
const imports = WebAssembly.Module.imports(mod);
console.log('imports:', imports.length ? imports : 'none');
const { exports: w } = await WebAssembly.instantiate(mod, {});

// Views must be re-created after calls in case memory grew.
const view = (ptr, n) => new Float32Array(w.memory.buffer, ptr, n);
for (let f = 0; f < 30; f++) {
  const input = view(w.input_ptr(), FFT_SIZE);
  for (let i = 0; i < FFT_SIZE; i++) input[i] = 0.5 * Math.sin((2 * Math.PI * HZ * (i + f * FFT_SIZE)) / SR);
  w.process(SR, 1 / 60);
}
const bars = view(w.output_ptr(), BARS), peaks = view(w.peaks_ptr(), BARS);
const top = bars.indexOf(Math.max(...bars));
const edge = (i) => 30 * (16000 / 30) ** (i / BARS);
console.log(`top band: ${top} (${edge(top).toFixed(0)}-${edge(top + 1).toFixed(0)} Hz) = ${bars[top].toFixed(3)}, peak ${peaks[top].toFixed(3)}`);
console.log(`band 10: ${bars[10].toFixed(3)}, band 60: ${bars[60].toFixed(3)}`);
console.log(`energy: ${w.energy().toFixed(3)}, bass: ${w.bass().toFixed(3)}, mid: ${w.mid().toFixed(3)}, treble: ${w.treble().toFixed(3)}`);
const wave = view(w.waveform_ptr(), 256);
console.log(`waveform[0..4]: ${[...wave.slice(0, 4)].map((v) => v.toFixed(3)).join(' ')}, max |v|: ${Math.max(...wave.map(Math.abs)).toFixed(3)}`);

// Kick pattern: 60 Hz bursts every 0.5 s, 60 fps, each frame sees the last FFT_SIZE samples.
const kick = (t) => (t < 0 ? 0 : 0.8 * Math.sin(2 * Math.PI * 60 * (t % 0.5)) * Math.exp(-(t % 0.5) / 0.06));
const beats = [];
for (let f = 1; f <= 120; f++) {
  const end = f / 60, input = view(w.input_ptr(), FFT_SIZE);
  for (let i = 0; i < FFT_SIZE; i++) input[i] = kick(end - (FFT_SIZE - i) / SR);
  w.process(SR, 1 / 60);
  if (w.beat() === 1) beats.push(end.toFixed(3));
}
console.log(`beats (kicks at 0.5 s steps, 2 s): ${beats.join(' ')}`);
if (!(edge(top) <= HZ && HZ < edge(top + 1)) || imports.length || beats.length !== 4) process.exit(1);
