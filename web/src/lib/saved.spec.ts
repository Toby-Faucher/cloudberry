import { describe, expect, it } from 'vitest';
import { dropMove, filterPlaylists, fmtTotal, moved, nameTaken, totalMs, validName } from './saved';
import type { Playlist, Track } from './types';

const pl = (id: number, name: string, track_ids: string[] = []): Playlist => ({
	id,
	name,
	created_by: 'Ana',
	created_at: 0,
	updated_at: 0,
	track_ids,
	source_list_id: null
});
const tr = (id: string, s: number) => ({ id, duration_ms: s * 1000 }) as Track;

describe('saved playlists', () => {
	it('validates names like the server', () => {
		expect(validName('  Mix ')).toBe(true);
		expect(validName('é'.repeat(60))).toBe(true);
		expect(validName('x'.repeat(61))).toBe(false);
		expect(validName('   ')).toBe(false);
	});

	it('warns about duplicate names, case-insensitively, except the playlist itself', () => {
		const lists = [pl(1, 'Friday'), pl(2, 'Focus')];
		expect(nameTaken(lists, ' friday ')).toBe(true);
		expect(nameTaken(lists, 'Friday', 1)).toBe(false);
		expect(nameTaken(lists, 'Fri')).toBe(false);
	});

	it('filters the picker by substring', () => {
		const lists = [pl(1, 'Friday Mix'), pl(2, 'Focus'), pl(3, 'mixtape')];
		expect(filterPlaylists(lists, 'MIX').map((p) => p.id)).toEqual([1, 3]);
		expect(filterPlaylists(lists, '  ')).toHaveLength(3);
		expect(filterPlaylists(lists, 'zzz')).toEqual([]);
	});

	it('totals durations, counting repeats and skipping unknown ids', () => {
		const lib = new Map([tr('a', 61), tr('b', 3599)].map((t) => [t.id, t]));
		expect(totalMs(['a', 'b', 'a', 'gone'], lib)).toBe((61 + 3599 + 61) * 1000);
		expect(fmtTotal(245_000)).toBe('4:05');
		expect(fmtTotal(3_723_000)).toBe('1:02:03');
		expect(fmtTotal(0)).toBe('0:00');
	});

	it('turns a finished drag into one move, and mirrors it locally', () => {
		expect(dropMove([0, 1, 2, 3], [1, 2, 0, 3], 0)).toEqual({ from: 0, to: 2 });
		expect(dropMove([0, 1, 2], [0, 1, 2], 1)).toBeNull();
		expect(dropMove([0, 1], [0, 1], 9)).toBeNull();
		expect(moved(['a', 'b', 'c', 'd'], 0, 2)).toEqual(['b', 'c', 'a', 'd']);
		expect(moved(['a', 'b', 'c'], 2, 0)).toEqual(['c', 'a', 'b']);
		expect(moved(['a', 'b', 'c'], 0, 99)).toEqual(['b', 'c', 'a']);
	});
});
