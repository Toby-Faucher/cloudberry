// Throwaway stand-in for the Rust server, for UI work without cargo:
//   node scripts/mock-server.mjs   (listens on :8080, which `npm run dev` proxies to)
// Speaks the spec's WS protocol; /audio/{id}.mp3 serves a generated WAV tone
// (decodeAudioData sniffs the format, so the extension doesn't matter).
import http from 'node:http';
import { WebSocketServer } from 'ws';

const now = () => performance.timeOrigin + performance.now();
const SR = 44100;

function wav(seconds, freqs) {
	const n = seconds * SR;
	const buf = Buffer.alloc(44 + n * 2);
	buf.write('RIFF', 0);
	buf.writeUInt32LE(36 + n * 2, 4);
	buf.write('WAVEfmt ', 8);
	buf.writeUInt32LE(16, 16);
	buf.writeUInt16LE(1, 20); // PCM
	buf.writeUInt16LE(1, 22); // mono
	buf.writeUInt32LE(SR, 24);
	buf.writeUInt32LE(SR * 2, 28);
	buf.writeUInt16LE(2, 32);
	buf.writeUInt16LE(16, 34);
	buf.write('data', 36);
	buf.writeUInt32LE(n * 2, 40);
	for (let i = 0; i < n; i++) {
		const t = i / SR;
		const beat = Math.exp(-((t * 2) % 1) * 6); // a pulse every 0.5 s makes sync audible
		let v = 0;
		for (const f of freqs) v += Math.sin(2 * Math.PI * f * t);
		buf.writeInt16LE(Math.round((v / freqs.length) * beat * 0.5 * 32767), 44 + i * 2);
	}
	return buf;
}

// `art` (two colours) -> served as an SVG gradient at /art/{id}.jpg, so the tint has something to sample.
const track = (id, title, secs, freqs, added_by = 'mock', art = null) => ({
	id, title, duration_ms: secs * 1000, thumbnail: !!art, added_by, added_at: Date.now(), gain_db: -3.5, freqs, art
});
const library = [
	track('aaaaaaaaaaa', 'Warm Hum in A (Official Video)', 40, [220, 277, 330], 'Ana', ['#2b6cb0', '#90cdf4']),
	track('bbbbbbbbbbb', 'Bright Fifths', 30, [330, 495, 660], 'Sam', ['#2f855a', '#c6f6d5']),
	track('ccccccccccc', 'Low Drone (a rather long title to check truncation in the queue)', 35, [110, 165], 'Priya')
];
// Saved shared playlists (same rules and error strings as the real server).
let nextPl = 4;
const pl = (id, name, created_by, track_ids, source_list_id = null) => ({
	id, name, created_by, created_at: Date.now() - id * 3.6e6, updated_at: Date.now() - id * 3.6e6, track_ids, source_list_id
});
let playlists = [
	pl(1, 'Friday 4pm', 'Ana', ['aaaaaaaaaaa', 'bbbbbbbbbbb', 'ccccccccccc', 'aaaaaaaaaaa']),
	pl(2, 'Heads down', 'Priya', ['ccccccccccc', 'bbbbbbbbbbb']),
	pl(3, 'Mock Mix', 'Sam', ['bbbbbbbbbbb'], 'PLmockmix')
];
const playlistsMsg = () => ({ type: 'playlists', playlists });
const touch = (p) => {
	p.updated_at = Date.now();
	playlists = [p, ...playlists.filter((x) => x.id !== p.id)];
	broadcast(playlistsMsg());
};
const plName = (n) => {
	const t = String(n ?? '').trim();
	return [...t].length >= 1 && [...t].length <= 60 ? t : null;
};
const MAX_QUEUE = 200;
let nextChat = 1;
const chatLog = [];
const t0 = Date.now();
for (const [kind, name, text, ago] of [
	['event', 'Ana', 'joined', 600],
	['event', 'Sam', 'joined', 598],
	['event', 'Priya', 'joined', 597],
	['user', 'Ana', 'morning all ☕', 590],
	['event', 'Sam', 'added Bright Fifths', 400],
	['user', 'Sam', 'this one is for the 10am slump', 395],
	['user', 'Sam', 'turn it up', 380],
	['event', 'Sam', 'left', 300], // a reconnect: hidden by the client
	['event', 'Sam', 'joined', 290],
	['event', 'Priya', 'skipped Some Song', 200],
	['user', 'Priya', 'sorry, not feeling that one 😅', 190]
])
	chatLog.push({ id: nextChat++, kind, name, text, at: t0 - ago * 1000 });
