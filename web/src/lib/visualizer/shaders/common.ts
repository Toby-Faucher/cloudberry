/**
 * GLSL shared by every lean-back shader: the runner's uniforms, the instrument palette
 * (paper, graphite, one hot orange) and a few helpers. Edges are hard and AA'd over
 * ~1 px (`fill`, `aastep`, `stroke`, `isolines`), never blurred.
 */
export const PRELUDE = `#version 300 es
precision highp float;
uniform vec2 u_res;
uniform float u_time, u_bass, u_mid, u_treble, u_energy, u_beat, u_beats, u_dark, u_aura;
uniform vec3 u_tint, u_accent, u_bg;
uniform sampler2D u_bars, u_wave;
out vec4 outColor;

#define PI 3.14159265
#define TAU 6.2831853
// One output pixel in the centred, min-axis-normalised space p() returns.
#define PX (1.0 / min(u_res.x, u_res.y))

const vec3 ORANGE = vec3(0.941, 0.569, 0.169);

// Light: off-white paper, graphite ink. Dark: near-black graphite paper, off-white ink.
vec3 paper() { return mix(vec3(0.925, 0.922, 0.910), vec3(0.071, 0.075, 0.078), u_dark); }
vec3 panel() { return mix(vec3(0.855, 0.851, 0.839), vec3(0.122, 0.127, 0.133), u_dark); }
vec3 ink() { return mix(vec3(0.118, 0.122, 0.129), vec3(0.902, 0.898, 0.882), u_dark); }
vec3 dim() { return mix(vec3(0.66, 0.66, 0.65), vec3(0.27, 0.28, 0.29), u_dark); }
// The one accent: cloudberry orange, nudged a little toward the cover's tint.
vec3 accent() { return mix(ORANGE, u_tint, 0.18); }

vec2 p() { return (gl_FragCoord.xy - 0.5 * u_res) / min(u_res.x, u_res.y); }
// Half extents of p() space: (aspect/2, 0.5) landscape, (0.5, aspect/2) portrait. Layouts anchor
// to these so on a phone the busy parts sit above and below the art, not behind it.
float hw() { return 0.5 * u_res.x / min(u_res.x, u_res.y); }
float hh() { return 0.5 * u_res.y / min(u_res.x, u_res.y); }
float bar(float x) { return textureLod(u_bars, vec2(clamp(x, 0.0, 1.0) * (63.0 / 64.0) + 0.5 / 64.0, 0.5), 0.0).r; }
float wave(float x) { return textureLod(u_wave, vec2(clamp(x, 0.0, 1.0), 0.5), 0.0).r; }
// Inside of an SDF (d < 0) with a crisp ~1px edge.
float fill(float d) { return 1.0 - smoothstep(-PX, PX, d); }
// A line of half-width w (p units) along d = 0.
float stroke(float d, float w) { return fill(abs(d) - w); }
// Posterise without jaggies: a hard step AA'd over one pixel of x's own gradient.
float aastep(float e, float x) { float w = fwidth(x) * 0.75; return smoothstep(e - w, e + w, x); }
// Hairlines on the integer crossings of x (contours, ruled lines), ~px wide whatever the gradient.
float isolines(float x, float px) {
	float f = abs(fract(x + 0.5) - 0.5) / max(fwidth(x), 1e-5);
	return 1.0 - smoothstep(px - 0.5, px + 0.5, f);
}
float hash(vec2 q) {
	vec3 p3 = fract(vec3(q.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}
float hash1(float n) { return fract(sin(n * 127.1) * 43758.5453); }
float noise(vec2 q) {
	vec2 i = floor(q), f = fract(q);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1, 0)), u.x), mix(hash(i + vec2(0, 1)), hash(i + vec2(1, 1)), u.x), u.y);
}
float noise1(float x) {
	float i = floor(x), f = fract(x);
	return mix(hash1(i), hash1(i + 1.0), f * f * (3.0 - 2.0 * f));
}
mat2 rot(float a) { float c = cos(a), s = sin(a); return mat2(c, s, -s, c); }
`;

export interface ShaderDef {
	id: string;
	label: string;
	/** Fraction of CSS px (× DPR, capped at 2) the canvas renders at. */
	scale: number;
	/** GLSL after PRELUDE: must define `void main()`. */
	frag: string;
}
