/**
 * Cloudberry sync engine.
 *
 * The server never streams audio; it publishes a *timeline*: "track X was at
 * `position_ms` at server time `anchor_server_ms`, and is (or isn't) playing".
 * Every client independently downloads the file and schedules it so that the
 * sample that is audible at server time S is `position_ms + (S - anchor)`.
 *
 * Three clocks are involved:
 *   - server clock   (ms since epoch, f64)            -> what the timeline is in
 *   - local clock    (performance.timeOrigin + now())  -> monotonic, epoch-ish ms
 *   - audio clock    (AudioContext.currentTime, s)     -> what source.start() wants
 *
 * server  = local + offset              (offset estimated NTP-style via ping/pong)
 * audio   = pair.ctxTime + (local - pair.localMs) / 1000
 *           (a "pair" is a simultaneous reading of both local and audio clocks)
 *
 * The pure functions at the top are unit-tested in sync.spec.ts; the Clock and
 * Player classes below just wire them to the browser.
 */

import type { StationState } from './types';

// ---------------------------------------------------------------------------
// Pure math
// ---------------------------------------------------------------------------

/** Keep this many recent ping/pong samples. */
export const MAX_SAMPLES = 10;
/** Restart the source when it is further than this from where it should be. */
export const DRIFT_TOLERANCE_MS = 25;

export interface ClockSample {
	/** server - local, ms */
	offset: number;
	/** round trip, ms */
	rtt: number;
}

/**
 * NTP-style sample: assume the server read its clock halfway through the round trip.
 * t0/t1 are local send/receive times, serverTime is what the server stamped in between.
 */
export function makeSample(t0: number, t1: number, serverTime: number): ClockSample {
	const rtt = t1 - t0;
	return { rtt, offset: serverTime - (t0 + rtt / 2) };
}

/**
 * The lowest-RTT sample has the smallest possible asymmetry error (|err| <= rtt/2),
 * so it is the best offset estimate we have. Returns null with no samples.
 */
export function bestOffset(samples: readonly ClockSample[]): number | null {
	let best: ClockSample | null = null;
	for (const s of samples) if (!best || s.rtt < best.rtt) best = s;
	return best ? best.offset : null;
}

type Timeline = Pick<StationState, 'playing' | 'position_ms' | 'anchor_server_ms'>;

/**
 * Where (ms into the track) the shared timeline is at server time `serverMs`.
 * Negative when playing and the anchor is still in the future ("starts in N ms").
 */
export function expectedPositionMs(t: Timeline, serverMs: number): number {
	return t.playing ? t.position_ms + (serverMs - t.anchor_server_ms) : t.position_ms;
}

/** A simultaneous reading of the audio clock (s) and the local clock (ms). */
export interface TimePair {
	ctxTime: number;
	localMs: number;
}

/** Server time (ms) -> AudioContext time (s). */
export function serverToCtx(serverMs: number, offset: number, pair: TimePair): number {
	return pair.ctxTime + (serverMs - offset - pair.localMs) / 1000;
}

/** AudioContext time (s) -> server time (ms). Inverse of serverToCtx. */
export function ctxToServer(ctxTime: number, offset: number, pair: TimePair): number {
	return pair.localMs + offset + (ctxTime - pair.ctxTime) * 1000;
}

export interface StartPlan {
	/** Server time at which the first sample should be *audible*. */
	startServerMs: number;
	/** Where in the track to start, ms. */
	offsetMs: number;
}

/**
 * Decide when and where to start a source. We can't start in the past, so we
 * start `leadMs` from now (enough to set up nodes), or at the anchor if that is
 * later (e.g. a fresh track the server scheduled 1.5 s ahead). Returns null if
 * the timeline is already past the end of the track.
 */
export function planStart(
	t: Timeline,
	nowServerMs: number,
	durationMs: number,
	leadMs = 60
): StartPlan | null {
	const startServerMs = Math.max(nowServerMs + leadMs, t.anchor_server_ms);
	const offsetMs = Math.max(0, expectedPositionMs(t, startServerMs));
	return offsetMs >= durationMs ? null : { startServerMs, offsetMs };
}

