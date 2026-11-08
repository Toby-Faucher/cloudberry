import { describe, expect, it } from 'vitest';
import { addError, groupJobs, linkParts, playlistUrl } from './playlist';
import type { Job } from './types';

const job = (id: number, status: Job['status'], playlist: string | null = null, progress = 0): Job => ({
	id,
	url: `https://www.youtube.com/watch?v=vid${id}xxxxx`.slice(0, 43),
	status,
	progress,
	title: null,
	error: null,
	added_by: 'Ana',
	playlist: playlist ? { id: playlist, title: `Mix ${playlist}` } : null
});

describe('linkParts', () => {
	it('tells playlists, videos with a list, and plain videos apart', () => {
		expect(linkParts('https://www.youtube.com/playlist?list=PL123')).toEqual({ video: null, list: 'PL123', playlist: true });
		expect(linkParts('https://music.youtube.com/playlist?list=OLAK5uy_x&si=1')).toMatchObject({ playlist: true });
		expect(linkParts('https://www.youtube.com/watch?v=jNQXAC9IVzw&list=PL123&index=2')).toEqual({
			video: 'jNQXAC9IVzw',
			list: 'PL123',
			playlist: false
		});
		expect(linkParts('https://youtu.be/jNQXAC9IVzw')).toEqual({ video: 'jNQXAC9IVzw', list: null, playlist: false });
		expect(linkParts('https://www.youtube.com/playlist?list=bad;id').playlist).toBe(false);
		expect(linkParts('not a url').playlist).toBe(false);
		expect(playlistUrl('PL123')).toBe('https://www.youtube.com/playlist?list=PL123');
	});
});

describe('groupJobs', () => {
	it('folds a playlist into one row at its first job, counting progress', () => {
		const rows = groupJobs([
			job(1, 'downloading'),
			job(2, 'done', 'A'),
			job(3, 'downloading', 'A', 0.5),
			job(4, 'queued', 'A'),
			job(5, 'failed', 'A'),
			job(6, 'queued', 'B')
		]);
		expect(rows.map((r) => r.key)).toEqual(['j1', 'pA', 'pB']);
		const a = rows[1];
		expect(a.kind === 'playlist' && a).toMatchObject({
			title: 'Mix A',
			total: 4,
			done: 1,
			failed: 1,
			queued: 1,
			progress: (1 + 1 + 0.5) / 4
		});
		expect(a.kind === 'playlist' && a.current?.id).toBe(3);
	});
});

describe('addError', () => {
	it('rewords capacity errors, passes other add errors through, ignores the rest', () => {
		expect(addError('Queue is full')).toMatch(/queue is full/i);
		expect(addError('Too many downloads queued, try again soon')).toMatch(/Too many downloads/);
		expect(addError("Couldn't load that playlist: gone")).toBe("Couldn't load that playlist: gone");
		expect(addError("Can't poke that listener")).toBeNull();
	});
});
