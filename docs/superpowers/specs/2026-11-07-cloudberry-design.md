# Cloudberry: design (MVP)

A tiny self-hosted shared radio for Toby and 3–4 coworkers. Open the site, pick a
name, press **Tune in**, and hear exactly what everyone else hears. Anyone can add,
skip, reorder, pause. Music comes from pasted YouTube links, downloaded to the server.

Hosted on Toby's home server (Dockge, 10.0.0.163) as one Docker container. Private
network only.

## Goals
- Everyone hears the same audio at the same moment (target: within ~30 ms).
- Very low server RAM (target: < 40 MB RSS idle). Single Rust binary.
- Clean, calm UI with a warm orange accent and plenty of motion; WASM-powered visualizer.
- Paste a YouTube link → MP3 stored in `CLOUDBERRY_MUSIC_DIR` → playable.

## Non-goals (MVP)
- Spotify (DRM; later at most "match by name then fetch from YouTube").
- Accounts/passwords (private network; a display name is enough).
- Multiple rooms. There is exactly one station.
- Mobile apps.

## Anti-goals
- Listeners drifting audibly apart.
- Server streaming/transcoding audio live (costs RAM/CPU; clients fetch files instead).

## Repo layout
```
Cargo.toml              # workspace, members = ["crates/*"]
crates/server/          # axum binary "cloudberry"
crates/visualizer/      # cdylib -> wasm32-unknown-unknown, no wasm-bindgen
web/                    # SvelteKit (adapter-static, SPA fallback) + Bits UI
Dockerfile, compose.yaml, README.md
```

## Configuration (env vars, all read by the server)
| Var | Default | Meaning |
|---|---|---|
| `CLOUDBERRY_MUSIC_DIR` | `./data/music` | where `{id}.mp3` and `{id}.jpg` are stored |
| `CLOUDBERRY_DB_PATH` | `./data/cloudberry.db` | SQLite file |
| `CLOUDBERRY_BIND` | `0.0.0.0:8080` | listen address |
| `CLOUDBERRY_STATIC_DIR` | `./web/build` | built SvelteKit site |
| `CLOUDBERRY_YTDLP` | `yt-dlp` | yt-dlp executable |

## How sync works
The server never streams audio. It holds the **station state** and a clock.

- Each track: `GET /audio/{id}.mp3` (static file, HTTP range support).
- State anchor: `position_ms` was the playback position at server time `anchor_server_ms`
  (ms since Unix epoch, f64). If `playing`, the live position is
  `position_ms + (server_now - anchor_server_ms)`.
- Clients estimate `server_now` via WebSocket ping/pong (NTP-style: keep the sample with
  the lowest RTT out of the last ~10; offset = `server_time - (t0 + rtt/2)`).
- Client fetches + `decodeAudioData`s the MP3, then starts an `AudioBufferSourceNode` with
  `start(ctxWhen, offsetSec)` so it lands exactly on the shared timeline, compensating
  `ctx.outputLatency`/`baseLatency`. Every ~2 s it measures drift; if > 25 ms it restarts
  the source at the corrected offset.
- The server owns track advancement: a tokio timer fires when the current track ends,
  pops the next queue item, sets `anchor_server_ms = now + 1500` (lead time so all clients
  have decoded it; clients prefetch the next track), `position_ms = 0`, broadcasts state.
- Pause: server freezes `position_ms` at the live position, `playing=false`. Play: new
  anchor `now + 300`. Seek: same, at the requested position.

## HTTP API
- `GET /api/state` → `StationState` (same as WS `state` payload)
- `GET /audio/{id}.mp3`, `GET /art/{id}.jpg` → files from `CLOUDBERRY_MUSIC_DIR`
  (`Cache-Control: public, max-age=31536000, immutable`)
- `GET /ws?name=<display name>` → WebSocket (name trimmed, 1–32 chars, else 400)
- Everything else → static site, SPA fallback to `index.html`.

