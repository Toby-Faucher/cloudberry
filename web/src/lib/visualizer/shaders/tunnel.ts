import type { ShaderDef } from './common.ts';

/**
 * A wireframe tube rushing out from behind the art: hairline-outlined rings and radial
 * spokes, every fourth ring solid orange. Each ring's thickness is one of the 64 bars,
 * speed follows energy, beats lurch the tube forward, treble ripples the ring edges.
 */
export const tunnel: ShaderDef = {
	id: 'tunnel',
	label: 'Tunnel',
	scale: 0.75,
	frag: `
const float FOC = 2.4;

// Signed screen-space distance to the ring band at uv; id/fr out.
float band(vec2 uv, out float id) {
	float r = length(uv);
	float a = atan(uv.y, uv.x);
	float z0 = FOC / max(r, 1e-3);
	a += 0.25 * sin(z0 * 0.4 + u_time * 0.2) * (0.3 + u_mid); // a slow twist down the tube
	float rs = r * (1.0 + (0.008 + 0.03 * u_treble) * sin(12.0 * a + u_time * 0.3));
	float s = FOC / max(rs, 1e-3) + u_time * 0.9 + 0.35 * u_beat;
	id = floor(s);
	float fr = fract(s);
	float th = 0.3 + 0.6 * bar(fract(id * 0.618));
	// Depth units -> output pixels via the real gradient (twist and frills included), -> p() units.
	return max(-fr, fr - th) / max(fwidth(s), 1e-5) * PX;
}

void main() {
	vec2 uv = p();
	uv -= 0.03 * vec2(sin(u_time * 0.3), cos(u_time * 0.23)) * (0.5 + u_bass);
	float r = length(uv);
	float a = atan(uv.y, uv.x);

	float id;
	float d = band(uv, id);
	int k = int(mod(id, 4.0));
	vec3 col = paper();
	// Wireframe: 24 radial hairline spokes in the gaps...
	float spoke = abs(fract(a / TAU * 24.0 + 0.5) - 0.5) * TAU / 24.0 * r;
	col = mix(col, dim(), fill(spoke - 0.5 * PX));
	// ...rings outlined in ink, every 4th solid orange and every 4th+2 solid graphite.
	vec3 c = k == 0 ? accent() : k == 2 ? ink() : paper();
	col = mix(col, c, fill(d));
	col = mix(col, ink(), stroke(d, 0.6 * PX));

	// The far end: a solid disc with an orange hairline, before rings get thinner than a pixel.
	col = mix(col, panel(), fill(r - 0.2));
	col = mix(col, accent(), stroke(r - 0.2, 0.8 * PX));
	outColor = vec4(col, 1.0);
}`
};
