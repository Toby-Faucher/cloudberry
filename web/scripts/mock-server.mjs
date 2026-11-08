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

const track = (id, title, secs, freqs) => ({
	id, title, duration_ms: secs * 1000, thumbnail: false, added_by: 'mock', added_at: Date.now(), freqs
});
const library = [
	track('aaaaaaaaaaa', 'Warm Hum in A', 40, [220, 277, 330]),
	track('bbbbbbbbbbb', 'Bright Fifths', 30, [330, 495, 660]),
	track('ccccccccccc', 'Low Drone (a rather long title to check truncation in the queue)', 35, [110, 165])
];
let nextQ = 1;
const qi = (t) => ({ queue_id: nextQ++, track: t, added_by: 'mock' });
const st = {
	current: qi(library[0]),
	queue: [qi(library[1]), qi(library[2])],
	playing: true,
	position_ms: 0,
	anchor_server_ms: now() + 1500,
	listeners: [],
	library,
	jobs: []
};
let nextListener = 1;
let endTimer;

const livePos = () => (st.playing ? st.position_ms + (now() - st.anchor_server_ms) : st.position_ms);
const wire = () => ({ ...st, library: st.library.map(({ freqs, ...t }) => t) });
const broadcast = (msg) => wss.clients.forEach((c) => c.send(JSON.stringify(msg)));
const pushState = () => {
	clearTimeout(endTimer);
	if (st.current && st.playing)
		endTimer = setTimeout(advance, st.current.track.duration_ms - livePos());
	broadcast({ type: 'state', state: wire() });
};
function advance() {
	st.current = st.queue.shift() ?? null;
	st.position_ms = 0;
	st.anchor_server_ms = now() + 1500;
	st.playing = !!st.current;
	pushState();
}

const server = http.createServer((req, res) => {
	const m = req.url.match(/^\/audio\/(\w+)\.mp3/);
	const t = m && library.find((t) => t.id === m[1]);
	if (!t) return res.writeHead(404).end();
	res.writeHead(200, { 'content-type': 'audio/wav' }).end(wav(t.duration_ms / 1000, t.freqs));
});
const wss = new WebSocketServer({ server, path: '/ws' });

wss.on('connection', (ws, req) => {
	const name = new URL(req.url, 'http://x').searchParams.get('name')?.trim() || 'anon';
	const id = nextListener++;
	st.listeners.push({ id, name });
	ws.send(JSON.stringify({ type: 'welcome', you: id }));
	pushState();
	ws.on('close', () => {
		st.listeners = st.listeners.filter((l) => l.id !== id);
		pushState();
	});
	ws.on('message', (raw) => {
		const msg = JSON.parse(raw);
		switch (msg.type) {
			case 'ping':
				return ws.send(JSON.stringify({ type: 'pong', t0: msg.t0, server_time: now() }));
			case 'play':
				if (!st.current) return advance();
				st.playing = true;
				st.anchor_server_ms = now() + 300;
				break;
			case 'pause':
				st.position_ms = livePos();
				st.playing = false;
				break;
			case 'seek':
				st.position_ms = msg.position_ms;
				st.anchor_server_ms = now() + 300;
				break;
			case 'skip':
				return advance();
			case 'enqueue':
				st.queue.push(qi(library.find((t) => t.id === msg.track_id)));
				break;
			case 'remove':
				st.queue = st.queue.filter((q) => q.queue_id !== msg.queue_id);
				break;
			case 'move': {
				const i = st.queue.findIndex((q) => q.queue_id === msg.queue_id);
				if (i >= 0) st.queue.splice(msg.to_index, 0, ...st.queue.splice(i, 1));
				break;
			}
			case 'add': {
				const job = { id: Date.now(), url: msg.url, status: 'downloading', progress: 0, title: null, error: null, added_by: name };
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

server.listen(8080, () => console.log('mock cloudberry on :8080'));
