/**
 * Cartoon smear motion: fast-moving elements stretch along their velocity, squash across it,
 * blur a little at peak speed, then snap back. Physics runs in /smear.wasm (crates/smear, ABI in
 * the spec); if that fails to load, `createJsEngine` runs the same math in plain JS.
 *
 * How the smear is applied (CSS custom property approach):
 * - Every frame we set `--smear` on the element to `rotate(a) scale(1+s, 1/(1+s)) rotate(-a)`
 *   (`scale(1)` at rest). It smears about `transform-origin`, so keep that at the centre.
 * - If the element has no transform of its own when tracked, we also set inline
 *   `transform: var(--smear)`. Move such elements with the individual CSS `translate` /
 *   `rotate` / `scale` properties (they compose before `transform`), e.g. anime.js on `translate`.
 * - If it already has a transform, we do not touch it: add `var(--smear)` at the end of your own
 *   transform yourself, e.g. `transform: translateX(var(--x)) var(--smear)`.
 * - `filter: blur()` is written only above BLUR_MIN_PX, and only if the element had no filter.
 * - prefers-reduced-motion: reduce → track() and startBerryRain() do nothing.
 */

export const MAX_BODIES = 256;
export const MAX_RAIN = 256;
export const RAIN_STRIDE = 7; // [x, y, vx, vy, rot, size, life]

// Mirrors crates/smear/src/lib.rs: keep in sync.
const VEL_TAU = 0.03;
const SPEED_DEADZONE = 250;
const STRETCH_PER_SPEED = 1 / 2000;
const MAX_STRETCH = 1.5;
const BLUR_START = 0.35;
const BLUR_PER_STRETCH = 6;
const MAX_DT = 0.1;
const GRAVITY = 1500;
const AIR_DRAG = 0.6;
const WOBBLE_ACCEL = 140;
const WOBBLE_FREQ = 5;
const RESTITUTION = 0.45;
const FLOOR_FRICTION = 0.75;
const FADE_TIME = 0.9;
const TAU = Math.PI * 2;

/** Blur below this many px is not worth a filter (and its compositing cost). */
const BLUR_MIN_PX = 0.5;

export const stretchFor = (speed: number) =>
	Math.min(MAX_STRETCH, Math.max(0, (speed - SPEED_DEADZONE) * STRETCH_PER_SPEED));
export const blurFor = (stretch: number) => Math.max(0, (stretch - BLUR_START) * BLUR_PER_STRETCH);

/** The raw ABI with the pointers turned into views. Same shape for wasm and JS. */
export interface Engine {
	kind: 'wasm' | 'js';
	bodies(): Float32Array; // MAX_BODIES × [x, y], written by the caller
	smear(): Float32Array; // MAX_BODIES × [stretch, angle, blur, speed]
	rain(): Float32Array; // MAX_RAIN × RAIN_STRIDE, first `live` valid
	step(n: number, dt: number): void;
	reset(i: number): void;
	rain_spawn(count: number, width: number, seed: number): void;
	rain_step(dt: number, width: number, height: number): number;
}

// --- JS fallback (same math as the Rust crate) -------------------------------------

