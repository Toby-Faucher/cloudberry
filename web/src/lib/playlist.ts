// Playlist import helpers (pure, unit-tested): link parsing, job grouping, error copy.
import type { Job } from './types';

const LIST_ID = /^[A-Za-z0-9_-]{2,64}$/;

/**
 * What a YouTube link points at, loosely (the server re-validates):
 * `playlist?list=…` is a whole playlist; `watch?v=…&list=…` is one video that also
 * names a playlist (we offer "Add the whole playlist?" for it).
 */
export function linkParts(url: string): { video: string | null; list: string | null; playlist: boolean } {
	try {
		const u = new URL(url.trim());
		const list = u.searchParams.get('list');
		const okList = list && LIST_ID.test(list) ? list : null;
		const playlist = u.hostname !== 'youtu.be' && u.pathname.replace(/\/+$/, '') === '/playlist' && !!okList;
		const video = u.hostname === 'youtu.be' ? u.pathname.slice(1) || null : u.searchParams.get('v');
		return { video, list: okList, playlist };
	} catch {
		return { video: null, list: null, playlist: false };
	}
}

export const playlistUrl = (list: string) => `https://www.youtube.com/playlist?list=${list}`;

/** Video id of a job (its url is always the server's canonical watch?v= link). */
export const jobVideo = (j: Job) => linkParts(j.url).video;

export type JobRow =
	| { kind: 'job'; key: string; job: Job }
	| {
			kind: 'playlist';
			key: string;
			id: string;
			title: string;
			added_by: string;
			total: number;
			done: number;
			failed: number;
			/** Jobs not started yet (what cancel would drop). */
			queued: number;
			/** The one downloading now, if any. */
			current: Job | null;
			/** 0..1 across the whole playlist. */
			progress: number;
	  };

/** One row per single job; playlist jobs fold into one row per playlist, at the first job's spot. */
export function groupJobs(jobs: Job[]): JobRow[] {
	const rows: JobRow[] = [];
	const groups = new Map<string, Extract<JobRow, { kind: 'playlist' }>>();
	for (const job of jobs) {
		const p = job.playlist;
		if (!p) {
			rows.push({ kind: 'job', key: `j${job.id}`, job });
			continue;
		}
		let g = groups.get(p.id);
		if (!g) {
			g = { kind: 'playlist', key: `p${p.id}`, id: p.id, title: p.title, added_by: job.added_by, total: 0, done: 0, failed: 0, queued: 0, current: null, progress: 0 };
			groups.set(p.id, g);
			rows.push(g);
		}
		g.total++;
		if (job.status === 'done') g.done++;
		else if (job.status === 'failed') g.failed++;
		else if (job.status === 'queued') g.queued++;
		else g.current = job;
	}
	for (const g of groups.values()) g.progress = (g.done + g.failed + (g.current?.progress ?? 0)) / g.total;
	return rows;
}

/** Server error strings that are about adding, reworded where a bare string would puzzle. */
const ADD_ERRORS: Record<string, string> = {
	'Queue is full': 'The queue is full. Remove a few songs first.',
	'Too many downloads queued, try again soon': 'Too many downloads queued. Try again once a few have finished.'
};
const ADD_PREFIXES = ['Not a valid YouTube', "Couldn't load that playlist", 'Loading the playlist', 'Another playlist', 'Nothing in that playlist', 'could not run yt-dlp'];

/** The add box's copy for a server error, or null when the error isn't about adding. */
export function addError(message: string): string | null {
	if (message in ADD_ERRORS) return ADD_ERRORS[message];
	return ADD_PREFIXES.some((p) => message.startsWith(p)) ? message : null;
}
