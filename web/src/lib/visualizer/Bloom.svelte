<script lang="ts">
	import { onMount } from 'svelte';
	import { cssColor, fit, follow, frames, reducedMotion, type Frame } from './analysis.ts';

	/**
	 * Domain-warped noise in warm orange / --track-tint, rendered with WebGL2 at
	 * `scale`x resolution and upscaled. `aura` fades out radially (behind the art),
	 * `fill` covers its box (backdrop, lean-back). Without WebGL2, a CSS gradient
	 * breathes with `energy` instead.
	 */
	let { variant = 'fill', scale = 0.5 }: { variant?: 'aura' | 'fill'; scale?: number } = $props();

	let canvas = $state<HTMLCanvasElement>();
	let fallback = $state<HTMLDivElement>();
	let gl2 = $state(true);

	const VERT = `#version 300 es
in vec2 pos;
void main() { gl_Position = vec4(pos, 0.0, 1.0); }`;

	const FRAG = `#version 300 es
precision highp float;
uniform vec2 uRes;
uniform float uTime, uBass, uBeat, uTreble, uEnergy, uAura, uDark, uZoom;
uniform vec3 uTint, uBg;
out vec4 outColor;

float hash(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}
float noise(vec2 p) {
	vec2 i = floor(p), f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1, 0)), u.x), mix(hash(i + vec2(0, 1)), hash(i + vec2(1, 1)), u.x), u.y);
}
float fbm(vec2 p) {
	float v = 0.0, a = 0.5;
	for (int i = 0; i < 5; i++) {
		v += a * noise(p);
		p = mat2(1.6, 1.2, -1.2, 1.6) * p + 3.1;
		a *= 0.5;
	}
	return v;
}

void main() {
	vec2 p = (gl_FragCoord.xy - 0.5 * uRes) / min(uRes.x, uRes.y);
	float t = uTime * 0.05;
	float warp = 1.1 + 2.4 * uBass;
	vec2 sp = p * uZoom;
	vec2 q = vec2(fbm(sp + vec2(0.0, t)), fbm(sp + vec2(5.2, 1.3) - 0.8 * t));
	vec2 r = vec2(fbm(sp + warp * q + vec2(1.7, 9.2) + 1.2 * t), fbm(sp + warp * q + vec2(8.3, 2.8) - t));
	float f = fbm(sp + warp * r);

	vec3 tint = uTint;
	vec3 amber = mix(tint, vec3(1.0, 0.8, 0.5), 0.5);
	vec3 ember = mix(tint * vec3(0.6, 0.32, 0.26), vec3(0.5, 0.1, 0.16), 0.35);
	// Posterised: flat tone bands with crisp edges (sticker look), not a soft wash.
	vec3 col = mix(ember, tint, step(0.45, f));
	col = mix(col, amber, step(0.66, f + 0.3 * length(r) - 0.25));

	// Treble: sparse glints that twinkle on the bright ridges.
	vec2 g = p * 26.0;
	vec2 id = floor(g);
	float h = hash(id);
	vec2 off = vec2(hash(id + 7.1), hash(id + 3.7)) - 0.5;
	float d = length(fract(g) - 0.5 - off * 0.6);
	float tw = 0.5 + 0.5 * sin(uTime * (2.0 + 4.0 * h) + h * 40.0);
	float spark = step(d, 0.12) * step(0.8, h) * step(0.5, tw) * step(0.55, f);
	col += vec3(1.0, 0.93, 0.82) * spark * uTreble * 1.8;

	float lift = 0.5 + 0.5 * uEnergy + 0.3 * uBeat;
	// Light: soft wash. Dark: deep valleys so only the ridges glow, like embers.
	float glow = lift * mix(0.3 + 0.7 * f, 1.2 * pow(smoothstep(0.2, 0.95, f), 1.6), uDark);
	glow = floor(clamp(glow, 0.0, 1.0) * 3.0 + 0.5) / 3.0;
	vec3 rgb = mix(uBg, col, glow) + amber * step(0.5, uBeat) * 0.1;
	// Aura: a crisp blob whose edge wobbles with the field, like a die-cut shape.
	float m = mix(1.0, step(length(p), 0.36 + 0.12 * f + 0.04 * uBass), uAura);
	outColor = vec4(rgb * m, m);
}`;

	onMount(() => {
		const el = canvas!;
		const gl = el.getContext('webgl2', { premultipliedAlpha: true, antialias: false, alpha: true });
		let prog: WebGLProgram | null = null;
		if (gl) prog = build(gl);
		if (!gl || !prog) {
			gl2 = false;
			return runFallback();
		}

		const u = (n: string) => gl.getUniformLocation(prog!, n);
		const U = {
			res: u('uRes'), time: u('uTime'), bass: u('uBass'), beat: u('uBeat'), treble: u('uTreble'),
			energy: u('uEnergy'), aura: u('uAura'), dark: u('uDark'), tint: u('uTint'), bg: u('uBg'),
			zoom: u('uZoom')
		};
		const buf = gl.createBuffer();
		gl.bindBuffer(gl.ARRAY_BUFFER, buf);
		gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
		const loc = gl.getAttribLocation(prog, 'pos');
		gl.enableVertexAttribArray(loc);
		gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
		gl.useProgram(prog);

		const dark = matchMedia('(prefers-color-scheme: dark)');
		let bg = cssColor(getComputedStyle(el).getPropertyValue('--bg') || '#f5f4f2');
		const onScheme = () => (bg = cssColor(getComputedStyle(el).getPropertyValue('--bg')));
		dark.addEventListener('change', onScheme);

		const ro = new ResizeObserver(() => fit(el, scale));
		ro.observe(el);

		// Extra easing on top of the analysis so the field flows rather than twitches.
		let t = 0, bass = 0, beat = 0, energy = 0, treble = 0;
		const draw = (f: Frame) => {
			const dt = f.dt;
			bass = follow(bass, f.bass, dt, 0.12, 0.5);
			beat = follow(beat, f.beat, dt, 0.04, 0.3);
			energy = follow(energy, f.energy, dt, 0.2, 0.6);
			treble = follow(treble, f.treble, dt, 0.08, 0.3);
			// Music makes the field drift faster; reduced motion keeps it still (dt=0).
			if (!reducedMotion) t += dt * (0.6 + 1.2 * energy);
			else ({ bass, beat, energy, treble } = f);

			gl.viewport(0, 0, el.width, el.height);
			gl.uniform2f(U.res, el.width, el.height);
			gl.uniform1f(U.time, t + 20);
			gl.uniform1f(U.bass, bass);
			gl.uniform1f(U.beat, beat);
			gl.uniform1f(U.treble, treble);
			gl.uniform1f(U.energy, energy);
			gl.uniform1f(U.aura, variant === 'aura' ? 1 : 0);
			// Big surfaces get bigger, calmer shapes.
			gl.uniform1f(U.zoom, variant === 'aura' ? 1.5 : 0.95);
			gl.uniform1f(U.dark, dark.matches ? 1 : 0);
			gl.uniform3f(U.tint, f.tint[0] / 255, f.tint[1] / 255, f.tint[2] / 255);
			gl.uniform3f(U.bg, bg[0] / 255, bg[1] / 255, bg[2] / 255);
			gl.drawArrays(gl.TRIANGLES, 0, 3);
		};

		const unsub = frames.subscribe(draw);
		return () => {
			unsub();
			ro.disconnect();
			dark.removeEventListener('change', onScheme);
			gl.getExtension('WEBGL_lose_context')?.loseContext();
		};
	});

	function build(gl: WebGL2RenderingContext): WebGLProgram | null {
		const sh = (type: number, src: string) => {
			const s = gl.createShader(type)!;
			gl.shaderSource(s, src);
			gl.compileShader(s);
			if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) console.warn('[bloom]', gl.getShaderInfoLog(s));
			return s;
		};
		const p = gl.createProgram()!;
		gl.attachShader(p, sh(gl.VERTEX_SHADER, VERT));
		gl.attachShader(p, sh(gl.FRAGMENT_SHADER, FRAG));
		gl.linkProgram(p);
		return gl.getProgramParameter(p, gl.LINK_STATUS) ? p : null;
	}

	/** No WebGL2: a soft tinted radial gradient that swells with energy. */
	function runFallback() {
		let energy = 0;
		const unsub = frames.subscribe((f) => {
			const el = fallback;
			if (!el) return;
			energy = follow(energy, f.energy + 0.3 * f.beat, f.dt, 0.1, 0.5);
			el.style.setProperty('--tint', `rgb(${f.tint.map(Math.round).join(' ')})`);
			el.style.opacity = (0.45 + 0.55 * energy).toFixed(3);
			el.style.transform = `scale(${(0.92 + 0.14 * energy).toFixed(3)})`;
		});
		return unsub;
	}
</script>

{#if gl2}
	<canvas bind:this={canvas} class={variant} aria-hidden="true"></canvas>
{:else}
	<div bind:this={fallback} class="css {variant}" aria-hidden="true"></div>
{/if}

<style>
	canvas,
	.css {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}
	.aura {
		left: -30%;
		top: -30%;
		width: 160%;
		height: 160%;
	}
	.css {
		--tint: var(--berry-500);
		/* Flat tone bands, hard stops: same posterised look as the shader. */
		background:
			radial-gradient(40% 34% at 35% 40%, var(--berry-300) 0 99%, transparent 100%),
			radial-gradient(70% 60% at 65% 65%, var(--tint) 0 99%, transparent 100%);
		will-change: transform, opacity;
	}
	.css.aura {
		border-radius: 50%;
		background: radial-gradient(closest-side, var(--tint) 0 60%, transparent 61%);
	}
</style>
