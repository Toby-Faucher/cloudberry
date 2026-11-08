/**
 * WebSocket connection to the station: reactive state, ping cadence for the
 * clock, auto-reconnect with exponential backoff. Feeds the Player.
 */
import { Clock, Player, expectedPositionMs, localNow } from './sync';
import { ui } from './ui.svelte';
import { Combo, COMBO_WINDOW_MS } from './game';
import { addError, linkParts } from './playlist';
import { matchPending, PENDING_MS, type PendingChat } from './chat';
import type { ChatMessage, ClientMsg, Emoji, Job, QueueItem, ServerMsg, StationState, Track } from './types';

export type ConnStatus = 'connecting' | 'live' | 'reconnecting';

export interface Toast {
	id: number;
	text: string;
	undo?: () => void;
	/** Styling/motion hint: a poke toast wiggles in. */
	kind?: 'poke';
}

/** Bumped on every event so components can replay a one-shot animation for listener `id`. */
export interface Ping {
	id: number;
	seq: number;
}

export interface Reaction {
	key: number;
	emoji: Emoji;
	name: string;
	mine: boolean;
	/** 0..1 horizontal start, -1..1 sideways drift (randomised once). */
	x: number;
	drift: number;
}

export const CHAT_MAX = 500;
const CHAT_KEEP = 200;
const UNDO_MS = 6000;
export const REACTION_MS = 2600;
const MAX_FLOATING = 24;
let toastSeq = 0;
let reactionSeq = 0;
let pendingSeq = 0;

const PING_BURST = 8;
const PING_BURST_GAP_MS = 120;
const PING_EVERY_MS = 2000;
/** Server errors this soon after our `add` belong to the add box (playlist expansion takes up to 60 s). */
const ADD_ERROR_WINDOW_MS = 65_000;
/** No pong this long after a wake-up ping: the socket is dead even if it still says OPEN. */
const WAKE_PONG_MS = 4000;

export class Station {
	state = $state<StationState | null>(null);
	jobs = $state<Job[]>([]);
	/** Every track ever added, newest first (its own `library` message, not part of state). */
	library = $state<Track[]>([]);
	/** Add-box feedback: a server error about our last add, or a "loading playlist" note. */
	addNote = $state<{ text: string; error: boolean } | null>(null);
	/** video id -> playlist id, for videos we added from a `watch?v=…&list=…` link. */
	listHints = $state<Record<string, string>>({});
	private lastAddAt = -Infinity;
	private lastPongAt = 0;
	you = $state<number | null>(null);
	status = $state<ConnStatus>('connecting');
	toast = $state<Toast | null>(null);
	/** Newest last; capped. */
	chat = $state<ChatMessage[]>([]);
	/** Our sent messages the server hasn't echoed yet (oldest first). */
	pendingChat = $state<PendingChat[]>([]);
	/** Echoed message id -> the pending key it settled: the chat keeps the same bubble (no re-entrance). */
	readonly settled = new Map<number, number>();
	/** Reactions currently floating over the art; each removes itself after REACTION_MS. */
	reactions = $state<Reaction[]>([]);
	/** Live reaction combo (3+ of the same emoji within 2 s), null when none. */
	combo = $state<{ emoji: Emoji; count: number } | null>(null);
	/** Last poke target (everyone wiggles that avatar) and last reactor (avatar bounces). */
	poked = $state<Ping | null>(null);
	reacted = $state<Ping | null>(null);
	private combos = new Combo();
	private comboTimer: ReturnType<typeof setTimeout> | undefined;
	private pingSeq = 0;

	readonly clock = new Clock();
	readonly player: Player;

	private ws: WebSocket | null = null;
	private attempt = 0;
	private timers: ReturnType<typeof setTimeout>[] = [];
	private closed = false;

	constructor(readonly name: string) {
		this.player = new Player(this.clock, (m) => this.showToast(m));
		this.connect();
		document.addEventListener('visibilitychange', this.visibility);
	}

	/** Away = tab hidden; everyone sees the avatar fade. */
	private presence = () => {
		if (this.ws?.readyState === WebSocket.OPEN)
			this.ws.send(JSON.stringify({ type: 'presence', away: document.hidden } satisfies ClientMsg));
	};

	/**
	 * Background tabs get their timers throttled (Chrome: ~1/min after 5 min hidden), which the
	 * server's 120 s idle timeout tolerates. Coming back: ping now, and reconnect straight away if
	 * the socket died meanwhile (closed, or open but no pong back).
	 */
	private visibility = () => {
		this.presence();
		if (document.hidden || this.closed) return;
		const ws = this.ws;
		if (!ws || ws.readyState === WebSocket.CLOSING || ws.readyState === WebSocket.CLOSED) return this.reconnectNow();
		if (ws.readyState !== WebSocket.OPEN) return; // still connecting
		const sent = performance.now();
		this.ping();
		setTimeout(() => {
			if (this.ws === ws && this.lastPongAt < sent) this.reconnectNow();
		}, WAKE_PONG_MS);
	};

