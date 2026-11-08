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
- Undo toast (~6 s) for remove and skip. Remove → Undo sends `insert` at the old index.
  Skip → Undo sends `insert` of the skipped track at index 0 (it plays next, from the start).
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

---

# Phase 2: visualization (added 2026-11-07, Toby: "want to be sure we work on some cool audio visualization")

## WASM ABI additions (backward compatible; existing exports unchanged)
```
bass()   -> f32   // smoothed 0..1, ~30–150 Hz
mid()    -> f32   // smoothed 0..1, ~150 Hz–2 kHz
treble() -> f32   // smoothed 0..1, ~2–16 kHz
beat()   -> f32   // 1.0 on a detected onset, decays to 0 over ~250 ms (uses dt)
waveform_ptr() -> *const f32   // 256 downsampled time-domain samples (-1..1), for oscilloscope rings
```
Beat detection: spectral flux on the low/low-mid bands with an adaptive threshold (mean + k·stddev
over the last ~1 s of flux values) and a ~200 ms refractory period. Native tests: a synthetic kick
pattern (decaying 60 Hz bursts every 500 ms) yields beats at those times and none in between; steady
tone yields none.

## Modes (web, all in `web/src/lib/visualizer/`)
Single rAF loop owns analysis (one wasm `process` per frame) and exposes a reactive store/rune of
`{bars, peaks, waveform, energy, bass, mid, treble, beat}`; renderers subscribe.
1. **Halo** (default): Canvas 2D ring of 64 radial bars around the now-playing art (mirrored, slow
   rotation), thin waveform ring, art scales ~1–3% on `beat`, glow tinted `--track-tint`.
2. **Bloom**: WebGL2 fullscreen fragment shader, domain-warped noise gradient in warm orange /
   `--track-tint`; `bass` → warp strength, `beat` → brief luminance swell, `treble` → fine sparkle,
   `energy` → overall brightness. Rendered at ~0.5× resolution, upscaled. Also used at low opacity as
   the app background (`<Backdrop/>`).
3. **Bars**: Canvas 2D bottom spectrum with falling peak caps and rounded tops.
- **Lean-back mode** (`F` key / button): fullscreen (Fullscreen API), hides controls, big art + title +
  lyrics over Bloom; Esc exits.
- Mode choice in localStorage (try/catch). Pause the loop when `document.hidden`; cap DPR at 2.
- `prefers-reduced-motion`: no animation, static gradient + still bars snapshot.
- WebGL2 unavailable → Bloom falls back to a CSS radial-gradient driven by `energy`.
- Public surface for the rest of the app: `<Visualizer mode={...}/>`, `<Backdrop/>`, `<LeanBack/>`,
  and `startAnalysis(analyser: AnalyserNode)` / `stopAnalysis()`.

---

# Phase 3: game feel (added 2026-11-07; Toby: "cloudberry orange", "a shit load of motion using anime.js",
# "any place you can add smear motion add it using wasm", "should feel like a game UI, super clear",
# and approved all ten quirks below)

## Colour: real cloudberry, not generic orange
Ripe cloudberries are a warm amber-orange (honey-gold in the light, deeper orange in shadow); unripe
ones blush red. Replace every `#F97316` with tokens:
```
--berry-300: #FBC77A   /* highlight, glows */
--berry-400: #F7AE4A
--berry-500: #F0912B   /* THE accent: buttons, progress, bars */
--berry-600: #D9741A   /* hover/pressed */
--berry-700: #A9550F   /* accent text on light bg (verify ≥ 4.5:1) */
--blush-500: #E0553A   /* unripe blush: heat states — combos, hold-to-skip near completion, live dot */
```
`--track-tint` blends toward `--berry-500`. Visualizer fallback colour = `--berry-500`. Check contrast in
light and dark; dark mode may use `--berry-400` for accent text.

## Game-UI direction: super clear
Think polished game HUD, not dashboard: big confident type for the current song, obvious primary
actions, strong states (hover/press/disabled/active are unmistakable), chunky hit targets, icon+label
buttons, clear counters, every action gives immediate feedback (press squash, ripple, toast). Never
cluttered: one focal point (now playing), everything else secondary. Readability first.

## Visual language: flat sticker, not soft-glow SaaS (Toby, later the same day)
"There is a lot of AI-looking UI … clean it up." Reference: a flat vector sticker logo (simple shapes,
2–3 flat tones each, one white highlight, thick white die-cut outline, chunky rounded lowercase
wordmark). So: Fredoka (self-hosted) for display/HUD, Inter for text; flat fills, hard-stop tones,
a white die-cut outline (`--die-cut`) + game-button lip (`--lip`) on the things you press; no blur
glows, no glass, no ambient backdrop (the `<Backdrop/>` app background is removed), no idle motion.
Visualizer glows become crisp shapes (Bloom is posterised; Halo/now-playing glow is a flat disc).

