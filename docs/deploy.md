# Deploying Cloudberry

One container, one volume. Pick whichever build path suits the host.

| | Compose from source | Prebuilt deploy script |
|---|---|---|
| Builds on | the server | your workstation |
| Image | [`Dockerfile`](../Dockerfile) (multi-stage: wasm → web → server → runtime) | [`deploy/Dockerfile.prebuilt`](../deploy/Dockerfile.prebuilt) (runtime only) |
| Needs on the server | Docker with BuildKit (Compose v2 / Dockge) | Docker, SSH access |
| Good for | a home server with a few GB of RAM to compile Rust | small hosts that can't compile Rust |

Both images are `debian:bookworm-slim` with `ffmpeg`, `tzdata`, the latest `yt-dlp_linux` (fetched
at build time) and `deno` (yt-dlp's JS runtime for YouTube challenges).

## Option A: Compose / Dockge, from source

```sh
git clone https://github.com/Toby-Faucher/cloudberry.git /opt/stacks/cloudberry
cd /opt/stacks/cloudberry
docker compose up -d --build
```

In Dockge the `cloudberry` stack appears under `/opt/stacks`; open it and press **Deploy**. The
first build takes a few minutes. Then open `http://<host>:8080`.

## Option B: prebuilt, via `scripts/deploy.sh`

```sh
scripts/deploy.sh [host] [port]
```

From your workstation this:

1. builds both WASM modules, the SvelteKit site, and a static musl server binary
   (`cargo zigbuild --target x86_64-unknown-linux-musl`);
2. copies them with `compose.yaml`, `deploy/Dockerfile.prebuilt` (as `Dockerfile`) and a `.env`
   containing `CLOUDBERRY_PORT=<port>` to `/opt/stacks/cloudberry` on the host over SSH;
3. runs `docker compose up -d --build` there. `./data` is never touched.

Needs locally: `cargo-zigbuild`, `zig`, `rustup target add wasm32-unknown-unknown x86_64-unknown-linux-musl`,
`npm`. The script's defaults (`10.0.0.163`, port `2279`) are Toby's home server; pass your own.

## Ports

The container listens on `8080` (`CLOUDBERRY_BIND`). The host port is `${CLOUDBERRY_PORT:-8080}`
in `compose.yaml`, so set `CLOUDBERRY_PORT` in a `.env` next to it to change it. There is no
auth: keep it on a private network or behind something that adds it.

## Volumes and permissions

`./data` on the host is mounted at `/data`:

```
data/
  cloudberry.db (+ -wal, -shm)   library, queue, chat, history, reactions
  music/{id}.mp3, {id}.jpg       downloaded audio and thumbnails
```

The [entrypoint](../docker-entrypoint.sh) starts as root only to `chown` `/data` to uid `10001`
the first time, then drops to the unprivileged `cloudberry` user. To skip the root step entirely:
`chown -R 10001:10001 data` and add `user: "10001:10001"` to `compose.yaml`.

Set `TZ` in `compose.yaml` (default there: `America/New_York`) so "DJ of the day" rolls over at
your local midnight. All variables: [README](../README.md#configuration).

## Updating

**Cloudberry** (Option A): `git pull && docker compose up -d --build`. (Option B): rerun `scripts/deploy.sh`.

**yt-dlp**: YouTube regularly breaks old versions, and the image pins whatever was "latest" at
build time. When downloads start failing, rebuild without cache:

```sh
cd /opt/stacks/cloudberry
docker compose build --no-cache --pull
docker compose up -d
```

## Backups

Everything worth keeping is in `./data`. SQLite runs in WAL mode, so either stop the container
first or take a consistent copy:

```sh
docker compose stop && tar czf cloudberry-$(date +%F).tgz data && docker compose start
# or, without downtime, the database alone (needs sqlite3 on the host):
sqlite3 data/cloudberry.db ".backup cloudberry-$(date +%F).db"
```

The MP3s can always be re-downloaded from YouTube, so the database is the irreplaceable part.

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| Downloads fail with "Sign in to confirm", 403, or "nsig extraction failed" | yt-dlp is out of date or can't solve YouTube's JS challenge | Rebuild with `--no-cache --pull` (above). Check `docker compose exec cloudberry deno --version` works. For a bare-metal run, install `deno` next to `yt-dlp`. |
| "Not a valid YouTube video link", or job fails with "longer than 20 minutes" | Not a YouTube video URL, or a video over the 20 min cap (livestreams have no duration) | Only `youtube.com`, `www.`/`m.`/`music.youtube.com` and `youtu.be` video links are accepted; a video link from a playlist adds just that video. |
| Container restarts during a download, `OOMKilled` in `docker inspect` | yt-dlp + ffmpeg spike above `mem_limit` | Raise `mem_limit` in `compose.yaml` (default `512m`). Downloads run one at a time already. |
| `cannot bind 0.0.0.0:8080` or Compose "port is already allocated" | Something else uses the port | Set `CLOUDBERRY_PORT` to a free port in `.env`. |
| Healthcheck "unhealthy" | Server didn't start; often a `/data` permission problem | `docker compose logs cloudberry`; check `data/` ownership (uid 10001). |
| Listeners are audibly out of sync | Very high latency / jittery network or a sleeping tab | Re-tune in; the player resyncs above 25 ms of drift every 2 s. |
| No sound until clicking | Browsers require a gesture for audio | That's what **Tune in** is for. |
