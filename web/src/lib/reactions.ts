// The wire still carries emoji strings (protocol unchanged); the UI draws a pictogram for each
// (the id is its Icon name). "Sticker" names are kept from Phase 4 so callers and tests stay put.
import { REACTIONS, type Emoji } from './types';

export type StickerId = 'flame' | 'laugh' | 'heart' | 'clap' | 'party' | 'zzz';

export const STICKERS: Record<Emoji, { id: StickerId; label: string }> = {
	'🔥': { id: 'flame', label: 'Fire' },
	'😂': { id: 'laugh', label: 'Crying laughing' },
	'❤️': { id: 'heart', label: 'Love' },
	'👏': { id: 'clap', label: 'Applause' },
	'🎉': { id: 'party', label: 'Party' },
	'😴': { id: 'zzz', label: 'Snooze' }
};

/** Sticker for a wire emoji. Tolerates a missing/extra variation selector (U+FE0F); null if unknown. */
export function stickerFor(emoji: string): { id: StickerId; label: string } | null {
	const bare = emoji.replace(/️/g, '');
	const hit = REACTIONS.find((e) => e.replace(/️/g, '') === bare);
	return hit ? STICKERS[hit] : null;
}