## Motion: anime.js v4 everywhere (`animejs`, MIT)
`import { animate, createTimeline, stagger, spring, utils } from 'animejs'`. Central helpers in
`web/src/lib/motion/` (e.g. `motion.ts`) with a `reduced` flag from `prefers-reduced-motion`
(reduced → instant or opacity-only). Use for: page/section entrance staggers, button press squash
& stretch, toasts, queue item enter/leave (keep `animate:flip` for reorder, or replace with anime FLIP),
reactions float-up, title card, combo counter, lean-back enter/exit, avatar join/leave/poke/bounce,
countdown pulse, berry rain. Svelte transitions may stay where simpler, but anime.js is the motion voice.

## Smear motion via WASM (`crates/smear`, no wasm-bindgen → `web/static/smear.wasm`)
Cartoon smear frames: fast-moving elements stretch along their velocity and squash across it, with a
directional blur at peak speed, then snap back. Physics in WASM; JS applies transforms.
ABI (all `extern "C"`, static buffers, MAX_BODIES = 256):
```
bodies_ptr()  -> *mut f32   // JS writes per body [x, y] current position (px) each frame
smear_ptr()   -> *const f32 // per body [stretch (0..~1.5), angle (rad), blur (px), speed (px/s)]
step(n_bodies: u32, dt: f32) // velocity from finite differences, critically-damped smoothing,
                             // stretch = clamp(k * speed), decays smoothly when stopping
reset(i: u32)               // forget history for body i (on spawn/teleport)
rain_spawn(count: u32, width: f32, seed: u32)   // berry rain particles (cap 256)
rain_step(dt: f32, width: f32, height: f32) -> u32   // gravity, air drag, wobble, floor bounce; returns live count
rain_ptr() -> *const f32    // per particle [x, y, vx, vy, rot, size, life]
```
JS bridge `web/src/lib/motion/smear.ts`: `smear.track(el)` / `smear.untrack(el)`; one rAF loop
reads each tracked element's animated position (from anime.js values, not layout reads where avoidable),
calls `step`, writes `transform: … rotate(a) scale(1+s, 1/(1+s)) rotate(-a)` and `filter: blur()` only
above a threshold. Plus `smearAnimate(el, animeParams)` = anime `animate` + auto track/untrack.
Fallback if wasm fails: same API, plain JS math. Reduced motion → no smear.
Apply smear to: reactions flying, title card slam-in, track art swap, queue items moving fast
(drag release/reorder), toasts entering, poke wiggle, combo counter pops, lean-back transition,
berry rain (canvas, per-particle stretch from velocity).
Native tests: still body → stretch 0; constant fast velocity → stretch > 0 and aligned angle;
stop → stretch decays to ~0 within ~150 ms; rain particles fall and stay within bounds.

## The ten quirks (all approved)
1. **Hold to skip**: press-and-hold ~600 ms; a ring fills (berry → blush near completion), release early
   cancels; keyboard `N` also requires hold. Click = small "hold to skip" hint wiggle.
2. **Title card**: on track change a big title (+ "added by") slams in centred with smear, holds ~1.2 s,
   then flies into its spot in the now-playing card.
3. **Reaction combos**: ≥ 3 of the same emoji within 2 s (from anyone) → "🔥 ×N COMBO" counter that grows,
   shakes, colour heats toward blush; resets after 2 s idle. Client-only.
4. **DJ of the day**: crown on that listener's avatar everywhere. Server field below.
5. **Sync readout**: HUD chip "SYNC 3 ms" from the player's latest measured drift (abs, ms), green < 10,
   amber < 30, red ≥ 30 or "—" when not playing; tooltip explains.
6. **Poke**: click another listener's avatar → their avatar wiggles on everyone's screen; the target also
   gets a toast "Toby poked you". Not self. Server rate limit 1 per 2 s per connection.
7. **Away state**: `visibilitychange` → `presence` message; away avatars fade/desaturate; bounce on react.
8. **Up-next countdown**: chip "Up next in 0:09 · <title>" visible in the last 10 s, pulsing each second.
9. **Auto-pause when everyone leaves**: server pauses when the last listener disconnects (event message
   "paused" with name "Cloudberry").
10. **Konami code** (↑↑↓↓←→←→BA) → cloudberry rain (WASM `rain_*`, canvas, smeared), ~4 s, local only.

## Protocol additions (phase 3)
Client → server:
```
{ "type": "poke", "listener_id": <u64> }
{ "type": "presence", "away": <bool> }
```
Server → client:
```
{ "type": "poke", "from_id": <u64>, "from_name": "<name>", "to_id": <u64> }   // broadcast
```
State additions: `listeners[].away: bool` (default false), `StationState.dj_of_day: string | null`
= name whose added tracks received the most reactions today (server local day; `TZ` env in compose,
default `America/New_York`), ties → most recent reaction wins. Persist `reactions(day TEXT, name TEXT,
count INT, last_at INT, PRIMARY KEY(day, name))`; a reaction counts toward the *current track's*
`added_by` (not the reactor). Poke with unknown/self target → `error`.
