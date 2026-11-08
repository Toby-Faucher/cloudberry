import type { ShaderDef } from './common.ts';

/**
 * Offset-print kaleidoscope: two misregistered dot screens (orange + graphite) whose dot
 * sizes fold into six mirrored wedges and follow the spectrum outward from the centre;
 * a dial of spectrum ticks rings the art, and beats flip which screen prints orange.
 */
export const halftone: ShaderDef = {
	id: 'halftone',
	label: 'Halftone',
	scale: 0.75,
	frag: `
// A classic screen-space dot screen at angle ang; each dot's size comes from the
// kaleidoscope pattern at its cell centre: spectrum outward from the middle (bass
// innermost), swirled into six arms.
float dots(vec2 uv, float ang, float g, float band, float spin) {
	vec2 q = rot(ang) * uv;
	vec2 cell = floor(q / g);
	vec2 cc = (cell + 0.5) * g;
	vec2 wc = cc * rot(ang);
	float rad = length(wc);
	float th = atan(wc.y, wc.x);
	float fa = abs(mod(th + u_time * 0.04 * spin, TAU / 6.0) - PI / 6.0); // mirror-fold into wedges
	float lv = bar(clamp((rad - 0.15) * 0.9 + band, 0.0, 1.0));
	// Twelve fat spiral arms (six mirrored wedges), swept by the music clock.
	float arm = 0.5 + 0.5 * cos(fa * 12.0 - rad * 5.0 * spin + u_time * 0.4 * spin);
	float v = clamp(0.02 + (0.5 + 1.4 * lv) * arm * arm + 0.2 * u_beat * arm, 0.0, 1.0);
	return length(q - cc) - g * 0.5 * sqrt(v); // max: dots just kiss, never clipped by the cell
}

void main() {
	vec2 uv = p();
	float r = length(uv);
	float a = atan(uv.y, uv.x) + u_time * 0.04;


	// Beats flip which screen prints orange; every fourth beat inverts paper and ink.
	float flip = mod(u_beats, 2.0);
	float inv = step(3.0, mod(u_beats, 4.0));
	vec3 pap = mix(paper(), ink(), inv * 0.92);
	vec3 inkG = mix(ink(), paper(), inv);
	vec3 grey = mix(inkG, pap, 0.3); // the graphite screen prints a touch lighter than ink
	vec3 inkA = mix(accent(), grey, flip);
	vec3 inkB = mix(grey, accent(), flip);

	float g = 0.05;
	vec3 col = pap;
	// Two screens at different angles, a hair out of register, like offset print.
	col = mix(col, inkB, fill(dots(uv + vec2(0.006, -0.004), 0.26, g, 0.3, -1.0)));
	col = mix(col, inkA, fill(dots(uv, 0.785, g, 0.0, 1.0)));

	// A dial behind the art: paper disc, hairline rim, 72 ticks whose length is the spectrum.
	float R = 0.36 + 0.02 * u_bass;
	col = mix(col, pap, fill(r - R - 0.09));
	col = mix(col, inkG, stroke(r - R, 0.7 * PX));
	float ti = floor(a / TAU * 72.0 + 0.5);
	float ta = abs(fract(a / TAU * 72.0 + 0.5) - 0.5) * TAU / 72.0 * r;
	float tl = 0.012 + 0.07 * bar(fract(abs(ti) / 36.0));
	float tick = fill(ta - 0.7 * PX) * step(R + 0.008, r) * step(r, R + 0.008 + tl);
	col = mix(col, mix(inkG, accent(), step(0.5, bar(fract(abs(ti) / 36.0)))), tick);
	outColor = vec4(col, 1.0);
}`
};
