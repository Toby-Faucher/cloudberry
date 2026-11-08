# Cloudberry

A tiny self-hosted shared radio: everyone who tunes in hears the same track at the same moment.
Paste a YouTube link, it is downloaded to the server, and anyone can queue, skip, reorder or pause.

Design: [`docs/superpowers/specs/2026-11-07-cloudberry-design.md`](docs/superpowers/specs/2026-11-07-cloudberry-design.md).

## Local development

Needs Rust (with `rustup target add wasm32-unknown-unknown`), Node 22+, `ffmpeg` and `yt-dlp` on `PATH`.

```sh
# 1. visualizer wasm -> web/static/visualizer.wasm
./scripts/build-wasm.sh

# 2. server on :8080 (uses ./data by default)
cargo run -p server

# 3. frontend with hot reload; Vite proxies /api, /ws, /audio, /art to :8080
cd web && npm install && npm run dev
```

Tests: `cargo test --workspace`. WASM ABI smoke test: `node crates/visualizer/scripts/smoke.mjs`.

## Configuration

| Var | Default (bare) | Default (Docker image) | Meaning |
|---|---|---|---|
| `CLOUDBERRY_MUSIC_DIR` | `./data/music` | `/data/music` | where `{id}.mp3` / `{id}.jpg` are stored |
| `CLOUDBERRY_DB_PATH` | `./data/cloudberry.db` | `/data/cloudberry.db` | SQLite file |
| `CLOUDBERRY_BIND` | `0.0.0.0:8080` | `0.0.0.0:8080` | listen address |
| `CLOUDBERRY_STATIC_DIR` | `./web/build` | `/app/web/build` | built SvelteKit site |
| `CLOUDBERRY_YTDLP` | `yt-dlp` | `yt-dlp` | yt-dlp executable |

## Deploy on the home server (Dockge)

1. Put the repo in the Dockge stacks dir:
   ```sh
   git clone <repo url> /opt/stacks/cloudberry   # or rsync/copy this folder there
   ```
2. In the Dockge UI the `cloudberry` stack shows up; open it and press **Deploy**
   (Compose builds the image from the `Dockerfile`; the first build takes a few minutes).
3. Open <http://10.0.0.163:8080>.

Data (music + SQLite) lives in `/opt/stacks/cloudberry/data`. The container starts as root
only to `chown` that folder to uid `10001`, then runs as the unprivileged `cloudberry` user.
To skip the root step entirely, `chown -R 10001:10001 data` yourself and add `user: "10001:10001"`
to `compose.yaml`.

## Updating yt-dlp

YouTube regularly breaks old yt-dlp versions. The image downloads the latest `yt-dlp_linux`
release at build time, so when downloads start failing, rebuild the image:

```sh
cd /opt/stacks/cloudberry
git pull                       # optional: also update Cloudberry
docker compose build --no-cache --pull
docker compose up -d
```

(`--no-cache` forces a fresh yt-dlp download.) Your library in `data/` is kept.
