/**
 * WebSocket connection to the station: reactive state, ping cadence for the
 * clock, auto-reconnect with exponential backoff. Feeds the Player.
 */
import { Clock, Player, localNow } from './sync';
import type { ClientMsg, Job, ServerMsg, StationState } from './types';

export type ConnStatus = 'connecting' | 'live' | 'reconnecting';

const PING_BURST = 8;
const PING_BURST_GAP_MS = 120;
const PING_EVERY_MS = 2000;

export class Station {
	state = $state<StationState | null>(null);
	jobs = $state<Job[]>([]);
	you = $state<number | null>(null);
	status = $state<ConnStatus>('connecting');
	toast = $state<{ id: number; text: string } | null>(null);

	readonly clock = new Clock();
	readonly player: Player;

	private ws: WebSocket | null = null;
	private attempt = 0;
	private timers: ReturnType<typeof setTimeout>[] = [];
	private closed = false;

	constructor(private name: string) {
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

	showToast(text: string) {
		const id = Date.now();
		this.toast = { id, text };
		setTimeout(() => this.toast?.id === id && (this.toast = null), 4000);
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
		}
	}
}