export function createJsEngine(): Engine {
	const bodies = new Float32Array(MAX_BODIES * 2);
	const out = new Float32Array(MAX_BODIES * 4);
	const st = new Float64Array(MAX_BODIES * 5); // px, py, vx, vy, angle
	const seen = new Uint8Array(MAX_BODIES);
	const p = new Float32Array(MAX_RAIN * RAIN_STRIDE);
	const aux = new Float64Array(MAX_RAIN * 3); // spin, wobble phase, bounced
	let live = 0;
	let rng = 0;
	const smooth = (v: number, raw: number, dt: number) => v + (raw - v) * (1 - Math.exp(-dt / VEL_TAU));
	const rand = () => {
		// xorshift32, identical sequence to the Rust side.
		rng ^= rng << 13;
		rng >>>= 0;
		rng ^= rng >>> 17;
		rng ^= rng << 5;
		rng >>>= 0;
		return (rng >>> 8) / (1 << 24);
	};
	const reset = (i: number) => {
		if (!(i >= 0 && i < MAX_BODIES)) return;
		st.fill(0, i * 5, i * 5 + 5);
		seen[i] = 0;
		out.fill(0, i * 4, i * 4 + 4);
	};

	return {
		kind: 'js',
		bodies: () => bodies,
		smear: () => out,
		rain: () => p,
		reset,
		step(n, dt) {
			if (!(dt > 0)) return;
			dt = Math.min(dt, MAX_DT);
			for (let i = 0; i < Math.min(n, MAX_BODIES); i++) {
				const x = bodies[i * 2], y = bodies[i * 2 + 1], s = i * 5;
				if (!Number.isFinite(x) || !Number.isFinite(y)) {
					reset(i);
					continue;
				}
				if (seen[i]) {
					st[s + 2] = smooth(st[s + 2], (x - st[s]) / dt, dt);
					st[s + 3] = smooth(st[s + 3], (y - st[s + 1]) / dt, dt);
				}
				st[s] = x;
				st[s + 1] = y;
				seen[i] = 1;
				const speed = Math.hypot(st[s + 2], st[s + 3]);
				if (speed > 1) st[s + 4] = Math.atan2(st[s + 3], st[s + 2]);
				const k = stretchFor(speed);
				out.set([k, st[s + 4], blurFor(k), speed], i * 4);
			}
		},
		rain_spawn(count, width, seed) {
			rng = (seed ^ 0x9e3779b9) >>> 0 || 1;
			width = Number.isFinite(width) ? Math.max(0, width) : 0;
			const add = Math.min(count, MAX_RAIN - live);
			for (let i = live; i < live + add; i++) {
				const size = 14 + 14 * rand();
				// Same call order as Rust's array literal: x, y, vx, vy, rot.
				const x = rand() * width, y = -size - 160 * rand();
				const vx = (rand() - 0.5) * 160, vy = rand() * 220, rot = rand() * TAU;
				p.set([x, y, vx, vy, rot, size, 1], i * RAIN_STRIDE);
				const spin = (rand() - 0.5) * 6;
				aux.set([spin, rand() * TAU, 0], i * 3);
			}
			live += add;
		},
		rain_step(dt, width, height) {
			if (!(dt > 0) || !Number.isFinite(width) || !Number.isFinite(height)) return live;
			dt = Math.min(dt, MAX_DT);
			const drag = Math.exp(-AIR_DRAG * dt);
			let kept = 0;
			for (let i = 0; i < live; i++) {
				const o = i * RAIN_STRIDE, a = i * 3;
				let [x, y, vx, vy, rot, size, life] = p.subarray(o, o + RAIN_STRIDE);
				let [spin, phase, bounced] = aux.subarray(a, a + 3);
				phase += WOBBLE_FREQ * dt;
				vx = (vx + Math.sin(phase) * WOBBLE_ACCEL * dt) * drag;
				vy = (vy + GRAVITY * dt) * drag;
				x += vx * dt;
				y += vy * dt;
				rot += spin * dt;
				const r = size * 0.5;
				if (x < r || x > width - r) {
					x = Math.min(Math.max(x, Math.min(r, width * 0.5)), Math.max(width - r, width * 0.5));
					vx = -vx * RESTITUTION;
				}
				if (y > height - r) {
					y = height - r;
					vy = -Math.abs(vy) * RESTITUTION;
					vx *= FLOOR_FRICTION;
					spin *= FLOOR_FRICTION;
					bounced = 1;
				}
				if (bounced > 0) life -= dt / FADE_TIME;
				if (!Number.isFinite(x) || !Number.isFinite(y)) life = 0;
				if (life > 0) {
					p.set([x, y, vx, vy, rot, size, life], kept * RAIN_STRIDE);
					aux.set([spin, phase, bounced], kept * 3);
					kept++;
				}
			}
			live = kept;
			return live;
		}
	};
}

// --- WASM loader -----------------------------------------------------------------

