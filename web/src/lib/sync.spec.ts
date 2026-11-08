import { describe, it, expect } from 'vitest';
import {
	bestOffset,
	ctxToServer,
	dbToGain,
	driftMs,
	expectedPositionMs,
	makeSample,
	planStart,
	serverToCtx,
	shouldPrefetch,
	type TimePair
} from './sync';

describe('clock offset', () => {
	it('assumes the server stamped halfway through the round trip', () => {
		// sent at 1000, received at 1100, server said 5050 -> server is 4000 ahead
		expect(makeSample(1000, 1100, 5050)).toEqual({ rtt: 100, offset: 4000 });
	});

	it('takes the median offset of the 3 lowest-RTT samples', () => {
		const samples = [
			{ rtt: 80, offset: 4030 },
			{ rtt: 12, offset: 4001 },
			{ rtt: 300, offset: 3900 },
			{ rtt: 15, offset: 4009 },
			{ rtt: 14, offset: 3950 }, // fast but skewed: the median shrugs it off
			{ rtt: 40, offset: 4100 }
		];
		expect(bestOffset(samples)).toBe(4001);
	});

	it('copes with fewer than 3 samples', () => {
		expect(bestOffset([])).toBeNull();
		expect(bestOffset([{ rtt: 9, offset: 7 }])).toBe(7);
		expect(bestOffset([{ rtt: 9, offset: 10 }, { rtt: 30, offset: 20 }])).toBe(15);
	});
});

describe('track gain', () => {
	it('converts dB to linear, unity when missing', () => {
		expect(dbToGain(0)).toBe(1);
		expect(dbToGain(-6)).toBeCloseTo(0.501, 3);
		expect(dbToGain(undefined as unknown as number)).toBe(1);
	});
});

describe('expected position', () => {
	const t = { playing: true, position_ms: 10_000, anchor_server_ms: 1_000_000 };

	it('advances in real time while playing', () => {
		expect(expectedPositionMs(t, 1_002_500)).toBe(12_500);
	});

	it('is negative before a future anchor', () => {
		expect(expectedPositionMs({ ...t, position_ms: 0 }, 1_000_000 - 1500)).toBe(-1500);
	});

	it('is frozen while paused', () => {
		expect(expectedPositionMs({ ...t, playing: false }, 9_999_999)).toBe(10_000);
	});
});

describe('server <-> AudioContext time', () => {
	const pair: TimePair = { ctxTime: 2.0, localMs: 50_000 };
	const offset = 1_000; // server = local + 1000

	it('maps server time to ctx seconds', () => {
		// server 51_500 == local 50_500 == 0.5 s after the pair
		expect(serverToCtx(51_500, offset, pair)).toBeCloseTo(2.5, 9);
	});

	it('round-trips', () => {
		expect(ctxToServer(serverToCtx(123_456.7, offset, pair), offset, pair)).toBeCloseTo(123_456.7, 6);
	});
});

describe('planStart', () => {
	it('starts a little ahead of now when the anchor is in the past', () => {
		const t = { playing: true, position_ms: 0, anchor_server_ms: 10_000 };
		expect(planStart(t, 15_000, 60_000, 60)).toEqual({ startServerMs: 15_060, offsetMs: 5_060 });
	});

	it('waits for a future anchor and starts at position_ms', () => {
		const t = { playing: true, position_ms: 0, anchor_server_ms: 20_000 };
		expect(planStart(t, 18_500, 60_000)).toEqual({ startServerMs: 20_000, offsetMs: 0 });
	});

	it('returns null past the end of the track', () => {
		const t = { playing: true, position_ms: 0, anchor_server_ms: 0 };
		expect(planStart(t, 70_000, 60_000)).toBeNull();
	});
});

describe('drift', () => {
	const offset = 1_000;
	const pair: TimePair = { ctxTime: 2.0, localMs: 50_000 };
	const latency = 0.04;
	const t = { playing: true, position_ms: 0, anchor_server_ms: 40_000 };

	// Schedule exactly like Player does, then check drift a few seconds later.
	const plan = planStart(t, ctxToServer(pair.ctxTime, offset, pair), 600_000)!;
	const started = {
		ctxWhen: serverToCtx(plan.startServerMs, offset, pair) - latency,
		offsetSec: plan.offsetMs / 1000
	};

	it('is zero for a correctly scheduled source', () => {
		expect(driftMs(started, started.ctxWhen + 3.7, latency, t, offset, pair)).toBeCloseTo(0, 6);
	});

	it('shows up when the clock offset estimate improves', () => {
		// Server is actually 30 ms further ahead than we thought -> we are 30 ms behind.
		expect(driftMs(started, started.ctxWhen + 3.7, latency, t, offset + 30, pair)).toBeCloseTo(-30, 6);
	});
});

describe('prefetch policy', () => {
	it('decodes the next track ahead only up to 8 minutes (~23 MB of PCM per minute)', () => {
		expect(shouldPrefetch({ duration_ms: 3 * 60_000 })).toBe(true);
		expect(shouldPrefetch({ duration_ms: 8 * 60_000 })).toBe(true);
		expect(shouldPrefetch({ duration_ms: 8 * 60_000 + 1 })).toBe(false);
		expect(shouldPrefetch({ duration_ms: 20 * 60_000 })).toBe(false);
	});
});
