/**
 * Hold-to-skip, shared by the skip button (pointer / Enter / Space) and the global N key, so
 * both drive the same ring. A tap only bumps `hint` (the button wiggles "hold to skip").
 */
import { Hold } from './game';
import type { Station } from './station.svelte';

export const skipHold = $state({ p: 0, hint: 0 });

const hold = new Hold();
let raf = 0;

let frames: Window = window;

/** `win`: whose frames drive the ring (the mini player's own window: the page's are throttled while hidden). */
export function pressSkip(station: Station, win: Window = window) {
	if (hold.active || !station.state?.current) return;
	frames = win;
	hold.press(performance.now());
	const tick = () => {
		const { p, done } = hold.tick(performance.now());
		skipHold.p = p;
		if (done) {
			station.skip();
			raf = 0;
			return;
		}
		raf = frames.requestAnimationFrame(tick);
	};
	raf = frames.requestAnimationFrame(tick);
}

export function releaseSkip() {
	frames.cancelAnimationFrame(raf);
	raf = 0;
	if (hold.release(performance.now()) === 'tap') skipHold.hint++;
	skipHold.p = 0;
}
