import type { ShaderDef } from './common.ts';

/**
 * The berry as a measuring instrument would draw it: 19 spheres in a hex cluster, each a
 * hairline outline filled with a dot-matrix whose dot size is posterised Lambert light.
 * Each sphere's radius rides one band of the spectrum, bass wobbles the cluster, beats
 * squash it and push the spheres apart. Lit dots are orange, the rest graphite.
 */
export const drupelets: ShaderDef = {
	id: 'drupelets',
	label: 'Drupelets',
	scale: 0.6,
	frag: `
// Front-most sphere covering q (lower rows overlap upper). Only the 3x3 hex neighbourhood
// around q is tested, not all 19. Returns (local xy / r, d, id >= 0 if covered); dU = nearest edge.
vec4 owner(vec2 q, float sp, float spread, out float dU) {
	vec4 own = vec4(0.0, 0.0, 1e3, -1.0);
	dU = 1e3;
	float S = sp * spread;
	float fb0 = floor(q.y / (0.866 * S) + 0.5);
	float fa0 = floor(q.x / S - 0.5 * fb0 + 0.5);
	for (int j = 1; j >= -1; j--) {
		for (int k = -1; k <= 1; k++) {
			float fa = fa0 + float(k), fb = fb0 + float(j);
			float ring = max(max(abs(fa), abs(fb)), abs(fa + fb));
			if (ring > 2.0) continue;
			float i = (fb + 2.0) * 5.0 + fa + 2.0;
			vec2 c = S * vec2(fa + 0.5 * fb, 0.866 * fb);
			c += 0.012 * (0.3 + u_bass) * vec2(sin(u_time * 2.1 + i * 1.7), cos(u_time * 1.9 + i * 2.3));
			float lv = bar(0.06 + ring * 0.22 + 0.06 * hash1(i));
			float r = sp * (0.5 + 0.22 * lv) * (1.0 + 0.08 * u_bass);
			float d = length(q - c) - r;
			dU = min(dU, d);
			if (d < 0.0) own = vec4((q - c) / r, d, i);
		}
	}
	return own;
}

void main() {
	vec2 uv = p();
	float sp = 0.17 * mix(1.0, 1.3, step(u_res.x, u_res.y)); // portrait (phones): bigger, so it peeks past the art
	vec2 c0 = vec2(0.0, -0.04 - 0.015 * u_beat);
	vec2 sq = vec2(1.0 + 0.07 * u_beat, 1.0 - 0.09 * u_beat);
	float spread = 1.0 + 0.16 * u_beat;
	vec2 q = (uv - c0) / sq;

	vec3 col = paper();
	// Bench grid behind: faint dots every 0.05.
	vec2 bg = abs(fract(uv / 0.05 + 0.5) - 0.5) * 0.05;
	col = mix(col, dim(), fill(length(bg) - 1.1 * PX));

	// Dot matrix on a fixed screen grid; each dot's size is the posterised light where it sits.
	float pitch = 0.011;
	vec2 cell = (floor(uv / pitch) + 0.5) * pitch;
	float dP;
	vec4 op = owner(q, sp, spread, dP);
	if (op.w >= 0.0) {
		vec3 n = vec3(op.xy, sqrt(max(0.0, 1.0 - dot(op.xy, op.xy))));
		float lam = max(0.0, dot(n, normalize(vec3(-0.5, 0.6, 0.65))));
		lam = floor(lam * 4.0 + 0.5) / 4.0; // posterised: four dot sizes
		float rad = pitch * (0.12 + 0.36 * lam);
		col = mix(col, mix(ink(), accent(), step(0.5, lam)), fill(length(uv - cell) - rad));
		// The owner's rim: the front sphere's arc over the ones behind it.
		col = mix(col, ink(), stroke(op.z, 0.6 * PX));
	}
	col = mix(col, ink(), stroke(dP, 0.6 * PX));
	outColor = vec4(col, 1.0);
}`
};
