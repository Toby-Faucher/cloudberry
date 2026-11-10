import { describe, it, expect } from 'vitest';
import { recolor } from './logo';

describe('recolor (accent logo)', () => {
	it('moves berry orange to the accent and leaves leaves, white and transparent alone', () => {
		const px = new Uint8ClampedArray([
			0xf2, 0x83, 0x22, 255, // brand orange
			0x6b, 0x9a, 0x2a, 255, // leaf green
			255, 255, 255, 255, // white rim
			0xf2, 0x83, 0x22, 0 // transparent
		]);
		recolor(px, '#1fb8d6');
		[0x1f, 0xb8, 0xd6].forEach((v, i) => expect(Math.abs(px[i] - v)).toBeLessThan(4));
		expect([...px.slice(4)]).toEqual([0x6b, 0x9a, 0x2a, 255, 255, 255, 255, 255, 0xf2, 0x83, 0x22, 0]);
	});
});
