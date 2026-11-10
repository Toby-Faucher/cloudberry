// Wire types, mirrored 1:1 from the spec (docs/superpowers/specs/2026-11-07-cloudberry-design.md).

export interface Track {
	id: string;
	title: string;
	duration_ms: number;
	thumbnail: boolean;
	added_by: string;
	added_at: number;
	/** Loudness normalisation, dB (clamped -12..12; 0 until measured). */
	gain_db: number;
}

export interface QueueItem {
	queue_id: number;
	track: Track;
	added_by: string;
}

export interface Listener {
	id: number;
	name: string;
	/** Tab hidden (sent via `presence`). */
	away: boolean;
}

export type JobStatus = 'queued' | 'downloading' | 'done' | 'failed';

export interface Job {
	id: number;
	url: string;
	status: JobStatus;
	progress: number;
	title: string | null;
	error: string | null;
	added_by: string;
	/** Set for jobs from a playlist import; clients group these into one row. */
	playlist: { id: string; title: string } | null;
}

export interface StationState {
	current: QueueItem | null;
	queue: QueueItem[];
	playing: boolean;
	position_ms: number;
	anchor_server_ms: number;
	listeners: Listener[];
	jobs: Job[];
	/** Last 20 played, newest first. */
	history: Track[];
	/** Whose added tracks got the most reactions today (server local day), or null. */
	dj_of_day: string | null;
}

/** A saved shared playlist (anyone can edit). The same track may appear more than once. */
export interface Playlist {
	id: number;
	name: string;
	created_by: string;
	created_at: number;
	updated_at: number;
	track_ids: string[];
	/** YouTube list id when it came from an import (re-importing updates it). */
	source_list_id: string | null;
}

export const REACTIONS = ['🔥', '😂', '❤️', '👏', '🎉', '😴'] as const;
export type Emoji = (typeof REACTIONS)[number];

export interface ChatMessage {
	id: number;
	kind: 'user' | 'event';
	name: string;
	text: string;
	/** ms since epoch */
	at: number;
}

export type ClientMsg =
	| { type: 'ping'; t0: number }
	| { type: 'add'; url: string }
	| { type: 'enqueue'; track_id: string }
	| { type: 'remove'; queue_id: number }
	| { type: 'move'; queue_id: number; to_index: number }
	| { type: 'play' }
	| { type: 'pause' }
	| { type: 'skip' }
	| { type: 'seek'; position_ms: number }
	| { type: 'chat'; text: string }
	| { type: 'react'; emoji: Emoji }
	| { type: 'insert'; track_id: string; index: number }
	| { type: 'poke'; listener_id: number }
	| { type: 'presence'; away: boolean }
	| { type: 'cancel_playlist'; playlist_id: string }
	| { type: 'playlist_create'; name: string; track_ids?: string[] }
	| { type: 'playlist_rename'; id: number; name: string }
	| { type: 'playlist_delete'; id: number }
	| { type: 'playlist_add'; id: number; track_ids: string[] }
	| { type: 'playlist_remove'; id: number; index: number }
	| { type: 'playlist_move'; id: number; from: number; to: number }
	| { type: 'playlist_queue'; id: number; shuffle: boolean };

export type ServerMsg =
	| { type: 'welcome'; you: number; version: string; build: string }
	| { type: 'pong'; t0: number; server_time: number }
	| { type: 'state'; state: StationState }
	/** The whole library, newest first: sent before the first state and whenever it changes. */
	| { type: 'library'; tracks: Track[] }
	/** Saved playlists, most recently updated first: after `library` on connect and after any change. */
	| { type: 'playlists'; playlists: Playlist[] }
	| { type: 'job'; job: Job }
	| { type: 'error'; message: string }
	| { type: 'chat_history'; messages: ChatMessage[] }
	| { type: 'chat'; message: ChatMessage }
	| { type: 'react'; listener_id: number; name: string; emoji: Emoji }
	| { type: 'poke'; from_id: number; from_name: string; to_id: number };