const chat = (kind, name, text) => {
	const message = { id: nextChat++, kind, name, text, at: Date.now() };
	chatLog.push(message);
	if (chatLog.length > 500) chatLog.shift();
	broadcast({ type: 'chat', message });
};
const EMOJI = ['🔥', '😂', '❤️', '👏', '🎉', '😴'];
let nextQ = 1;
const qi = (t, by = t.added_by) => ({ queue_id: nextQ++, track: t, added_by: by });
const st = {
	current: qi(library[0]),
	queue: [qi(library[1]), qi(library[2]), qi(library[0], 'Sam')],
	playing: true,
	position_ms: 0,
	anchor_server_ms: now() + 1500,
	listeners: [],
	library,
	jobs: [],
	history: [library[2], library[1]],
	// Seeded so the crown shows without reacting first.
	dj_of_day: process.env.MOCK_DJ ?? null
};
// DJ of the day: reactions count toward the current track's added_by (mock: in memory, no day roll).
const reactionsFor = new Map(); // name -> [count, last_at]
const lastPoke = new Map(); // listener id -> ms
let nextListener = 1;
let endTimer;

const livePos = () => (st.playing ? st.position_ms + (now() - st.anchor_server_ms) : st.position_ms);
const clean = ({ freqs, art, ...t }) => t;
const wire = () => ({
	...st,
	current: st.current && { ...st.current, track: clean(st.current.track) },
	queue: st.queue.map((q) => ({ ...q, track: clean(q.track) })),
	history: st.history.map(clean)
});
const libraryMsg = () => ({ type: 'library', tracks: st.library.map(clean) });
const broadcast = (msg) => wss.clients.forEach((c) => c.send(JSON.stringify(msg)));
const pushState = () => {
	clearTimeout(endTimer);
	if (st.current && st.playing)
		endTimer = setTimeout(advance, st.current.track.duration_ms - livePos());
	broadcast({ type: 'state', state: wire() });
};
function advance() {
	if (st.current) st.history = [st.current.track, ...st.history].slice(0, 20);
	st.current = st.queue.shift() ?? null;
	st.position_ms = 0;
	st.anchor_server_ms = now() + 1500;
	st.playing = !!st.current;
	pushState();
}

const server = http.createServer((req, res) => {
	const a = req.url.match(/^\/art\/(\w+)\.jpg/);
	const at = a && library.find((t) => t.id === a[1]);
	if (at?.art)
		return res.writeHead(200, { 'content-type': 'image/svg+xml' }).end(
			`<svg xmlns="http://www.w3.org/2000/svg" width="300" height="300"><defs><linearGradient id="g" x2="1" y2="1"><stop offset="0" stop-color="${at.art[0]}"/><stop offset="1" stop-color="${at.art[1]}"/></linearGradient></defs><rect width="300" height="300" fill="url(#g)"/><circle cx="210" cy="90" r="50" fill="#fff" opacity=".25"/></svg>`
		);
	const m = req.url.match(/^\/audio\/(\w+)\.mp3/);
	const t = m && library.find((t) => t.id === m[1]);
	if (!t) return res.writeHead(404).end();
	res.writeHead(200, { 'content-type': 'audio/wav' }).end(wav(t.duration_ms / 1000, t.freqs));
});
const wss = new WebSocketServer({ server, path: '/ws' });

