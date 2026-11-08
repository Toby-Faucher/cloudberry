/**
 * The app's motion voice: anime.js v4, plus WASM smear on fast movers (./smear.ts).
 *
 * Elements are moved with the individual CSS `translate` / `scale` / `rotate` properties (never
 * `transform`, which belongs to the smear). anime.js tweens a plain object and we write those
 * properties from it, so the smear loop can read the animated position without a layout read.
 *
 * Reduced motion: the global CSS kill in app.css does not reach JS, so every helper here checks
 * `reduced` and jumps straight to the end state (opacity is the only thing that still eases).
 */
import { animate, stagger, type AnimationParams, type JSAnimation } from 'animejs';
import { smear } from './smear';

export { stagger };

export const reduced =
	typeof matchMedia !== 'undefined' && matchMedia('(prefers-reduced-motion: reduce)').matches;

type Val = number | number[];
export interface MotionParams {
	x?: Val; // px
	y?: Val; // px
	scale?: Val;
	scaleX?: Val;
	scaleY?: Val;
	rotate?: Val; // deg
	opacity?: Val;
	duration?: number;
	delay?: number;
	ease?: string;
	/** Drop the inline styles at the end so CSS owns the element again (default true). */
	clear?: boolean;
}

const KEYS = ['x', 'y', 'scale', 'scaleX', 'scaleY', 'rotate', 'opacity'] as const;
type Key = (typeof KEYS)[number];
const REST: Record<Key, number> = { x: 0, y: 0, scale: 1, scaleX: 1, scaleY: 1, rotate: 0, opacity: 1 };

const running = new WeakMap<HTMLElement, { a: JSAnimation; smeared: boolean }>();
const first = (v: Val) => (Array.isArray(v) ? v[0] : v);
const last = (v: Val) => (Array.isArray(v) ? v[v.length - 1] : v);

function write(el: HTMLElement, o: Record<Key, number>, keys: Key[], baseScale: number) {
	if (keys.includes('x') || keys.includes('y')) el.style.translate = `${o.x}px ${o.y}px`;
	if (keys.some((k) => k.startsWith('scale')))
		el.style.scale = `${baseScale * o.scale * o.scaleX} ${baseScale * o.scale * o.scaleY}`;
	if (keys.includes('rotate')) el.style.rotate = `${o.rotate}deg`;
	if (keys.includes('opacity')) el.style.opacity = String(o.opacity);
}

function clearStyles(el: HTMLElement) {
	el.style.translate = el.style.scale = el.style.rotate = el.style.opacity = '';
}

/**
 * Animate an element (interrupts any previous motion() on it). Values are `to`, `[from, to]`, or
 * keyframes `[a, b, c, ...]`. Resolves when done (or immediately under reduced motion).
 */
export function motion(el: HTMLElement | null | undefined, params: MotionParams, withSmear = false): Promise<void> {
	if (!el) return Promise.resolve();
	const prev = running.get(el);
	prev?.a.cancel(); // interruptible: a new motion on the same element takes over at once
	if (prev?.smeared && !withSmear) smear.untrack(el);
	running.delete(el);
	const { duration = 420, delay = 0, ease = 'outExpo', clear = true } = params;
	const keys = KEYS.filter((k) => params[k] !== undefined);
	// Respect a scale set by CSS (e.g. the Halo disc), so animating it never fights the layout.
	const cssScale = parseFloat(getComputedStyle(el).scale);
	const baseScale = el.style.scale || !Number.isFinite(cssScale) ? 1 : cssScale;
	const o = { ...REST };
	for (const k of keys) o[k] = first(params[k]!);

	if (reduced) {
		// Opacity-only: fade if there is a fade, otherwise just land.
		const fade = params.opacity !== undefined && first(params.opacity) !== last(params.opacity);
		for (const k of keys) o[k] = last(params[k]!);
		if (clear) clearStyles(el);
		else write(el, o, keys, baseScale);
		if (!fade) return Promise.resolve();
		el.style.opacity = String(first(params.opacity!));
		const a = animate(el, { opacity: last(params.opacity!), duration: 200, ease: 'linear' });
		return a.then(() => clear && (el.style.opacity = '')).then(() => {});
	}

	write(el, o, keys, baseScale);
	let base = { x: 0, y: 0 };
	if (withSmear) {
		const r = el.getBoundingClientRect(); // once, at the start
		base = { x: r.left + r.width / 2 - o.x, y: r.top + r.height / 2 - o.y };
		smear.track(el, () => ({ x: base.x + o.x, y: base.y + o.y }));
	}
	const tween: AnimationParams = {
		duration,
		delay,
		ease,
		onUpdate: () => write(el, o, keys, baseScale)
	};
	for (const k of keys) {
		const v = params[k]!;
		tween[k] = (Array.isArray(v) && v.length === 2 ? v[1] : v) as AnimationParams[string]; // from already applied
	}
	const a = animate(o, tween);
	running.set(el, { a, smeared: withSmear });
	const end = () => {
		if (running.get(el)?.a !== a) return; // a newer motion owns the element now
		running.delete(el);
		if (withSmear) smear.untrack(el);
		if (clear) clearStyles(el);
	};
	return a.then(end).then(() => {});
}

