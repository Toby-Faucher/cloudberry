/**
 * Hold-to-skip, shared by the skip button (pointer / Enter / Space) and the global N key, so
 * both drive the same ring. A tap only bumps `hint` (the button wiggles "hold to skip").
 */
import { Hold } from './game';
import type { Station } from './station.svelte';

export const skipHold = $state({ p: 0, hint: 0 });

const hold = new Hold();
let raf = 0;

export function pressSkip(station: Station) {
	if (hold.active || !station.state?.current) return;
	hold.press(performance.now());
	const tick = () => {
		const { p, done } = hold.tick(performance.now());
		skipHold.p = p;
		if (done) {
			station.skip();
			raf = 0;
			return;
		}
		raf = requestAnimationFrame(tick);
	};
	raf = requestAnimationFrame(tick);
}

export function releaseSkip() {
	cancelAnimationFrame(raf);
	raf = 0;
	if (hold.release(performance.now()) === 'tap') skipHold.hint++;
	skipHold.p = 0;
}
