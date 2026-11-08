/** Pure game-feel logic (phase 3 quirks), kept DOM-free so it is unit tested. Times are ms. */
import { fmtTime } from './util';

// --- 1. hold to skip -----------------------------------------------------------------

export const HOLD_MS = 600;
/** A release faster than this is a tap: show the "hold to skip" hint instead of nothing. */
export const TAP_MS = 220;

/** One press of a hold-to-confirm control. Feed it timestamps; it never fires twice. */
export class Hold {
	private start = -1;
	private fired = false;
	constructor(readonly ms = HOLD_MS) {}

	get active() {
		return this.start >= 0 && !this.fired;
	}

	press(t: number) {
		if (this.start >= 0) return; // key repeat / second pointer
		this.start = t;
		this.fired = false;
	}

	/** Progress 0..1; `done` is true exactly once, on the frame the hold completes. */
	tick(t: number): { p: number; done: boolean } {
		if (this.start < 0) return { p: 0, done: false };
		const p = Math.min(1, Math.max(0, (t - this.start) / this.ms));
		const done = p >= 1 && !this.fired;
		if (done) this.fired = true;
		return { p, done };
	}

	/** 'tap' = released almost at once, 'cancel' = let go early, 'done' = it already fired. */
	release(t: number): 'tap' | 'cancel' | 'done' | 'idle' {
		if (this.start < 0) return 'idle';
		const held = t - this.start;
		const fired = this.fired || held >= this.ms;
		this.start = -1;
		this.fired = false;
		return fired ? 'done' : held < TAP_MS ? 'tap' : 'cancel';
	}
}

// --- 3. reaction combos --------------------------------------------------------------

export const COMBO_WINDOW_MS = 2000;
export const COMBO_MIN = 3;

/** Same emoji again within 2 s (from anyone) extends the streak; anything else restarts it. */
export class Combo {
	emoji: string | null = null;
	count = 0;
	last = -Infinity;

	add(emoji: string, t: number): number {
		if (emoji === this.emoji && t - this.last <= COMBO_WINDOW_MS) this.count++;
		else {
			this.emoji = emoji;
			this.count = 1;
		}
		this.last = t;
		return this.count;
	}

	/** Visible combo at time t (shown from the 3rd hit, gone after 2 s idle). */
	shown(t: number): { emoji: string; count: number } | null {
		return this.emoji && this.count >= COMBO_MIN && t - this.last <= COMBO_WINDOW_MS
			? { emoji: this.emoji, count: this.count }
			: null;
	}
}

/** 0 at the first combo, 1 at "very hot": drives the berry → blush colour. */
export const comboHeat = (count: number) => Math.min(1, Math.max(0, (count - COMBO_MIN) / 7));

// --- 10. Konami code -----------------------------------------------------------------

const KONAMI = ['arrowup', 'arrowup', 'arrowdown', 'arrowdown', 'arrowleft', 'arrowright', 'arrowleft', 'arrowright', 'b', 'a'];

/** Returns a feed(key) that is true on the key that completes ↑↑↓↓←→←→BA. */
export function konami(): (key: string) => boolean {
	const recent: string[] = [];
	return (key) => {
		recent.push(key.toLowerCase());
		if (recent.length > KONAMI.length) recent.shift();
		const hit = recent.length === KONAMI.length && recent.every((k, i) => k === KONAMI[i]);
		if (hit) recent.length = 0;
		return hit;
	};
}

// --- 8. up-next countdown ------------------------------------------------------------

export const COUNTDOWN_MS = 10_000;

/** "0:09": whole seconds left, rounded up so it reads 0:01 until the very end. */
export const fmtCountdown = (remainingMs: number) => fmtTime(Math.ceil(Math.max(0, remainingMs) / 1000) * 1000);

/** Shown in the last 10 s of a playing track when something is queued. */
export const showCountdown = (remainingMs: number, playing: boolean, hasNext: boolean) =>
	playing && hasNext && remainingMs > 0 && remainingMs <= COUNTDOWN_MS;

// --- 5. sync readout -----------------------------------------------------------------

export type SyncLevel = 'good' | 'fair' | 'poor' | 'off';
export const syncLevel = (driftMs: number | null): SyncLevel => {
	if (driftMs === null || !Number.isFinite(driftMs)) return 'off';
	const d = Math.abs(driftMs);
	return d < 10 ? 'good' : d < 30 ? 'fair' : 'poor';
};