	private reconnectNow() {
		const old = this.ws;
		this.ws = null; // its onclose must not schedule a second reconnect
		old?.close();
		this.clearTimers();
		this.attempt = 0;
		this.status = 'reconnecting';
		this.connect();
	}

	/** Add a link (video or playlist). Server errors about it land in `addNote`. */
	add(url: string) {
		const v = url.trim();
		const { video, list, playlist } = linkParts(v);
		if (!playlist && video && list) this.listHints[video] = list;
		this.lastAddAt = performance.now();
		const note = playlist ? { text: 'Loading the playlist…', error: false } : null;
		this.addNote = note;
		if (note) setTimeout(() => this.addNote?.error === false && (this.addNote = null), ADD_ERROR_WINDOW_MS);
		this.send({ type: 'add', url: v });
	}

	/** Poke someone else (server rate-limits to 1 per 2 s and rejects self). */
	poke(id: number) {
		if (id === this.you) return;
		this.send({ type: 'poke', listener_id: id });
	}

	send(msg: ClientMsg) {
		if (this.ws?.readyState === WebSocket.OPEN) this.ws.send(JSON.stringify(msg));
		else this.showToast('Not connected — reconnecting…');
	}

	close() {
		this.closed = true;
		document.removeEventListener('visibilitychange', this.visibility);
		clearTimeout(this.comboTimer);
		this.clearTimers();
		this.ws?.close();
		this.player.destroy();
	}

	/** One toast at a time (calm); a new one replaces the old. Undo toasts linger longer. */
	showToast(text: string, undo?: () => void, kind?: Toast['kind']) {
		const id = ++toastSeq;
		this.toast = { id, text, undo, kind };
		setTimeout(() => this.dismissToast(id), undo ? UNDO_MS : 4000);
	}

	dismissToast(id: number) {
		if (this.toast?.id === id) this.toast = null;
	}

	/** Live position on the shared timeline, ms (0 when nothing is current). */
	positionMs(): number {
		const st = this.state;
		return st?.current ? Math.max(0, expectedPositionMs(st, this.clock.serverNow())) : 0;
	}

	togglePlay() {
		const s = this.state;
		if (s?.current || s?.queue.length) this.send({ type: s.playing ? 'pause' : 'play' });
	}

	/** Skip for everyone; Undo re-inserts the skipped track at the head (plays next, from the start). */
	skip() {
		const cur = this.state?.current;
		if (!cur) return;
		this.send({ type: 'skip' });
		this.showToast(`Skipped “${cur.track.title}”`, () =>
			this.send({ type: 'insert', track_id: cur.track.id, index: 0 })
		);
	}

	/** Remove from the queue; Undo re-inserts it where it was. */
	remove(item: QueueItem) {
		const index = this.state?.queue.findIndex((q) => q.queue_id === item.queue_id) ?? -1;
		if (index < 0) return;
		this.send({ type: 'remove', queue_id: item.queue_id });
		this.showToast(`Removed “${item.track.title}”`, () =>
			this.send({ type: 'insert', track_id: item.track.id, index })
		);
	}

	sendChat(text: string): boolean {
		const t = text.trim();
		if (!t || t.length > CHAT_MAX) return false;
		this.send({ type: 'chat', text: t });
		// Shown at once as pending; the server's echo settles it (matched by name + text).
		const key = ++pendingSeq;
		this.pendingChat = [...this.pendingChat, { key, text: t, failed: false }];
		setTimeout(() => {
			const p = this.pendingChat.find((x) => x.key === key);
			if (p) p.failed = true;
		}, PENDING_MS);
		return true;
	}

	/** A pending message never came back: drop it and send it again. */
	retryChat(key: number) {
		const p = this.pendingChat.find((x) => x.key === key);
		if (!p) return;
		this.pendingChat = this.pendingChat.filter((x) => x.key !== key);
		this.sendChat(p.text);
	}

	react(emoji: Emoji) {
		this.send({ type: 'react', emoji });
		// Show ours immediately; the server's echo (if any) is ignored below.
		this.float(emoji, this.name, true);
		if (this.you !== null) this.reacted = { id: this.you, seq: ++this.pingSeq };
	}

	private float(emoji: Emoji, name: string, mine: boolean) {
		const r = { key: ++reactionSeq, emoji, name, mine, x: Math.random(), drift: Math.random() * 2 - 1 };
		this.reactions = [...this.reactions.slice(-(MAX_FLOATING - 1)), r];
		const now = performance.now();
		this.combos.add(emoji, now);
		this.combo = this.combos.shown(now) as Station['combo'];
		clearTimeout(this.comboTimer);
		this.comboTimer = setTimeout(() => (this.combo = null), COMBO_WINDOW_MS);
		setTimeout(() => (this.reactions = this.reactions.filter((x) => x.key !== r.key)), REACTION_MS);
	}

