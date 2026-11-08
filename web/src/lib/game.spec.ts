import { describe, expect, it } from 'vitest';
import { Combo, Hold, HOLD_MS, comboHeat, fmtCountdown, konami, showCountdown, syncLevel } from './game';

describe('Hold (hold to skip)', () => {
	it('fires once when held for the full duration', () => {
		const h = new Hold();
		h.press(1000);
		expect(h.tick(1000 + HOLD_MS / 2)).toEqual({ p: 0.5, done: false });
		expect(h.tick(1000 + HOLD_MS)).toEqual({ p: 1, done: true });
		expect(h.tick(1000 + HOLD_MS + 50).done).toBe(false);
		expect(h.release(1000 + HOLD_MS + 80)).toBe('done');
	});

	it('cancels on an early release and tells a tap apart', () => {
		const h = new Hold();
		h.press(0);
		expect(h.release(100)).toBe('tap');
		h.press(1000);
		h.tick(1300);
		expect(h.release(1300)).toBe('cancel');
		expect(h.tick(2000)).toEqual({ p: 0, done: false });
		expect(h.release(2000)).toBe('idle');
	});

	it('ignores key repeat presses while held', () => {
		const h = new Hold();
		h.press(0);
		h.press(400); // auto-repeat must not restart the ring
		expect(h.tick(HOLD_MS).done).toBe(true);
	});
});

describe('Combo', () => {
	it('counts the same emoji within 2 s and shows from the 3rd', () => {
		const c = new Combo();
		expect(c.add('🔥', 0)).toBe(1);
		expect(c.add('🔥', 1500)).toBe(2);
		expect(c.shown(1500)).toBeNull();
		expect(c.add('🔥', 3400)).toBe(3);
		expect(c.shown(3500)).toEqual({ emoji: '🔥', count: 3 });
		expect(c.shown(5401)).toBeNull(); // 2 s idle
	});

	it('restarts on a different emoji or a gap', () => {
		const c = new Combo();
		c.add('🔥', 0);
		c.add('🔥', 100);
		expect(c.add('😂', 200)).toBe(1);
		expect(c.add('😂', 2201)).toBe(1);
	});

	it('heats from 0 to 1', () => {
		expect(comboHeat(3)).toBe(0);
		expect(comboHeat(10)).toBe(1);
		expect(comboHeat(40)).toBe(1);
	});
});

describe('konami', () => {
	const seq = ['ArrowUp', 'ArrowUp', 'ArrowDown', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'ArrowLeft', 'ArrowRight', 'b', 'a'];
	it('fires on the last key only', () => {
		const feed = konami();
		expect(seq.map(feed)).toEqual([...Array(9).fill(false), true]);
	});
	it('survives a stray prefix and overlapping starts, case-insensitive', () => {
		const feed = konami();
		expect(['x', 'ArrowUp', ...seq.slice(0, -2), 'B'].map(feed).some(Boolean)).toBe(false);
		expect(feed('A')).toBe(true);
	});
	it('does not fire on a wrong sequence', () => {
		const feed = konami();
		expect([...seq.slice(0, 8), 'a', 'b'].map(feed).some(Boolean)).toBe(false);
	});
});

describe('countdown', () => {
	it('formats whole seconds rounded up', () => {
		expect(fmtCountdown(9001)).toBe('0:10');
		expect(fmtCountdown(9000)).toBe('0:09');
		expect(fmtCountdown(1)).toBe('0:01');
		expect(fmtCountdown(-5)).toBe('0:00');
	});
	it('shows only in the last 10 s while playing with something next', () => {
		expect(showCountdown(10_000, true, true)).toBe(true);
		expect(showCountdown(10_001, true, true)).toBe(false);
		expect(showCountdown(5000, false, true)).toBe(false);
		expect(showCountdown(5000, true, false)).toBe(false);
		expect(showCountdown(0, true, true)).toBe(false);
	});
});

describe('syncLevel', () => {
	it('grades drift', () => {
		expect(syncLevel(null)).toBe('off');
		expect(syncLevel(-3)).toBe('good');
		expect(syncLevel(12)).toBe('fair');
		expect(syncLevel(30)).toBe('poor');
	});
});
