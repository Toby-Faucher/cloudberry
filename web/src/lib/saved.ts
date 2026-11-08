// Saved shared playlists: pure helpers (unit-tested in saved.spec.ts).
import type { Playlist, Track } from './types';
import { fmtTime } from './util';

export const NAME_MAX = 60;

/** Same rule as the server: 1–60 characters after trimming. */
export const validName = (name: string) => {
	const n = [...name.trim()].length;
	return n >= 1 && n <= NAME_MAX;
};

/** Another playlist already has this name (case-insensitive). Allowed, but we warn. */
export const nameTaken = (lists: Playlist[], name: string, exceptId?: number) => {
	const n = name.trim().toLowerCase();
	return lists.some((p) => p.id !== exceptId && p.name.toLowerCase() === n);
};

/** Picker filter: case-insensitive substring on the name; blank shows everything. */
export function filterPlaylists(lists: Playlist[], query: string): Playlist[] {
	const q = query.trim().toLowerCase();
	return q ? lists.filter((p) => p.name.toLowerCase().includes(q)) : lists;
}

/** Total running time of a playlist's tracks (ids missing from the library count as 0). */
export function totalMs(ids: string[], byId: Map<string, Track>): number {
	return ids.reduce((sum, id) => sum + (byId.get(id)?.duration_ms ?? 0), 0);
}

/** 4:05, 59:59, 1:02:03. */
export function fmtTotal(ms: number): string {
	const s = Math.max(0, Math.floor(ms / 1000));
	const h = Math.floor(s / 3600);
	if (!h) return fmtTime(ms);
	return `${h}:${String(Math.floor((s % 3600) / 60)).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;
}

/**
 * A drag finished: rows are keyed by their server index, so the dragged row's key in the old and
 * new orders gives the `playlist_move` to send. Null when nothing moved.
 */
export function dropMove(before: number[], after: number[], key: number): { from: number; to: number } | null {
	const from = before.indexOf(key);
	const to = after.indexOf(key);
	return from < 0 || to < 0 || from === to ? null : { from, to };
}

/** Local mirror of the server's move (optimistic until the next `playlists` arrives). */
export function moved<T>(list: T[], from: number, to: number): T[] {
	const next = [...list];
	next.splice(Math.min(to, next.length - 1), 0, ...next.splice(from, 1));
	return next;
}