	private clearTimers() {
		this.timers.forEach((t) => clearTimeout(t));
		this.timers = [];
	}

	private ping = () => {
		if (this.ws?.readyState === WebSocket.OPEN)
			this.ws.send(JSON.stringify({ type: 'ping', t0: localNow() } satisfies ClientMsg));
	};

	private connect() {
		const proto = location.protocol === 'https:' ? 'wss' : 'ws';
		const ws = new WebSocket(`${proto}://${location.host}/ws?name=${encodeURIComponent(this.name)}`);
		this.ws = ws;

		ws.onopen = () => {
			this.attempt = 0;
			this.status = 'live';
			if (document.hidden) this.presence();
			// Burst for a quick initial estimate, then keep refining.
			for (let i = 0; i < PING_BURST; i++) this.timers.push(setTimeout(this.ping, i * PING_BURST_GAP_MS));
			this.timers.push(setInterval(this.ping, PING_EVERY_MS));
		};

		ws.onmessage = (ev) => {
			let msg: ServerMsg;
			try {
				msg = JSON.parse(ev.data);
			} catch {
				return;
			}
			this.handle(msg);
		};

		ws.onclose = () => {
			this.clearTimers();
			if (this.ws !== ws || this.closed) return;
			this.status = 'reconnecting';
			// 0.5 s, 1 s, 2 s … capped at 15 s, with jitter so tabs don't stampede.
			const delay = Math.min(15_000, 500 * 2 ** this.attempt++) * (0.75 + Math.random() / 2);
			this.timers.push(setTimeout(() => this.connect(), delay));
		};
	}

	private handle(msg: ServerMsg) {
		switch (msg.type) {
			case 'welcome':
				this.you = msg.you;
				break;
			case 'pong':
				this.lastPongAt = performance.now();
				this.clock.add(msg.t0, msg.server_time);
				break;
			case 'library':
				this.library = msg.tracks;
				break;
			case 'state':
				this.state = msg.state;
				ui.dj = msg.state.dj_of_day ?? null;
				this.jobs = msg.state.jobs;
				if (!this.addNote?.error && msg.state.jobs.some((j) => j.playlist && j.added_by === this.name)) this.addNote = null;
				this.player.update(msg.state);
				break;
			case 'job': {
				const i = this.jobs.findIndex((j) => j.id === msg.job.id);
				if (i >= 0) this.jobs[i] = msg.job;
				else this.jobs.push(msg.job);
				// Playlist rows stay until the server clears the whole group (they drive "7/25").
				if (msg.job.status === 'done' && !msg.job.playlist) {
					// Let the checkmark show for a moment before it leaves.
					const id = msg.job.id;
					setTimeout(() => (this.jobs = this.jobs.filter((j) => j.id !== id)), 2500);
				}
				break;
			}
			case 'error': {
				const add = performance.now() - this.lastAddAt < ADD_ERROR_WINDOW_MS ? addError(msg.message) : null;
				if (add) this.addNote = { text: add, error: true };
				else this.showToast(msg.message);
				break;
			}
			case 'chat_history':
				// Sent on every (re)connect: replace, don't append. What was said before we
				// first joined isn't "unread".
				if (!this.chat.length) ui.seenId = Math.max(ui.seenId, msg.messages.at(-1)?.id ?? 0);
				this.chat = msg.messages.slice(-CHAT_KEEP);
				break;
			case 'chat': {
				if (this.chat.some((m) => m.id === msg.message.id)) break;
				const mine = matchPending(this.pendingChat, msg.message, this.name);
				if (mine >= 0) {
					this.settled.set(msg.message.id, this.pendingChat[mine].key);
					this.pendingChat = this.pendingChat.toSpliced(mine, 1);
				}
				// Our playlist landed (possibly all from cache, so no job rows): drop "Loading…".
				if (msg.message.name === this.name && msg.message.text.startsWith('added playlist') && !this.addNote?.error)
					this.addNote = null;
				this.chat = [...this.chat.slice(-(CHAT_KEEP - 1)), msg.message];
				break;
			}
			case 'react':
				if (msg.listener_id !== this.you) {
					this.float(msg.emoji, msg.name, false);
					this.reacted = { id: msg.listener_id, seq: ++this.pingSeq };
				}
				break;
			case 'poke':
				this.poked = { id: msg.to_id, seq: ++this.pingSeq };
				if (msg.to_id === this.you) this.showToast(`${msg.from_name} poked you`, undefined, 'poke');
				else if (msg.from_id === this.you) {
					const to = this.state?.listeners.find((l) => l.id === msg.to_id)?.name;
					if (to) this.showToast(`You poked ${to}`);
				}
				break;
		}
	}
}
