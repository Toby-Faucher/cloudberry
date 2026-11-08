/**
 * WebSocket connection to the station: reactive state, ping cadence for the
 * clock, auto-reconnect with exponential backoff. Feeds the Player.
 */
import { Clock, Player, expectedPositionMs, localNow } from './sync';
import { ui } from './ui.svelte';
import type { ChatMessage, ClientMsg, Emoji, Job, QueueItem, ServerMsg, StationState } from './types';

export type ConnStatus = 'connecting' | 'live' | 'reconnecting';

export interface Toast {
	id: number;
	text: string;
	undo?: () => void;
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

const PING_BURST = 8;
const PING_BURST_GAP_MS = 120;
const PING_EVERY_MS = 2000;

export class Station {
	state = $state<StationState | null>(null);
	jobs = $state<Job[]>([]);
	you = $state<number | null>(null);
	status = $state<ConnStatus>('connecting');
	toast = $state<Toast | null>(null);
	/** Newest last; capped. */
	chat = $state<ChatMessage[]>([]);
	/** Reactions currently floating over the art; each removes itself after REACTION_MS. */
	reactions = $state<Reaction[]>([]);

	readonly clock = new Clock();
	readonly player: Player;

	private ws: WebSocket | null = null;
	private attempt = 0;
	private timers: ReturnType<typeof setTimeout>[] = [];
	private closed = false;

	constructor(readonly name: string) {
		this.player = new Player(this.clock, (m) => this.showToast(m));
		this.connect();
	}

	send(msg: ClientMsg) {
		if (this.ws?.readyState === WebSocket.OPEN) this.ws.send(JSON.stringify(msg));
		else this.showToast('Not connected — reconnecting…');
	}

	close() {
		this.closed = true;
		this.clearTimers();
		this.ws?.close();
		this.player.destroy();
	}

	/** One toast at a time (calm); a new one replaces the old. Undo toasts linger longer. */
	showToast(text: string, undo?: () => void) {
		const id = ++toastSeq;
		this.toast = { id, text, undo };
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
		return true;
	}

	react(emoji: Emoji) {
		this.send({ type: 'react', emoji });
		// Show ours immediately; the server's echo (if any) is ignored below.
		this.float(emoji, this.name, true);
	}

	private float(emoji: Emoji, name: string, mine: boolean) {
		const r = { key: ++reactionSeq, emoji, name, mine, x: Math.random(), drift: Math.random() * 2 - 1 };
		this.reactions = [...this.reactions.slice(-(MAX_FLOATING - 1)), r];
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
				this.clock.add(msg.t0, msg.server_time);
				break;
			case 'state':
				this.state = msg.state;
				this.jobs = msg.state.jobs;
				this.player.update(msg.state);
				break;
			case 'job': {
				const i = this.jobs.findIndex((j) => j.id === msg.job.id);
				if (i >= 0) this.jobs[i] = msg.job;
				else this.jobs.push(msg.job);
				if (msg.job.status === 'done') {
					// Let the checkmark show for a moment before it leaves.
					const id = msg.job.id;
					setTimeout(() => (this.jobs = this.jobs.filter((j) => j.id !== id)), 2500);
				}
				break;
			}
			case 'error':
				this.showToast(msg.message);
				break;
			case 'chat_history':
				// Sent on every (re)connect: replace, don't append. What was said before we
				// first joined isn't "unread".
				if (!this.chat.length) ui.seenId = Math.max(ui.seenId, msg.messages.at(-1)?.id ?? 0);
				this.chat = msg.messages.slice(-CHAT_KEEP);
				break;
			case 'chat':
				if (this.chat.some((m) => m.id === msg.message.id)) break;
				this.chat = [...this.chat.slice(-(CHAT_KEEP - 1)), msg.message];
				break;
			case 'react':
				if (msg.listener_id !== this.you) this.float(msg.emoji, msg.name, false);
				break;
		}
	}
}