interface SmearExports {
	memory: WebAssembly.Memory;
	bodies_ptr(): number;
	smear_ptr(): number;
	rain_ptr(): number;
	step(n: number, dt: number): void;
	reset(i: number): void;
	rain_spawn(count: number, width: number, seed: number): void;
	rain_step(dt: number, width: number, height: number): number;
}

export function wasmEngine(x: SmearExports): Engine {
	// The crate never allocates so memory never grows, but re-create views if it ever does.
	const view = (ptr: () => number, len: number) => {
		let v = new Float32Array(0);
		return () => (v.buffer === x.memory.buffer ? v : (v = new Float32Array(x.memory.buffer, ptr(), len)));
	};
	return {
		kind: 'wasm',
		bodies: view(x.bodies_ptr, MAX_BODIES * 2),
		smear: view(x.smear_ptr, MAX_BODIES * 4),
		rain: view(x.rain_ptr, MAX_RAIN * RAIN_STRIDE),
		step: (n, dt) => x.step(n, dt),
		reset: (i) => x.reset(i),
		rain_spawn: (c, w, s) => x.rain_spawn(c, w, s >>> 0),
		rain_step: (dt, w, h) => x.rain_step(dt, w, h)
	};
}

async function instantiate(): Promise<SmearExports> {
	const res = await fetch('/smear.wasm');
	if (!res.ok) throw new Error(`HTTP ${res.status}`);
	let inst: WebAssembly.Instance;
	try {
		inst = (await WebAssembly.instantiateStreaming(res.clone(), {})).instance;
	} catch {
		// Wrong MIME type etc.: fall back to the buffered path.
		inst = (await WebAssembly.instantiate(await res.arrayBuffer(), {})).instance;
	}
	const x = inst.exports as unknown as SmearExports;
	if (typeof x.step !== 'function' || !(x.memory instanceof WebAssembly.Memory)) throw new Error('bad exports');
	return x;
}

let enginePromise: Promise<Engine> | null = null;
let engine: Engine | null = null;

export function loadEngine(): Promise<Engine> {
	enginePromise ??= instantiate()
		.then(wasmEngine)
		.catch((e) => {
			console.warn('smear.wasm unavailable, using JS fallback:', e);
			return createJsEngine();
		})
		.then((e) => (engine = e));
	return enginePromise;
}

const reducedMotion = () =>
	typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;

// --- DOM tracking ------------------------------------------------------------------

export type Smearable = HTMLElement | SVGElement;
export type GetPos = () => { x: number; y: number };

interface Tracked {
	slot: number;
	getPos: GetPos;
	ownTransform: boolean;
	ownFilter: boolean;
	resting: boolean;
	blurred: boolean;
	/** untrack() was called: finish once the smear has decayed so it never snaps. */
	leaving: boolean;
}

const tracked = new Map<Smearable, Tracked>();
const freeSlots: number[] = [];
let nextSlot = 0;
let raf = 0;
let lastT = 0;

/**
 * Default position: the element's bounding-box centre. That is a layout read every frame; callers
 * animating transforms (anime.js etc.) should pass getPos returning the animated values instead.
 */
const rectCentre = (el: Smearable): GetPos => () => {
	const r = el.getBoundingClientRect();
	return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
};

function track(el: Smearable, getPos?: GetPos): void {
	const t = tracked.get(el);
	if (t) {
		t.leaving = false;
		if (getPos) t.getPos = getPos;
		return;
	}
	if (reducedMotion()) return;
	const slot = freeSlots.pop() ?? nextSlot;
	if (slot >= MAX_BODIES) return; // shortcut: silently unsmeared past 256 bodies, fine for this UI
	if (slot === nextSlot) nextSlot++;
	const cs = getComputedStyle(el);
	const ownTransform = el.style.transform === '' && cs.transform === 'none';
	const ownFilter = el.style.filter === '' && cs.filter === 'none';
	el.style.setProperty('--smear', 'scale(1)');
	if (ownTransform) el.style.transform = 'var(--smear)';
	tracked.set(el, {
		slot,
		getPos: getPos ?? rectCentre(el),
		ownTransform,
		ownFilter,
		resting: true,
		blurred: false,
		leaving: false
	});
	engine?.reset(slot);
	if (engine) startLoop();
	else void loadEngine().then(startLoop);
}

