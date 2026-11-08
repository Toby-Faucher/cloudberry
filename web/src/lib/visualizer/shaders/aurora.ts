import type { ShaderDef } from './common.ts';

/**
 * Aurora as plotter line art: three ribbons, each a bundle of ruled hairlines following a
 * wavy baseline, the lowest lines orange and the rest graphite, cut by flickering vertical
 * rays. Mids lift and bend the ribbons, treble drives the rays, beats blink a dot-grid sky.
 * A ruled horizon with tick marks sits underneath.
 */
export const aurora: ShaderDef = {
	id: 'aurora',
	label: 'Aurora',
	scale: 0.75,
	frag: `
void main() {
	vec2 uv = p();
	float t = u_time;
	float H = hh();
	vec3 col = paper();

	// Sky: a dot grid; on each beat a fresh handful of dots flash orange.
	float pitch = 0.04;
	vec2 id = floor(uv / pitch);
	vec2 dq = uv - (id + 0.5) * pitch;
	float flash = step(0.9, hash(id + mod(u_beats, 97.0))) * step(0.3, u_beat);
	col = mix(col, mix(dim(), accent(), flash), fill(length(dq) - (1.2 + 1.5 * flash) * PX));

	for (int k = 0; k < 3; k++) {
		float f = float(k);
		float x = uv.x;
		float drift = t * (0.12 + 0.05 * f);
		float base = H - 0.34 - 0.1 * f
			+ (0.05 + 0.08 * u_mid) * sin(x * (1.4 + 0.5 * f) + drift + f * 2.0)
			+ 0.06 * (noise1(x * 2.3 + drift * 1.5 + f * 9.0) - 0.5);
		float tall = (0.15 + 0.14 * u_mid + 0.06 * u_treble + 0.1 * bar(0.15 + 0.6 * fract(x * 0.21 + f * 0.37)))
			* (0.75 + 0.25 * sin(x * 3.1 + drift * 2.0 + f * 4.0));
		float e = uv.y - base;
		float v = e / tall;
		float inside = fill(-e) * (1.0 - aastep(1.0, v));
		// Knock out what's behind this ribbon, then rule it: 9 lines, the lowest 3 orange.
		col = mix(col, paper(), inside);
		float n = v * 9.0;
		float ln = isolines(n, 0.55 + 0.5 * step(n, 0.5));
		col = mix(col, mix(ink(), accent(), step(n, 2.5)), ln * inside);
		// Rays: hairline verticals that flicker up and down with the treble.
		float cw = 0.012;
		float ci = floor(x / cw);
		float flick = floor(t * (1.5 + 6.0 * u_treble) + hash1(ci) * 9.0);
		float reach = 0.25 + 0.7 * hash1(ci * 1.3 + flick * 0.71) * (0.4 + u_treble);
		float rx = abs(fract(x / cw) - 0.5) * cw;
		col = mix(col, dim(), fill(rx - 0.5 * PX) * step(0.45, hash1(ci * 7.7)) * (1.0 - aastep(reach, v)) * inside);
		// Hard baseline.
		col = mix(col, accent(), stroke(e, 1.2 * PX) * step(0.0, 1.0 - v));
	}

	// Horizon ruler: a hairline with ticks every 0.05 (taller every 0.25), the ticks rising with bars.
	float hy = 0.12 - H;
	col = mix(col, ink(), stroke(uv.y - hy, 0.6 * PX));
	float tx = abs(fract(uv.x / 0.05 + 0.5) - 0.5) * 0.05;
	float tid = floor(uv.x / 0.05 + 0.5);
	float major = step(0.5, 1.0 - abs(fract(tid / 5.0 + 0.5) - 0.5) * 2.0 - 0.75);
	float th = 0.012 + 0.012 * major + 0.05 * bar(fract(abs(tid) * 0.031));
	col = mix(col, ink(), fill(tx - 0.6 * PX) * step(hy, uv.y) * step(uv.y, hy + th));
	outColor = vec4(col, 1.0);
}`
};
