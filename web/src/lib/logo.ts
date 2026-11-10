/**
 * Accent-tinted logo and favicons: the mark is an orange PNG with green leaves, so we recolour only
 * the orange pixels on a canvas (hue/lightness/saturation shifted from brand orange to the accent),
 * leaving leaves, the white rim and the highlight alone.
 */

type Hsl = [number, number, number]; // h 0..360, s 0..1, l 0..1

function toHsl(r: number, g: number, b: number): Hsl {
	(r /= 255), (g /= 255), (b /= 255);
	const max = Math.max(r, g, b);
	const min = Math.min(r, g, b);
	const l = (max + min) / 2;
	const d = max - min;
	if (!d) return [0, 0, l];
	const h = 60 * (max === r ? ((g - b) / d + 6) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4);
	return [h, d / (1 - Math.abs(2 * l - 1)), l];
}

function fromHsl([h, s, l]: Hsl): [number, number, number] {
	const f = (n: number) => {
		const k = (n + h / 30) % 12;
		return Math.round(255 * (l - s * Math.min(l, 1 - l) * Math.max(-1, Math.min(k - 3, 9 - k, 1))));
	};
	return [f(0), f(8), f(4)];
}

const hexHsl = (hex: string) => toHsl(...([1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)) as [number, number, number]));
const BASE = hexHsl('#f28322'); // the orange the PNGs are drawn in

/** Recolour orange RGBA pixels in place toward `hex`. */
export function recolor(px: Uint8ClampedArray, hex: string): void {
	const [th, ts, tl] = hexHsl(hex);
	const clamp = (v: number) => Math.min(1, Math.max(0, v));
	for (let i = 0; i < px.length; i += 4) {
		if (px[i + 3] < 8) continue;
		const [h, s, l] = toHsl(px[i], px[i + 1], px[i + 2]);
		if (h <= 5 || h >= 55 || s <= 0.35 || l >= 0.92) continue; // not berry orange
		const rgb = fromHsl([(th + h - BASE[0] + 360) % 360, clamp((s * ts) / BASE[1]), clamp(l + tl - BASE[2])]);
		px.set(rgb, i);
	}
}

const cache = new Map<string, Promise<string>>();

/** `url` (same-origin PNG) recoloured toward `hex`, as a data URL. */
export function tinted(url: string, hex: string): Promise<string> {
	const key = `${url}|${hex}`;
	let p = cache.get(key);
	if (!p) {
		p = new Promise<string>((resolve, reject) => {
			const img = new Image();
			img.onload = () => {
				const c = document.createElement('canvas');
				c.width = img.naturalWidth;
				c.height = img.naturalHeight;
				const ctx = c.getContext('2d', { willReadFrequently: true });
				if (!ctx) return reject(new Error('no 2d canvas'));
				ctx.drawImage(img, 0, 0);
				const d = ctx.getImageData(0, 0, c.width, c.height);
				recolor(d.data, hex);
				ctx.putImageData(d, 0, 0);
				resolve(c.toDataURL('image/png'));
			};
			img.onerror = reject;
			img.src = url;
		});
		p.catch(() => cache.delete(key));
		cache.set(key, p);
	}
	return p;
}