/** Stops smearing `el` once its current smear has decayed (immediately if it left the DOM). */
function untrack(el: Smearable): void {
	const t = tracked.get(el);
	if (!t) return;
	if (t.resting || !el.isConnected) finish(el, t);
	else t.leaving = true;
}

function finish(el: Smearable, t: Tracked): void {
	tracked.delete(el);
	freeSlots.push(t.slot);
	el.style.removeProperty('--smear');
	if (t.ownTransform && el.style.transform === 'var(--smear)') el.style.transform = '';
	if (t.blurred) el.style.filter = '';
}

function startLoop(): void {
	if (!raf && engine && tracked.size) raf = requestAnimationFrame(frame);
}

function frame(now: number): void {
	const e = engine!;
	const dt = lastT ? (now - lastT) / 1000 : 1 / 60;
	lastT = now;
	// Read every position first, then write every style: no layout thrash between elements.
	const bodies = e.bodies();
	let n = 0;
	for (const [el, t] of tracked) {
		if (!el.isConnected) {
			finish(el, t);
			continue;
		}
		const { x, y } = t.getPos();
		bodies[t.slot * 2] = x;
		bodies[t.slot * 2 + 1] = y;
		n = Math.max(n, t.slot + 1);
	}
	e.step(n, dt);
	const out = e.smear();
	for (const [el, t] of tracked) apply(el, t, out, t.slot * 4);
	if (tracked.size) raf = requestAnimationFrame(frame);
	else raf = lastT = 0;
}

function apply(el: Smearable, t: Tracked, out: Float32Array, o: number): void {
	const s = out[o], a = out[o + 1], blur = out[o + 2];
	const resting = s < 0.001;
	if (!(resting && t.resting)) {
		el.style.setProperty(
			'--smear',
			resting ? 'scale(1)' : `rotate(${a}rad) scale(${1 + s}, ${1 / (1 + s)}) rotate(${-a}rad)`
		);
	}
	t.resting = resting;
	if (t.ownFilter) {
		const blurred = blur > BLUR_MIN_PX;
		if (blurred) el.style.filter = `blur(${blur.toFixed(1)}px)`;
		else if (t.blurred) el.style.filter = '';
		t.blurred = blurred;
	}
	if (t.leaving && resting) finish(el, t);
}

export const smear = { track, untrack };

// --- Berry rain (canvas) ---------------------------------------------------------------

let sprite: HTMLCanvasElement | null = null;

/** One cloudberry drawn once at 64 px: a cluster of amber drupelets, highlights, green sepals. */
function berrySprite(): HTMLCanvasElement {
	if (sprite) return sprite;
	const c = document.createElement('canvas');
	c.width = c.height = 64;
	const g = c.getContext('2d')!;
	g.translate(32, 34);
	const R = 24; // berry radius
	// Sepals peeking out behind the top.
	g.fillStyle = '#5E8C3A';
	for (const ang of [-2.2, -1.57, -0.95]) {
		g.save();
		g.rotate(ang + Math.PI / 2);
		g.beginPath();
		g.ellipse(0, -R * 0.95, 4, 11, 0, 0, TAU);
		g.fill();
		g.restore();
	}
	// Drupelets back to front: outer ring (shadowed), middle ring, centre.
	const rings: [number, number, number, string, string][] = [
		[11, 0.7, 0.3, '#F0912B', '#A9550F'],
		[6, 0.36, 0.5, '#F7AE4A', '#D9741A'],
		[1, 0, 0, '#FBC77A', '#F0912B']
	];
	const dr = R * 0.32;
	for (const [count, dist, phase, light, dark] of rings) {
		for (let i = 0; i < count; i++) {
			const ang = phase + (i / count) * TAU;
			const x = Math.cos(ang) * dist * R, y = Math.sin(ang) * dist * R;
			const grad = g.createRadialGradient(x - dr * 0.35, y - dr * 0.4, dr * 0.1, x, y, dr);
			grad.addColorStop(0, light);
			grad.addColorStop(1, dark);
			g.fillStyle = grad;
			g.beginPath();
			g.arc(x, y, dr, 0, TAU);
			g.fill();
			// Glossy pip on each drupelet.
			g.fillStyle = 'rgba(255, 248, 230, 0.75)';
			g.beginPath();
			g.arc(x - dr * 0.35, y - dr * 0.4, dr * 0.22, 0, TAU);
			g.fill();
		}
	}
	// Soft sheen across the upper left of the whole berry.
	const sheen = g.createRadialGradient(-R * 0.4, -R * 0.45, 0, -R * 0.4, -R * 0.45, R * 0.8);
	sheen.addColorStop(0, 'rgba(255, 255, 255, 0.35)');
	sheen.addColorStop(1, 'rgba(255, 255, 255, 0)');
	g.fillStyle = sheen;
	g.beginPath();
	g.arc(0, 0, R, 0, TAU);
	g.fill();
	return (sprite = c);
}

