import type { ShaderDef } from './common.ts';

/**
 * An oscilloscope bench: a ruled graticule, the waveform traced across the screen with
 * phosphor persistence done as stepped echo traces (no blur), a Lissajous figure in the
 * top-right, and a 64-column LED spectrum meter along the bottom. A TRIG LED fires on beats.
 */
export const scope: ShaderDef = {
	id: 'scope',
	label: 'Scope',
	scale: 0.75,
	frag: `
// Signed distance to y = f(x) for the waveform trace, amplitude amp, echo k.
float trace(vec2 uv, float y0, float amp, float W, float k) {
	float e = 0.002;
	float x = (uv.x + W) / (2.0 * W);
	float sh = 0.006 * k; // each echo samples a little earlier along the trace
	float f = amp * wave(x - sh);
	float df = amp * (wave(x - sh + e) - wave(x - sh - e)) / (2.0 * e * 2.0 * W);
	return (uv.y - y0 - f) / sqrt(1.0 + df * df);
}

void main() {
	vec2 uv = p();
	float W = hw(), H = hh();
	vec3 col = paper();

	// Graticule: 0.1 divisions in dim hairlines, dotted subdivisions on the centre axes.
	vec2 g = abs(fract(uv / 0.1 + 0.5) - 0.5) * 0.1;
	col = mix(col, dim(), fill(min(g.x, g.y) - 0.5 * PX) * 0.7);
	vec2 sub = abs(fract(uv / 0.02 + 0.5) - 0.5) * 0.02;
	float axisDots = fill(length(vec2(sub.x, uv.y)) - 1.4 * PX) + fill(length(vec2(uv.x, sub.y)) - 1.4 * PX);
	col = mix(col, ink(), min(axisDots, 1.0));

	// Main trace across the middle (it shows either side of the art), oldest echo first.
	float amp = 0.12 + 0.28 * u_energy + 0.08 * u_beat;
	for (int k = 3; k >= 0; k--) {
		float fk = float(k);
		float d = trace(uv, 0.0, amp * (1.0 - 0.12 * fk), W, fk);
		vec3 c = fk < 0.5 ? accent() : mix(dim(), ink(), 0.35 - 0.1 * fk);
		col = mix(col, c, stroke(d, (fk < 0.5 ? 1.4 : 0.7) * PX));
	}

	// Lissajous: x = wave(t), y = wave(t + quarter), 40 segments.
	float port = step(u_res.x, u_res.y);
	// Top-right clear of the exit button; on a phone, centred above the art.
	vec2 lc = mix(vec2(W - 0.24, H - 0.27), vec2(0.0, H - 0.25), port);
	float lr = 0.15 + 0.03 * u_bass;
	vec2 lq = abs(uv - lc);
	float best = 1e3;
	if (max(lq.x, lq.y) < lr + 0.03) { // only pixels in the box pay for the loop
	vec2 prev = lc + lr * vec2(wave(0.0), wave(0.25)) * 1.6;
	for (int i = 1; i < 40; i++) {
		float t = float(i) / 52.0;
		vec2 cur = lc + lr * vec2(wave(t), wave(t + 0.25)) * 1.6;
		vec2 pa = uv - prev, ba = cur - prev;
		best = min(best, length(pa - ba * clamp(dot(pa, ba) / max(dot(ba, ba), 1e-8), 0.0, 1.0)));
		prev = cur;
	}
	}
	float frame = max(lq.x, lq.y) - lr - 0.02;
	col = mix(col, paper(), fill(frame));
	col = mix(col, ink(), stroke(frame, 0.5 * PX));
	col = mix(col, dim(), fill(min(lq.x, lq.y) - 0.5 * PX) * fill(frame));
	col = mix(col, accent(), fill(best - 1.2 * PX) * fill(frame));

	// LED spectrum meter: 64 columns of square segments, lit up to each bar; peak segment in ink.
	float mx = (uv.x + W) / (2.0 * W);
	float ci = floor(mx * 64.0);
	float cw = 2.0 * W / 64.0;
	float base = -H + 0.13 + 0.1 * port; // phones: clear of the picker
	float seg = 0.016;
	float row = floor((uv.y - base) / seg);
	vec2 lp = vec2(fract(mx * 64.0) * cw, fract((uv.y - base) / seg) * seg) - vec2(cw, seg) * 0.5;
	float cellBox = max(abs(lp.x) - cw * 0.32, abs(lp.y) - seg * 0.3);
	float rows = 9.0;
	float inMeter = step(0.0, row) * step(row, rows - 1.0);
	float lvl = bar((ci + 0.5) / 64.0) * rows;
	float on = step(row + 0.5, lvl);
	col = mix(col, mix(panel(), accent(), on), fill(cellBox) * inMeter);

	// TRIG LED, top-left.
	vec2 led = uv - vec2(-W + 0.06, H - 0.06);
	col = mix(col, ink(), stroke(length(led) - 0.012, 0.5 * PX));
	col = mix(col, mix(panel(), accent(), step(0.3, u_beat)), fill(length(led) - 0.009));
	outColor = vec4(col, 1.0);
}`
};
