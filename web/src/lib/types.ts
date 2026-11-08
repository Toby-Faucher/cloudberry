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
}

export interface StationState {
	current: QueueItem | null;
	queue: QueueItem[];
	playing: boolean;
	position_ms: number;
	anchor_server_ms: number;
	listeners: Listener[];
	library: Track[];
	jobs: Job[];
	/** Last 20 played, newest first. */
	history: Track[];
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
	| { type: 'insert'; track_id: string; index: number };

export type ServerMsg =
	| { type: 'welcome'; you: number }
	| { type: 'pong'; t0: number; server_time: number }
	| { type: 'state'; state: StationState }
	| { type: 'job'; job: Job }
	| { type: 'error'; message: string }
	| { type: 'chat_history'; messages: ChatMessage[] }
	| { type: 'chat'; message: ChatMessage }
	| { type: 'react'; listener_id: number; name: string; emoji: Emoji };
