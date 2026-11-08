/**
 * One WebGL2 context drawing any of the shader pack: fullscreen triangle, shared
 * uniforms, bars (64x1) and waveform (256x1) float textures, programs compiled on
 * first use, and a 400 ms crossfade when the shader changes.
 */
import { BARS, WAVE_POINTS, cssColor, fit, follow, type Frame, type Rgb } from './analysis.ts';
import { PRELUDE, SHADERS, type ShaderDef } from './shaders/index.ts';

const VERT = `#version 300 es
in vec2 pos;
void main() { gl_Position = vec4(pos, 0.0, 1.0); }`;

export const FADE_MS = 400;

/** The music values the shaders see: the analysis eased a bit more so fields flow, not twitch. */
export interface Smoothed {
	time: number;
	bass: number;
	mid: number;
	treble: number;
	energy: number;
	beat: number;
	beats: number; // onsets seen so far (shaders flip colours etc. on its integer part)
	lastBeat: number;
}

export const smoothed = (): Smoothed => ({ time: 20, bass: 0, mid: 0, treble: 0, energy: 0, beat: 0, beats: 0, lastBeat: 0 });

/** Advance `s` by one analysis frame. `still` = reduced motion: copy, don't animate. */
export function pack(s: Smoothed, f: Pick<Frame, 'bass' | 'mid' | 'treble' | 'energy' | 'beat' | 'dt'>, still = false): Smoothed {
	if (f.beat > s.lastBeat + 0.5) s.beats++;
	s.lastBeat = f.beat;
	if (still) {
		s.bass = f.bass;
		s.mid = f.mid;
		s.treble = f.treble;
		s.energy = f.energy;
		s.beat = 0;
		return s;
	}
	const dt = f.dt;
	s.bass = follow(s.bass, f.bass, dt, 0.12, 0.5);
	s.mid = follow(s.mid, f.mid, dt, 0.1, 0.5);
	s.treble = follow(s.treble, f.treble, dt, 0.08, 0.3);
	s.energy = follow(s.energy, f.energy, dt, 0.2, 0.6);
	s.beat = follow(s.beat, f.beat, dt, 0.04, 0.3);
	// Music makes everything drift faster.
	s.time += dt * (0.6 + 1.2 * s.energy);
	return s;
}

const UNIFORMS = ['u_res', 'u_time', 'u_bass', 'u_mid', 'u_treble', 'u_energy', 'u_beat', 'u_beats', 'u_dark', 'u_aura', 'u_tint', 'u_accent', 'u_bg', 'u_bars', 'u_wave'] as const;
type Prog = { p: WebGLProgram; u: Record<(typeof UNIFORMS)[number], WebGLUniformLocation | null> };

export interface Runner {
	/** Switch shader (crossfades unless `instant`). */
	use(id: string, instant?: boolean): void;
	/** Draw one frame from `f`. `still` = reduced motion (no easing, no time advance). */
	draw(f: Frame, still?: boolean): void;
	/** Override the animation clock (reduced motion: one stable still per track). */
	setTime(t: number): void;
	destroy(): void;
}

