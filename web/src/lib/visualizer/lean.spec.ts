import { describe, expect, it, beforeEach } from 'vitest';
import { SHADER_IDS } from './shaders/index.ts';
import { leanKey, setAuto, setShader, stepShader, trackChanged, viz } from './state.svelte.ts';
import { pack, smoothed } from './runner.ts';

describe('lean-back shader picker state', () => {
	beforeEach(() => {
		setShader(SHADER_IDS[0]);
		setAuto(false);
	});

	it('steps forward and back, wrapping at both ends', () => {
		stepShader(-1);
		expect(viz.shader).toBe(SHADER_IDS.at(-1));
		stepShader(1);
		expect(viz.shader).toBe(SHADER_IDS[0]);
		stepShader(1);
		expect(viz.shader).toBe(SHADER_IDS[1]);
	});

	it('ignores unknown shader ids', () => {
		setShader('nope');
		expect(viz.shader).toBe(SHADER_IDS[0]);
	});

	it('maps ←/→/A and leaves other keys alone', () => {
		expect(leanKey('ArrowRight')).toBe(true);
		expect(viz.shader).toBe(SHADER_IDS[1]);
		expect(leanKey('ArrowLeft')).toBe(true);
		expect(viz.shader).toBe(SHADER_IDS[0]);
		expect(leanKey('a')).toBe(true);
		expect(viz.auto).toBe(true);
		expect(leanKey('A')).toBe(true);
		expect(viz.auto).toBe(false);
		expect(leanKey('f')).toBe(false);
		expect(leanKey(' ')).toBe(false);
	});

	it('auto-cycle advances on a track change only when on', () => {
		trackChanged();
		expect(viz.shader).toBe(SHADER_IDS[0]);
		setAuto(true);
		trackChanged();
		expect(viz.shader).toBe(SHADER_IDS[1]);
	});
});

describe('runner uniform packing', () => {
	const frame = (beat: number, energy = 0.5, dt = 1 / 60) => ({ bass: 0.4, mid: 0.3, treble: 0.2, energy, beat, dt });

	it('counts each beat onset once, not every frame of its decay', () => {
		const s = smoothed();
		for (const b of [0, 1, 0.9, 0.7, 0.3, 0, 1, 0.8]) pack(s, frame(b));
		expect(s.beats).toBe(2);
	});

	it('advances time faster with more energy', () => {
		const quiet = smoothed();
		const loud = smoothed();
		for (let i = 0; i < 120; i++) {
			pack(quiet, frame(0, 0));
			pack(loud, frame(0, 1));
		}
		expect(loud.time - 20).toBeGreaterThan(quiet.time - 20);
		expect(quiet.time).toBeGreaterThan(20);
	});

	it('still frames copy the levels and freeze time', () => {
		const s = smoothed();
		pack(s, frame(1, 0.7), true);
		expect(s.time).toBe(20);
		expect(s.energy).toBe(0.7);
		expect(s.beat).toBe(0);
	});
});