wss.on('connection', (ws, req) => {
	const name = new URL(req.url, 'http://x').searchParams.get('name')?.trim() || 'anon';
	const id = nextListener++;
	st.listeners.push({ id, name, away: false });
	// Same order as the real server: welcome -> chat_history -> library -> state -> chat("joined").
	ws.send(JSON.stringify({ type: 'welcome', you: id, version: 'mock', build: 'mock' }));
	ws.send(JSON.stringify({ type: 'chat_history', messages: chatLog.slice(-100) }));
	ws.send(JSON.stringify(libraryMsg()));
	ws.send(JSON.stringify(playlistsMsg()));
	pushState();
	chat('event', name, 'joined');
	ws.on('close', () => {
		st.listeners = st.listeners.filter((l) => l.id !== id);
		chat('event', name, 'left');
		// Auto-pause when the last listener leaves (same as the real server).
		if (!st.listeners.length && st.playing) {
			st.position_ms = livePos();
			st.playing = false;
			chat('event', 'Cloudberry', 'paused');
		}
		pushState();
	});
	ws.on('message', (raw) => {
		const msg = JSON.parse(raw);
		switch (msg.type) {
			case 'ping':
				return ws.send(JSON.stringify({ type: 'pong', t0: msg.t0, server_time: now() }));
			case 'chat': {
				const text = String(msg.text ?? '').trim();
				if (!text || text.length > 500) return ws.send(JSON.stringify({ type: 'error', message: 'Message must be 1–500 characters' }));
				return chat('user', name, text);
			}
			case 'react':
				if (!EMOJI.includes(msg.emoji)) return ws.send(JSON.stringify({ type: 'error', message: 'Unknown reaction' }));
				broadcast({ type: 'react', listener_id: id, name, emoji: msg.emoji });
				if (st.current) {
					const by = st.current.added_by;
					const [n = 0] = reactionsFor.get(by) ?? [];
					reactionsFor.set(by, [n + 1, Date.now()]);
					// Most reactions wins; ties go to the most recent reaction.
					const top = [...reactionsFor].sort((a, b) => b[1][0] - a[1][0] || b[1][1] - a[1][1])[0][0];
					if (top !== st.dj_of_day) {
						st.dj_of_day = top;
						pushState();
					}
				}
				return;
			case 'poke': {
				const known = st.listeners.some((l) => l.id === msg.listener_id);
				if (msg.listener_id === id || !known) return ws.send(JSON.stringify({ type: 'error', message: "Can't poke that listener" }));
				if (Date.now() - (lastPoke.get(id) ?? 0) < 2000) return;
				lastPoke.set(id, Date.now());
				return broadcast({ type: 'poke', from_id: id, from_name: name, to_id: msg.listener_id });
			}
			case 'presence': {
				const l = st.listeners.find((l) => l.id === id);
				if (l) l.away = !!msg.away;
				break;
			}
			case 'insert': {
				const t = library.find((t) => t.id === msg.track_id);
				if (!t) return ws.send(JSON.stringify({ type: 'error', message: 'Unknown track' }));
				st.queue.splice(Math.min(msg.index, st.queue.length), 0, qi(t, name));
				if (!st.current) return advance();
				break;
			}
			case 'play':
				if (!st.current) return advance();
				if (!st.playing) chat('event', name, 'resumed');
				st.playing = true;
				st.anchor_server_ms = now() + 300;
				break;
			case 'pause':
				st.position_ms = livePos();
				if (st.playing) chat('event', name, 'paused');
				st.playing = false;
				break;
			case 'seek':
				st.position_ms = msg.position_ms;
				st.anchor_server_ms = now() + 300;
				break;
			case 'skip':
				if (st.current) chat('event', name, `skipped ${st.current.track.title}`);
				return advance();
			case 'enqueue': {
				const t = library.find((t) => t.id === msg.track_id);
				if (!t) return;
				st.queue.push(qi(t, name));
				chat('event', name, `added ${t.title}`);
				break;
			}
			case 'remove': {
				const q = st.queue.find((q) => q.queue_id === msg.queue_id);
				st.queue = st.queue.filter((q) => q.queue_id !== msg.queue_id);
				if (q) chat('event', name, `removed ${q.track.title}`);
				break;
			}
			case 'move': {
				const i = st.queue.findIndex((q) => q.queue_id === msg.queue_id);
				if (i >= 0) st.queue.splice(msg.to_index, 0, ...st.queue.splice(i, 1));
				break;
			}
			case 'playlist_create': case 'playlist_rename': case 'playlist_delete': case 'playlist_add':
			case 'playlist_remove': case 'playlist_move': case 'playlist_queue': {
				const err = (message) => ws.send(JSON.stringify({ type: 'error', message }));
				const known = (ids) => (ids ?? []).filter((id) => library.some((t) => t.id === id));
				if (msg.type === 'playlist_create') {
					const n = plName(msg.name);
					if (!n) return err('Playlist name must be 1–60 characters');
					if (playlists.length >= 100) return err('Too many playlists');
					const p = pl(nextPl++, n, name, known(msg.track_ids));
					p.created_at = Date.now();
					touch(p);
					return chat('event', name, `created playlist ${n}`);
				}
				const p = playlists.find((x) => x.id === msg.id);
				if (!p) return err('Unknown playlist');
				const ids = p.track_ids;
				switch (msg.type) {
					case 'playlist_rename': {
						const n = plName(msg.name);
						if (!n) return err('Playlist name must be 1–60 characters');
						const old = p.name;
						p.name = n;
						touch(p);
						return chat('event', name, `renamed playlist ${old} to ${n}`);
					}
					case 'playlist_delete':
						playlists = playlists.filter((x) => x !== p);
						broadcast(playlistsMsg());
						return chat('event', name, `deleted playlist ${p.name}`);
					case 'playlist_add': {
						const add = known(msg.track_ids);
						if (!add.length) return err('Unknown track');
						if (ids.length + add.length > 500) return err('Playlist is full');
						ids.push(...add);
						return touch(p);
					}
					case 'playlist_remove':
						if (!(msg.index < ids.length)) return err('No such song in that playlist');
						ids.splice(msg.index, 1);
						return touch(p);
					case 'playlist_move':
						if (!(msg.from < ids.length)) return err('No such song in that playlist');
						ids.splice(Math.min(msg.to, ids.length - 1), 0, ...ids.splice(msg.from, 1));
						return touch(p);
					case 'playlist_queue': {
						if (!ids.length) return err('That playlist is empty');
						const order = [...ids];
						if (msg.shuffle) order.sort(() => Math.random() - 0.5);
						const room = MAX_QUEUE - st.queue.length;
						const take = order.slice(0, Math.max(0, room));
						if (!take.length) return err('Queue is full');
						for (const id of take) st.queue.push(qi(library.find((t) => t.id === id), name));
						const n = take.length;
						chat('event', name, `${msg.shuffle ? 'shuffled' : 'queued'} playlist ${p.name} (${n} song${n === 1 ? '' : 's'})`);
						if (n < order.length) err(`Queued ${n} of ${order.length}, queue is full`);
						if (!st.current) return advance();
						break;
					}
				}
				break;
			}
			case 'cancel_playlist': {
				const before = jobs.length;
				jobs = jobs.filter((j) => !(j.status === 'queued' && j.playlist?.id === msg.playlist_id));
				if (jobs.length === before) return ws.send(JSON.stringify({ type: 'error', message: 'Nothing left to cancel' }));
				st.jobs = jobs;
				chat('event', name, 'cancelled playlist Mock Mix');
				break;
			}
			case 'add': {
				// Magic links for UI work: ...full -> "Queue is full", ...busy -> too many downloads.
				if (msg.url.includes('full')) return ws.send(JSON.stringify({ type: 'error', message: 'Queue is full' }));
				if (msg.url.includes('busy'))
					return ws.send(JSON.stringify({ type: 'error', message: 'Too many downloads queued, try again soon' }));
				const u = new URL(msg.url);
				if (u.pathname === '/playlist') return importPlaylist(u.searchParams.get('list'), name);
				const v = u.searchParams.get('v') ?? u.pathname.slice(1);
				const job = { id: Date.now(), url: `https://www.youtube.com/watch?v=${v}`, status: 'downloading', progress: 0, title: null, error: null, added_by: name, playlist: null };
				const tick = setInterval(() => {
					job.progress = Math.min(1, job.progress + 0.2);
					job.title = 'Fake download';
					if (job.progress >= 1) {
						clearInterval(tick);
						job.status = msg.url.includes('fail') ? 'failed' : 'done';
						job.error = job.status === 'failed' ? 'Video unavailable' : null;
					}
					broadcast({ type: 'job', job });
				}, 500);
				return;
			}
		}
		pushState();
	});
});