export interface Started {
	/** AudioContext time passed to source.start() */
	ctxWhen: number;
	/** offset (s) passed to source.start() */
	offsetSec: number;
}

/**
 * How far (ms) the running source is ahead (+) or behind (-) the shared timeline.
 * What the source renders at ctx time `ctxNow` reaches the speakers `latencySec`
 * later, so that is the server instant we compare against.
 */
export function driftMs(
	started: Started,
	ctxNow: number,
	latencySec: number,
	t: Timeline,
	offset: number,
	pair: TimePair
): number {
	const actualMs = (started.offsetSec + (ctxNow - started.ctxWhen)) * 1000;
	const audibleAt = ctxToServer(ctxNow + latencySec, offset, pair);
	return actualMs - expectedPositionMs(t, audibleAt);
}

// ---------------------------------------------------------------------------
// Clock: ping/pong bookkeeping
// ---------------------------------------------------------------------------

/** Monotonic local clock in epoch-ish ms (immune to wall-clock jumps). */
export const localNow = () => performance.timeOrigin + performance.now();

export class Clock {
	private samples: ClockSample[] = [];
	offset: number | null = null;
	private readyResolve!: () => void;
	/** Resolves once we have a few samples, so the first schedule is decent. */
	readonly ready = new Promise<void>((r) => (this.readyResolve = r));

	/** Feed a pong. t1 defaults to "now" (i.e. the receive time). */
	add(t0: number, serverTime: number, t1 = localNow()) {
		this.samples.push(makeSample(t0, t1, serverTime));
		if (this.samples.length > MAX_SAMPLES) this.samples.shift();
		this.offset = bestOffset(this.samples);
		if (this.samples.length >= 3) this.readyResolve();
	}

	serverNow(): number {
		return localNow() + (this.offset ?? 0);
	}
}

// ---------------------------------------------------------------------------
// Player: Web Audio scheduling
// ---------------------------------------------------------------------------

const DRIFT_CHECK_MS = 2000;

/**
 * Owns the AudioContext graph: source -> analyser -> gain -> destination.
 * Call `update(state)` on every `state` message; it figures out whether
 * anything relevant to audio changed and (re)schedules.
 */
export class Player {
	readonly ctx: AudioContext;
	readonly analyser: AnalyserNode;
	private gain: GainNode;
	private buffers = new Map<string, Promise<AudioBuffer>>();
	private source: AudioBufferSourceNode | null = null;
	private started: Started | null = null;
	private state: StationState | null = null;
	/** Identity of the audio-relevant part of the last state we scheduled for. */
	private key = '';
	/** Bumped on every (re)schedule so stale async loads bail out. */
	private gen = 0;
	private timer: ReturnType<typeof setInterval>;
	/** Last measured drift, ms (for the debug readout). */
	lastDrift = 0;

	constructor(
		private clock: Clock,
		private onError: (msg: string) => void = console.error
	) {
		// Must be constructed inside a user gesture (the "Tune in" click).
		this.ctx = new AudioContext({ latencyHint: 'playback' });
		this.analyser = new AnalyserNode(this.ctx, { fftSize: 2048, smoothingTimeConstant: 0 });
		this.gain = new GainNode(this.ctx);
		this.analyser.connect(this.gain).connect(this.ctx.destination);
		void this.ctx.resume();
		this.timer = setInterval(() => this.checkDrift(), DRIFT_CHECK_MS);
	}

	setVolume(v: number) {
		// Short ramp avoids zipper noise while dragging the slider.
		this.gain.gain.setTargetAtTime(v, this.ctx.currentTime, 0.02);
	}

	/** Reconcile with a fresh station state. Cheap when nothing audio-related changed. */
	update(state: StationState) {
		this.state = state;
		const cur = state.current;
		const next = state.queue[0];

		// Cache policy: current + next only (decoded PCM is ~10 MB/minute).
		const keep = new Set([cur?.track.id, next?.track.id]);
		for (const id of this.buffers.keys()) if (!keep.has(id)) this.buffers.delete(id);
		if (next) this.load(next.track.id).catch(() => {}); // prefetch; errors resurface on play

		const key = cur
			? `${cur.queue_id}|${state.playing}|${state.position_ms}|${state.anchor_server_ms}`
			: '';
		if (key !== this.key) {
			this.key = key;
			void this.reschedule();
		}
	}