## WebSocket protocol (JSON text frames, tagged by `type`)
Client → server:
```
{ "type": "ping", "t0": <client ms, f64> }
{ "type": "add", "url": "<youtube url>" }          // download, then append to queue
{ "type": "enqueue", "track_id": "<id>" }          // append an existing library track
{ "type": "remove", "queue_id": <u64> }
{ "type": "move", "queue_id": <u64>, "to_index": <usize> }
{ "type": "play" } | { "type": "pause" } | { "type": "skip" }
{ "type": "seek", "position_ms": <f64> }
```
Server → client:
```
{ "type": "welcome", "you": <listener id u64> }     // first frame after connect
{ "type": "pong", "t0": <echoed>, "server_time": <ms f64> }
{ "type": "state", "state": StationState }          // on connect and after every change
{ "type": "job", "job": Job }                        // download progress updates
{ "type": "error", "message": "<text>" }
```
Types:
```
Track   { id: string, title: string, duration_ms: u64, thumbnail: bool,
          added_by: string, added_at: u64 }          // id = YouTube video id
QueueItem { queue_id: u64, track: Track, added_by: string }
StationState {
  current: QueueItem | null,
  queue: QueueItem[],                // upcoming, excludes current
  playing: bool,
  position_ms: f64,
  anchor_server_ms: f64,
  listeners: { id: u64, name: string }[],
  library: Track[],                  // newest first
  jobs: Job[]                        // active + recently failed downloads
}
Job { id: u64, url: string, status: "queued"|"downloading"|"done"|"failed",
      progress: f32 /*0..1*/, title: string|null, error: string|null, added_by: string }
```
`play` with nothing current and a non-empty queue starts the head of the queue.
Empty queue at track end → `current=null, playing=false`.

## Downloads
- Only accept URLs whose host is `youtube.com`, `www.youtube.com`, `m.youtube.com`,
  `music.youtube.com` or `youtu.be`; extract the 11-char video id; reject otherwise.
  Rebuild a canonical `https://www.youtube.com/watch?v=<id>` before calling yt-dlp
  (never pass user strings as flags).
- If the id is already in the library, skip download and just enqueue.
- One download at a time (tokio mpsc worker). Command, roughly:
  `yt-dlp --no-playlist --match-filter "duration<=1200" -x --audio-format mp3 --audio-quality 2
   --write-thumbnail --convert-thumbnails jpg --newline --progress
   -o "<music_dir>/%(id)s.%(ext)s" --print-json <canonical url>` (parse progress lines + JSON for title/duration).
- Requires `ffmpeg` on PATH.

## Persistence (SQLite via rusqlite, bundled)
- `tracks(id TEXT PK, title, duration_ms, thumbnail INT, added_by, added_at)`
- `queue(queue_id INT PK, track_id, added_by, position INT)`
- `station(k TEXT PK, v TEXT)` for current item / position, so a restart resumes paused.

## Visualizer WASM ABI (`crates/visualizer`, no wasm-bindgen, built with
`cargo build -p visualizer --target wasm32-unknown-unknown --release`, copied to
`web/static/visualizer.wasm`)
```
const FFT_SIZE = 2048; const BARS = 64;
input_ptr()  -> *mut f32   // JS writes FFT_SIZE time-domain samples (-1..1) here
output_ptr() -> *const f32 // BARS values 0..1, log-spaced 30 Hz–16 kHz, smoothed, after process()
peaks_ptr()  -> *const f32 // BARS falling peak-hold values 0..1
energy()     -> f32        // overall loudness 0..1, smoothed (drives the ambient glow)
process(sample_rate: f32, dt_seconds: f32)
```
Hann window + real FFT (`realfft`), dB mapping (-90..-10 dB → 0..1), attack/decay smoothing.

## Frontend
- SvelteKit + TypeScript, `adapter-static` with `fallback: 'index.html'`, Bits UI for
  primitives (dialogs, tooltips, slider), plain CSS with tokens. No Tailwind.
- Look: warm off-white / near-black in dark mode, one orange accent (`#F97316`-ish), lots
  of whitespace, rounded cards, Inter or similar from Google Fonts. Light + dark.
- Screens: name prompt (stored in localStorage) → "Tune in" button (needed anyway: browsers
  require a gesture to start audio) → main view: now playing (art, title, progress, play/
  pause/skip), visualizer canvas, paste-a-link box with job progress, queue (drag or
  up/down to reorder, remove), library (click to enqueue), listeners avatars.
- Motion: animated track changes, queue item enter/leave/reorder (`svelte/animate` flip),
  visualizer + ambient orange glow following `energy()`. Respect `prefers-reduced-motion`.
- Dev: `vite` proxy `/api`, `/ws`, `/audio`, `/art` → `localhost:8080`.

## Deployment
Multi-stage `Dockerfile`: build wasm → build web (copy wasm in) → build server → runtime
`debian:bookworm-slim` with `ffmpeg` and the standalone `yt-dlp_linux` binary.
`compose.yaml` for Dockge: port `8080:8080`, volume `./data:/data`,
`CLOUDBERRY_MUSIC_DIR=/data/music`, `CLOUDBERRY_DB_PATH=/data/cloudberry.db`,
`restart: unless-stopped`.