let raining: Promise<void> | null = null;

/**
 * Rains cloudberries over `canvas` (size it with CSS, e.g. fixed full-screen, pointer-events none).
 * Berries spawn over the first ~70% of durationMs, then the last ones bounce and fade.
 * Resolves when the canvas is clear. One rain at a time: a second call joins the running one.
 */
export function startBerryRain(
	canvas: HTMLCanvasElement,
	{ count = 160, durationMs = 4000 }: { count?: number; durationMs?: number } = {}
): Promise<void> {
	if (reducedMotion()) return Promise.resolve();
	raining ??= rain(canvas, Math.min(count, MAX_RAIN), durationMs).finally(() => (raining = null));
	return raining;
}

async function rain(canvas: HTMLCanvasElement, count: number, durationMs: number): Promise<void> {
	const e = await loadEngine();
	const ctx = canvas.getContext('2d');
	if (!ctx) return;
	const img = berrySprite();
	const dpr = window.devicePixelRatio || 1;
	const w = canvas.clientWidth, h = canvas.clientHeight;
	canvas.width = Math.round(w * dpr);
	canvas.height = Math.round(h * dpr);
	const spawnFor = durationMs * 0.7;
	let spawned = 0;
	const start = performance.now();
	let last = start;

	await new Promise<void>((done) => {
		const tick = (now: number) => {
			const elapsed = now - start;
			const want = Math.round(count * Math.min(1, elapsed / spawnFor));
			if (want > spawned) {
				e.rain_spawn(want - spawned, w, (Math.random() * 2 ** 32) >>> 0);
				spawned = want;
			}
			const live = e.rain_step((now - last) / 1000, w, h);
			last = now;
			ctx.setTransform(1, 0, 0, 1, 0, 0);
			ctx.clearRect(0, 0, canvas.width, canvas.height);
			const p = e.rain();
			for (let i = 0; i < live; i++) {
				const [x, y, vx, vy, rot, size, life] = p.subarray(i * RAIN_STRIDE, (i + 1) * RAIN_STRIDE);
				const s = stretchFor(Math.hypot(vx, vy));
				const a = Math.atan2(vy, vx);
				ctx.globalAlpha = Math.min(1, life * 1.5);
				ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
				ctx.translate(x, y);
				ctx.rotate(a);
				ctx.scale(1 + s, 1 / (1 + s));
				ctx.rotate(rot - a);
				ctx.drawImage(img, -size / 2, -size / 2, size, size);
			}
			// Safety net in case something never settles (e.g. a zero-height canvas).
			if ((spawned >= count && live === 0) || elapsed > durationMs + 4000) {
				ctx.setTransform(1, 0, 0, 1, 0, 0);
				ctx.clearRect(0, 0, canvas.width, canvas.height);
				done();
			} else requestAnimationFrame(tick);
		};
		requestAnimationFrame(tick);
	});
}
