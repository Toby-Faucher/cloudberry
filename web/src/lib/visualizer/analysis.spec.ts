import { describe, expect, it } from 'vitest';
import { BARS, BeatTracker } from './analysis.ts';

describe('BeatTracker', () => {
	const run = (levelAt: (t: number) => number, seconds = 4) => {
		const b = new BeatTracker();
		const beats: number[] = [];
		const levels = new Float32Array(BARS);
		for (let f = 1; f < seconds * 60; f++) {
			const t = f / 60;
			levels.fill(levelAt(t));
			if (b.update(levels, 1 / 60)) beats.push(t);
		}
		return beats;
	};

	it('fires once per kick, right after it', () => {
		// Low bands jump on each 0.5 s kick and decay, like the decaying 60 Hz bursts.
		const beats = run((t) => 0.2 + 0.7 * Math.exp(-(t % 0.5) / 0.08));
		expect(beats.length).toBe(8);
		beats.forEach((b, i) => expect(Math.abs(b - i * 0.5)).toBeLessThan(0.05));
	});

	it('ignores a steady level after its start', () => {
		expect(run(() => 0.6).filter((t) => t > 0.1)).toEqual([]);
	});

	it('decays over 250 ms', () => {
		const b = new BeatTracker();
		b.update(new Float32Array(BARS).fill(0.9), 0.05);
		expect(b.value).toBe(1);
		b.update(new Float32Array(BARS).fill(0.9), 0.125);
		expect(b.value).toBeCloseTo(0.5);
	});
});
