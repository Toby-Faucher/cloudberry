// @ts-expect-error no @types/node here (web/package.json is not ours to change); vitest runs in node.
import { existsSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { createJsEngine, wasmEngine, MAX_RAIN, RAIN_STRIDE, type Engine } from './smear';

const DT = 1 / 60;

/** Moves body 0 by (dx, dy) per frame; returns [stretch, angle, blur, speed]. */
function run(e: Engine, dx: number, dy: number, frames: number): number[] {
	const b = e.bodies();
	for (let f = 0; f < frames; f++) {
		b[0] += dx;
		b[1] += dy;
		e.step(1, DT);
	}
	return Array.from(e.smear().subarray(0, 4));
}

describe('JS fallback smear math', () => {
	it('still body has no stretch', () => {
		expect(run(createJsEngine(), 0, 0, 30)).toEqual([0, 0, 0, 0]);
	});

	it('fast constant velocity stretches along the motion', () => {
		const [s, a, blur, speed] = run(createJsEngine(), 25, 0, 20);
		expect(speed).toBeCloseTo(1500, -1);
		expect(s).toBeGreaterThan(0.5);
		expect(a).toBeCloseTo(0, 3);
		expect(blur).toBeGreaterThan(0);
	});

	it('diagonal motion angle', () => {
		expect(run(createJsEngine(), -20, 20, 20)[1]).toBeCloseTo(0.75 * Math.PI, 3);
	});

	it('stretch decays to ~0 within 150 ms of stopping', () => {
		const e = createJsEngine();
		run(e, 50, 0, 20);
		expect(run(e, 0, 0, 9)[0]).toBeLessThan(0.01);
	});

	it('reset forgets history', () => {
		const e = createJsEngine();
		run(e, 40, 0, 10);
		e.reset(0);
		e.bodies()[0] = 99999;
		e.step(1, DT);
		expect(e.smear()[0]).toBe(0);
	});

	it('rain falls, stays in bounds, caps at 256 and dies out', () => {
		const e = createJsEngine();
		e.rain_spawn(300, 800, 7);
		let live = e.rain_step(DT, 800, 600);
		expect(live).toBe(MAX_RAIN);
		for (let f = 0; f < 600; f++) {
			live = e.rain_step(DT, 800, 600);
			const p = e.rain();
			for (let i = 0; i < live; i++) {
				expect(p[i * RAIN_STRIDE]).toBeGreaterThanOrEqual(0);
				expect(p[i * RAIN_STRIDE]).toBeLessThanOrEqual(800);
				expect(p[i * RAIN_STRIDE + 1]).toBeLessThanOrEqual(600);
			}
		}
		expect(live).toBe(0);
	});
});

// Parity with the real crate when it has been built (scripts/build-wasm.sh or cargo build).
const wasmPath = ['static/smear.wasm', '../target/wasm32-unknown-unknown/release/smear.wasm'].find(existsSync);

describe.skipIf(!wasmPath)('JS fallback matches smear.wasm', () => {
	it('bodies and rain agree', async () => {
		const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath!), {});
		const w = wasmEngine(instance.exports as never);
		const j = createJsEngine();
		for (const e of [w, j]) {
			run(e, 30, -12, 12);
			run(e, 0, 0, 4);
			e.rain_spawn(40, 640, 99);
			for (let f = 0; f < 60; f++) e.rain_step(DT, 640, 480);
		}
		const close = (a: Float32Array, b: Float32Array) =>
			a.forEach((v, i) => expect(v).toBeCloseTo(b[i], 1));
		close(w.smear().subarray(0, 4), j.smear().subarray(0, 4));
		close(w.rain().subarray(0, 40 * RAIN_STRIDE), j.rain().subarray(0, 40 * RAIN_STRIDE));
	});
});