	destroy() {
		clearInterval(this.timer);
		this.stop();
		void this.ctx.close();
	}

	/**
	 * Read both clocks together, plus the latency still to add on top of that pair.
	 *
	 * Preferred: getOutputTimestamp() says "the sample at contextTime is reaching the
	 * speakers at performanceTime". It is interpolated (currentTime only moves once per
	 * audio callback, ~10 ms+ with latencyHint 'playback') and already includes the output
	 * latency, so latency = 0. Fallback: currentTime + now(), compensated by
	 * outputLatency || baseLatency. In headless Chromium the fallback jittered by
	 * +-18 ms, close enough to the 25 ms threshold to cause spurious resyncs.
	 */
	private pair(): TimePair & { latency: number } {
		const ts = this.ctx.getOutputTimestamp?.();
		if (ts?.contextTime && ts.performanceTime)
			return { ctxTime: ts.contextTime, localMs: performance.timeOrigin + ts.performanceTime, latency: 0 };
		return {
			ctxTime: this.ctx.currentTime,
			localMs: localNow(),
			latency: this.ctx.outputLatency || this.ctx.baseLatency || 0
		};
	}

	private load(id: string): Promise<AudioBuffer> {
		let p = this.buffers.get(id);
		if (!p) {
			p = fetch(`/audio/${encodeURIComponent(id)}.mp3`)
				.then((r) => {
					if (!r.ok) throw new Error(`audio ${id}: HTTP ${r.status}`);
					return r.arrayBuffer();
				})
				.then((b) => this.ctx.decodeAudioData(b));
			// Don't cache failures; a later state can retry.
			p.catch(() => this.buffers.get(id) === p && this.buffers.delete(id));
			this.buffers.set(id, p);
		}
		return p;
	}

	private stop() {
		if (this.source) {
			try {
				this.source.stop();
			} catch {
				/* never started */
			}
			this.source.disconnect();
		}
		this.source = null;
		this.started = null;
	}

	/** Stop whatever is playing and schedule the current state from scratch. */
	private async reschedule() {
		const gen = ++this.gen;
		this.stop();
		this.lastDrift = 0;
		const state = this.state;
		if (!state?.current || !state.playing) return;

		let buffer: AudioBuffer;
		try {
			[buffer] = await Promise.all([this.load(state.current.track.id), this.clock.ready]);
		} catch (e) {
			if (gen === this.gen) this.onError(`Couldn't load "${state.current.track.title}"`);
			console.error(e);
			return;
		}
		if (gen !== this.gen || this.clock.offset === null) return; // superseded

		const plan = planStart(state, this.clock.serverNow(), buffer.duration * 1000);
		if (!plan) return;

		const pair = this.pair();
		const ctxWhen = Math.max(
			this.ctx.currentTime,
			serverToCtx(plan.startServerMs, this.clock.offset, pair) - pair.latency
		);
		const src = new AudioBufferSourceNode(this.ctx, { buffer });
		src.connect(this.analyser);
		src.start(ctxWhen, plan.offsetMs / 1000);
		this.source = src;
		this.started = { ctxWhen, offsetSec: plan.offsetMs / 1000 };
	}

	private checkDrift() {
		if (this.ctx.state === 'suspended') void this.ctx.resume();
		const s = this.state;
		if (!this.started || !s?.playing || this.clock.offset === null) return;
		const pair = this.pair();
		if (pair.ctxTime < this.started.ctxWhen) return; // not audible yet
		const d = driftMs(this.started, pair.ctxTime, pair.latency, s, this.clock.offset, pair);
		const prev = this.lastDrift;
		this.lastDrift = d;
		// A restart is audible, so ignore a single noisy reading unless it is way off.
		const over = (x: number) => Math.abs(x) > DRIFT_TOLERANCE_MS;
		if (over(d) && (over(prev) || Math.abs(d) > 4 * DRIFT_TOLERANCE_MS)) {
			this.lastDrift = 0;
			console.debug(`[sync] drift ${d.toFixed(1)} ms, resyncing`);
			void this.reschedule();
		}
	}
}