// Playlist import: 7 fake songs (2 already "cached"), downloaded one at a time, slowly.
let jobs = st.jobs;
function importPlaylist(list, name) {
	const playlist = { id: list, title: 'Mock Mix' };
	setTimeout(() => {
		for (const i of [1, 2]) st.queue.push(qi(library[i], name));
		const fresh = Array.from({ length: 5 }, (_, i) => ({
			id: Date.now() + i, url: `https://www.youtube.com/watch?v=mockmix000${i}`, status: 'queued', progress: 0,
			title: `Mix song ${i + 1}`, error: null, added_by: name, playlist
		}));
		jobs = st.jobs = [...jobs, ...fresh];
		chat('event', name, 'added playlist Mock Mix (7 songs, 0 skipped)');
		pushState();
		const next = () => {
			const j = jobs.find((j) => j.playlist?.id === list && j.status === 'queued');
			if (!j) {
				setTimeout(() => ((jobs = st.jobs = jobs.filter((x) => x.playlist?.id !== list)), pushState()), 8000);
				return;
			}
			j.status = 'downloading';
			const tick = setInterval(() => {
				j.progress = Math.min(1, j.progress + 0.1);
				if (j.progress >= 1) {
					clearInterval(tick);
					j.status = 'done';
					next();
				}
				broadcast({ type: 'job', job: j });
			}, Number(process.env.MOCK_DL_MS) || 800);
		};
		next();
	}, 600);
}

const port = Number(process.env.PORT) || 8080;
server.listen(port, () => console.log(`mock cloudberry on :${port}`));