/** Returns null when WebGL2 (or the first program) isn't available: callers show the CSS fallback. */
export function createRunner(canvas: HTMLCanvasElement, opts: { aura?: boolean; scale?: number } = {}): Runner | null {
	const gl = canvas.getContext('webgl2', { premultipliedAlpha: true, antialias: false, alpha: !!opts.aura });
	if (!gl) return null;

	const progs = new Map<string, Prog | null>();
	const compile = (def: ShaderDef): Prog | null => {
		if (progs.has(def.id)) return progs.get(def.id)!;
		const sh = (type: number, src: string) => {
			const s = gl.createShader(type)!;
			gl.shaderSource(s, src);
			gl.compileShader(s);
			if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) console.warn(`[shader ${def.id}]`, gl.getShaderInfoLog(s));
			return s;
		};
		const p = gl.createProgram()!;
		gl.attachShader(p, sh(gl.VERTEX_SHADER, VERT));
		gl.attachShader(p, sh(gl.FRAGMENT_SHADER, PRELUDE + def.frag));
		gl.bindAttribLocation(p, 0, 'pos');
		gl.linkProgram(p);
		let prog: Prog | null = null;
		if (gl.getProgramParameter(p, gl.LINK_STATUS)) {
			const u = {} as Prog['u'];
			for (const n of UNIFORMS) u[n] = gl.getUniformLocation(p, n);
			prog = { p, u };
		}
		progs.set(def.id, prog);
		return prog;
	};
	const find = (id: string) => SHADERS.find((s) => s.id === id) ?? SHADERS[0];

	if (!compile(SHADERS[0])) return null; // bloom is the floor: no bloom, no WebGL path

	const buf = gl.createBuffer();
	gl.bindBuffer(gl.ARRAY_BUFFER, buf);
	gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
	gl.enableVertexAttribArray(0);
	gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

	// R16F is filterable in core WebGL2 and takes FLOAT uploads, so bars interpolate smoothly.
	const tex = (unit: number, w: number) => {
		const t = gl.createTexture();
		gl.activeTexture(gl.TEXTURE0 + unit);
		gl.bindTexture(gl.TEXTURE_2D, t);
		gl.texImage2D(gl.TEXTURE_2D, 0, gl.R16F, w, 1, 0, gl.RED, gl.FLOAT, null);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
		gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
		return t;
	};
	tex(0, BARS);
	tex(1, WAVE_POINTS);

	const dark = matchMedia('(prefers-color-scheme: dark)');
	const css = (name: string, fb: string) => cssColor(getComputedStyle(canvas).getPropertyValue(name) || fb);
	let bg: Rgb = css('--bg', '#4b6b34');
	let accent: Rgb = css('--accent', '#f0912b');
	const onScheme = () => {
		bg = css('--bg', '#4b6b34');
		accent = css('--accent', '#f0912b');
	};
	dark.addEventListener('change', onScheme);

	let cur = SHADERS[0];
	let prev: ShaderDef | null = null;
	let fadeStart = 0;
	const scaleOf = (d: ShaderDef) => opts.scale ?? d.scale;
	const ro = new ResizeObserver(() => fit(canvas, scaleOf(cur)));
	ro.observe(canvas);

	const s = smoothed();

	const render = (def: ShaderDef, f: Frame, alpha: number) => {
		const pr = compile(def) ?? compile(SHADERS[0])!;
		const u = pr.u;
		gl.useProgram(pr.p);
		gl.uniform2f(u.u_res, canvas.width, canvas.height);
		gl.uniform1f(u.u_time, s.time);
		gl.uniform1f(u.u_bass, s.bass);
		gl.uniform1f(u.u_mid, s.mid);
		gl.uniform1f(u.u_treble, s.treble);
		gl.uniform1f(u.u_energy, s.energy);
		gl.uniform1f(u.u_beat, s.beat);
		gl.uniform1f(u.u_beats, s.beats);
		gl.uniform1f(u.u_dark, dark.matches ? 1 : 0);
		gl.uniform1f(u.u_aura, opts.aura ? 1 : 0);
		gl.uniform3f(u.u_tint, f.tint[0] / 255, f.tint[1] / 255, f.tint[2] / 255);
		gl.uniform3f(u.u_accent, accent[0] / 255, accent[1] / 255, accent[2] / 255);
		gl.uniform3f(u.u_bg, bg[0] / 255, bg[1] / 255, bg[2] / 255);
		gl.uniform1i(u.u_bars, 0);
		gl.uniform1i(u.u_wave, 1);
		if (alpha < 1) {
			gl.enable(gl.BLEND);
			gl.blendColor(0, 0, 0, alpha);
			gl.blendFunc(gl.CONSTANT_ALPHA, gl.ONE_MINUS_CONSTANT_ALPHA);
		} else gl.disable(gl.BLEND);
		gl.drawArrays(gl.TRIANGLES, 0, 3);
	};

	return {
		use(id, instant = false) {
			const next = find(id);
			if (next === cur) return;
			prev = instant ? null : cur;
			fadeStart = performance.now();
			cur = next;
			fit(canvas, scaleOf(cur));
		},
		draw(f, still = false) {
			pack(s, f, still);
			gl.activeTexture(gl.TEXTURE0);
			gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, BARS, 1, gl.RED, gl.FLOAT, f.bars);
			gl.activeTexture(gl.TEXTURE1);
			gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, WAVE_POINTS, 1, gl.RED, gl.FLOAT, f.waveform);
			gl.viewport(0, 0, canvas.width, canvas.height);
			if (opts.aura) {
				gl.clearColor(0, 0, 0, 0);
				gl.clear(gl.COLOR_BUFFER_BIT);
			}
			const k = prev && !still ? (performance.now() - fadeStart) / FADE_MS : 1;
			if (prev && k < 1) render(prev, f, 1);
			else prev = null;
			render(cur, f, prev ? k * k * (3 - 2 * k) : 1);
		},
		setTime(t) {
			s.time = t;
		},
		destroy() {
			ro.disconnect();
			dark.removeEventListener('change', onScheme);
			gl.getExtension('WEBGL_lose_context')?.loseContext();
		}
	};
}