/** motion() with WASM smear: the element stretches along its velocity while it moves fast. */
export const smearAnimate = (el: HTMLElement | null | undefined, params: MotionParams) => motion(el, params, true);

/** Svelte action: entrance on mount. `<li use:enter={{ x: [40, 0], opacity: [0, 1] }}>` */
export function enter(el: HTMLElement, params: MotionParams & { smear?: boolean; skip?: boolean }) {
	if (!params.skip) motion(el, params, params.smear);
}

// --- press squash: one delegated listener for every button in the app ---------------

const PRESSABLE = 'button:not(:disabled), [role="button"]:not([aria-disabled="true"]), a.btn';

/** Key travel: a short, linear-feeling dip on press and a quick return. No overshoot. */
export function installPress(root: Document | HTMLElement = document): () => void {
	if (reduced) return () => {};
	let held: HTMLElement | null = null;
	const down = (e: Event) => {
		const el = (e.target as Element | null)?.closest<HTMLElement>(PRESSABLE);
		if (!el || el.dataset.noPress !== undefined) return;
		held = el;
		motion(el, { scale: 0.96, duration: 50, ease: 'outQuad', clear: false });
	};
	const up = () => {
		const el = held;
		held = null;
		if (!el) return;
		motion(el, { scale: [0.96, 1], duration: 140, ease: 'outQuart' });
	};
	const keyDown = (e: Event) => {
		const k = e as KeyboardEvent;
		if ((k.key === 'Enter' || k.key === ' ') && !k.repeat) down(e);
	};
	root.addEventListener('pointerdown', down);
	root.addEventListener('keydown', keyDown);
	window.addEventListener('pointerup', up);
	window.addEventListener('pointercancel', up);
	window.addEventListener('keyup', up);
	return () => {
		root.removeEventListener('pointerdown', down);
		root.removeEventListener('keydown', keyDown);
		window.removeEventListener('pointerup', up);
		window.removeEventListener('pointercancel', up);
		window.removeEventListener('keyup', up);
	};
}

// --- small named moments ---------------------------------------------------------------

/** Buzz: a tight horizontal shake, like a device vibrating (pokes, hold-to-skip hint). */
export const wiggle = (el: HTMLElement | null | undefined) =>
	smearAnimate(el, { x: [0, -4, 4, -3, 3, -1, 0], duration: 300, ease: 'linear' });

/** Quick dip (someone reacted). */
export const bounce = (el: HTMLElement | null | undefined) => motion(el, { y: [0, -3, 0], duration: 220, ease: 'outQuad' });

/** Tick for counters and readouts whose value just changed. */
export const pop = (el: HTMLElement | null | undefined, amount = 0.06) =>
	motion(el, { scale: [1 + amount, 1], duration: 180, ease: 'outQuart' });
