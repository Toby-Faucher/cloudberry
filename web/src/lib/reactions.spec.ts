import { describe, expect, it } from 'vitest';
import { STICKERS, stickerFor } from './reactions';
import { REACTIONS } from './types';

describe('stickerFor', () => {
	it('draws a distinct sticker for every reaction the protocol sends', () => {
		const ids = REACTIONS.map((e) => stickerFor(e)?.id);
		expect(ids).toEqual(['flame', 'laugh', 'heart', 'clap', 'party', 'zzz']);
		expect(Object.keys(STICKERS)).toEqual([...REACTIONS]);
	});

	it('matches the heart with or without the variation selector', () => {
		expect(stickerFor('❤')?.id).toBe('heart');
		expect(stickerFor('❤️')?.id).toBe('heart');
	});

	it('returns null for anything it has no drawing for', () => {
		expect(stickerFor('🦆')).toBeNull();
		expect(stickerFor('')).toBeNull();
	});
});
