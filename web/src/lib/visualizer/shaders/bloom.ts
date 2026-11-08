import type { ShaderDef } from './common.ts';

/**
 * A domain-warped field drawn as a contour map: hairline isolines on paper, the highest
 * ground filled orange. bass -> warp, beat/energy -> the orange rises, treble -> "+"
 * markers light up on a fixed grid. With u_aura it's a disc of the same map behind the
 * now-playing art, outlined with one hairline.
 */
export const bloom: ShaderDef = {
	id: 'bloom',
	label: 'Bloom',
	scale: 0.75,
	frag: `
float fbm(vec2 q) {
	float v = 0.0, a = 0.5;
	for (int i = 0; i < 4; i++) {
		v += a * noise(q);
		q = mat2(1.6, 1.2, -1.2, 1.6) * q + 3.1;
		a *= 0.5;
	}
	return v;
}
float field(vec2 sp) {
	float t = u_time * 0.05;
	float warp = 1.0 + 2.2 * u_bass;
	vec2 q = vec2(fbm(sp + vec2(0.0, t)), fbm(sp + vec2(5.2, 1.3) - 0.8 * t));
	return fbm(sp + warp * q + vec2(1.7, 9.2) + 1.2 * t);
}

void main() {
	vec2 uv = p();
	float v = field(uv * mix(1.1, 1.8, u_aura)) + 0.1 * u_energy + 0.06 * u_beat;

	float top = 0.66;
	float hi = aastep(top, v);
	vec3 col = paper();
	col = mix(col, panel(), aastep(top - 0.12, v));
	col = mix(col, accent(), hi);
	// Contour hairlines every 1/14; inside the orange they're cut in paper, not inked.
	col = mix(col, mix(dim(), paper(), hi), isolines(v * 14.0, 0.6));

	// Treble: a fixed grid of "+" markers; the ones on high ground light up.
	float pitch = 0.09;
	vec2 cell = floor(uv / pitch);
	vec2 q = abs(uv - (cell + 0.5) * pitch);
	float plus = fill(max(min(q.x, q.y) - 0.6 * PX, max(q.x, q.y) - 0.009));
	float lit = step(0.55, v) * step(0.5, fract(hash(cell) + u_time * 0.3)) * step(0.1, u_treble);
	col = mix(col, mix(dim(), ink(), lit), plus * (1.0 - u_aura));

	if (u_aura < 0.5) {
		outColor = vec4(col, 1.0);
		return;
	}
	// Aura: a disc of the same map, its radius breathing with bass, ringed by one hairline.
	float R = 0.4 + 0.03 * u_bass + 0.02 * u_beat;
	float d = length(uv) - R;
	vec3 c = mix(ink(), col, fill(d + 1.5 * PX));
	float a = fill(d);
	outColor = vec4(c * a, a);
}`
};