## Left to the builder
Crate versions, file/module structure, exact CSS, reorder interaction details.

---

# Phase 2: polish (added 2026-11-07, approved by Toby: "fun things that make it feel polished, like a chat on the side")

## Protocol additions
Client → server:
```
{ "type": "chat", "text": "<1–500 chars after trim>" }
{ "type": "react", "emoji": "🔥" | "😂" | "❤️" | "👏" | "🎉" | "😴" }   // anything else rejected
{ "type": "insert", "track_id": "<id>", "index": <usize> }             // used by Undo to re-add at a spot
```
Server → client:
```
{ "type": "chat_history", "messages": ChatMessage[] }   // sent once right after welcome, last 100
{ "type": "chat", "message": ChatMessage }
{ "type": "react", "listener_id": <u64>, "name": "<name>", "emoji": "<emoji>" }  // not persisted
```
```
ChatMessage { id: u64, kind: "user" | "event", name: string, text: string, at: u64 /*ms epoch*/ }
```
Server emits `event` messages (persisted like chat) for: track added ("added <title>"), skipped,
paused, resumed, removed, joined, left. `name` = who did it, `text` = the action phrase.
Rate-limit per connection: chat 5/5 s, react 10/5 s; excess silently dropped.

State/Track additions:
- `Track.gain_db: f32` — after download the server runs an ffmpeg `ebur128` measuring pass
  (`ffmpeg -nostats -i f.mp3 -filter_complex ebur128=framelog=quiet -f null -`), parses integrated
  loudness `I`, stores `gain_db = clamp(-14 - I, -12, 12)`. Clients multiply the track's GainNode by it.
  Existing tracks get measured lazily on startup (one at a time, background).
- `StationState.history: Track[]` — last 20 played, newest first.

Persistence: `chat(id INT PK, kind, name, text, at)` (keep last 500), `tracks.gain_db REAL DEFAULT 0`,
`history(track_id, played_at)`.

## Client features
- Side chat panel (collapsible; a drawer/tab on mobile), events rendered as quiet grey lines,
  autoscroll unless the user scrolled up ("new messages" pill), `/` focuses chat input.
- Emoji reaction bar; incoming reactions float up over the now-playing art (randomised x, drift,
  fade; canvas or DOM with CSS; none under prefers-reduced-motion — show a small static badge).
- Listener avatars: deterministic colour per name (hash → hue in a warm, accessible range);
  shown on queue items for `added_by`, in chat, in "who's here".
- Optional desktop notification when a track you added starts (permission asked only on toggle).
- Global paste: a YouTube URL pasted anywhere (not into an input) → `add`, with a toast.
- Shortcuts: Space play/pause, N skip, / chat, ? help dialog (Bits UI dialog). Ignored while typing.
- Undo toast (~6 s) for remove and skip ("Removed X · Undo" → `insert` at old index; skip undo
  inserts the skipped track at index 0 and sends `skip`… no: for skip, Undo = `insert` at 0 only).
- Media Session API: metadata (title, artwork `/art/{id}.jpg`), play/pause/nexttrack handlers,
  positionState kept updated.
- Tab title "▶ <title> · Cloudberry" while playing, "Cloudberry" otherwise.
- Per-track accent: `fast-average-color` on the thumbnail → CSS var `--track-tint` used by the glow
  and visualizer (blend toward brand orange; keep contrast; fallback orange).
- Synced lyrics: client fetches LRCLIB (`https://lrclib.net/api/search?q=<cleaned title>`; clean
  "(Official Video)", "[Lyrics]", "ft." etc.), takes the first result with `syncedLyrics`, parses LRC,
  highlights the current line on the shared timeline. Cache per track id in memory; silently hidden
  if nothing found. Toggle button.
- Recently played list (from `history`), click to enqueue.
- Libraries: `svelte-dnd-action` (queue reorder; on finalize send `move`, let server state win),
  `fast-average-color`, `@lucide/svelte` icons, `@fontsource-variable/inter` (self-hosted, replaces
  Google Fonts). Motion stays on built-in `svelte/motion` / transitions / `animate:flip`.

## Sync hardening (from research)
- Map AudioContext time to wall time using `ctx.getOutputTimestamp()` (contextTime ↔
  performanceTime) and subtract `ctx.outputLatency` so Bluetooth output still lines up.
- Clock offset: median of the 3 lowest-RTT samples of the last 10.
