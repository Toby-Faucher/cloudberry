// Smoke test for the raw WASM ABI: node crates/smear/scripts/smoke.mjs [path.wasm]
import { readFileSync } from 'node:fs';

const path = process.argv[2] ?? new URL('../../../target/wasm32-unknown-unknown/release/smear.wasm', import.meta.url);
const bytes = readFileSync(path);
const mod = await WebAssembly.compile(bytes);
const imports = WebAssembly.Module.imports(mod);
console.log(`smear.wasm: ${bytes.length} bytes, imports:`, imports.length ? imports : 'none');
const { exports: w } = await WebAssembly.instantiate(mod, {});
const f32 = (ptr, n) => new Float32Array(w.memory.buffer, ptr, n);
const bodies = f32(w.bodies_ptr(), 2), out = f32(w.smear_ptr(), 4);

// A body that whips right at 2400 px/s for 10 frames, then stops dead.
const dt = 1 / 60;
let ok = true;
for (let f = 0; f < 22; f++) {
  if (f < 10) bodies[0] += 40;
  w.step(1, dt);
  const [s, a, blur, speed] = out;
  console.log(`frame ${String(f).padStart(2)} x=${bodies[0].toFixed(0).padStart(3)}  stretch=${s.toFixed(3)} angle=${a.toFixed(2)} blur=${blur.toFixed(2)}px speed=${speed.toFixed(0)}`);
  if (f === 9) ok &&= s > 0.5 && Math.abs(a) < 1e-3;
}
ok &&= out[0] < 0.01;

const W = 800, H = 600;
w.rain_spawn(300, W, 1234); // capped to 256
const rain = f32(w.rain_ptr(), 256 * 7);
for (let f = 0; f <= 240; f++) {
  const live = w.rain_step(dt, W, H);
  if (f % 40 === 0) {
    let maxY = -Infinity, inX = true;
    for (let i = 0; i < live; i++) {
      maxY = Math.max(maxY, rain[i * 7 + 1]);
      inX &&= rain[i * 7] >= 0 && rain[i * 7] <= W;
    }
    console.log(`rain t=${(f * dt).toFixed(2)}s live=${live} maxY=${live ? maxY.toFixed(0) : '-'} x-in-bounds=${inX}`);
    ok &&= inX && (!live || maxY <= H) && (f > 0 || live === 256);
  }
}
if (imports.length || !ok) process.exit(1);
console.log('ok');
