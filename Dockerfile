# syntax=docker/dockerfile:1.7
# Needs BuildKit (default in Docker 23+ / Compose v2, which Dockge uses).

# --- 1. visualizer wasm ---------------------------------------------------------
FROM rust:1-bookworm AS wasm
RUN rustup target add wasm32-unknown-unknown
WORKDIR /src
# The whole workspace is needed because members = ["crates/*"].
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target,id=cloudberry-target-wasm \
    cargo build -p visualizer --target wasm32-unknown-unknown --release \
 && cp target/wasm32-unknown-unknown/release/visualizer.wasm /visualizer.wasm

# --- 2. web (SvelteKit static build) ------------------------------------------------
FROM node:22-bookworm-slim AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN --mount=type=cache,target=/root/.npm npm ci
COPY web ./
COPY --from=wasm /visualizer.wasm static/visualizer.wasm
RUN npm run build

# --- 3. server -------------------------------------------------------------------
FROM rust:1-bookworm AS server
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target,id=cloudberry-target-server \
    cargo build -p server --release \
 && cp target/release/cloudberry /cloudberry

# --- 4. runtime ------------------------------------------------------------------
FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ffmpeg ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --uid 10001 --user-group --create-home --home-dir /home/cloudberry cloudberry
# "latest" is resolved at build time; rebuild the image to pick up a new yt-dlp.
ADD --chmod=755 https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux /usr/local/bin/yt-dlp
# yt-dlp needs a JS runtime (deno by default) to solve YouTube challenges.
COPY --from=denoland/deno:bin-2.9.8 /deno /usr/local/bin/deno
COPY --from=server /cloudberry /usr/local/bin/cloudberry
COPY --from=web /src/web/build /app/web/build
COPY --chmod=755 docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh

ENV CLOUDBERRY_MUSIC_DIR=/data/music \
    CLOUDBERRY_DB_PATH=/data/cloudberry.db \
    CLOUDBERRY_STATIC_DIR=/app/web/build \
    CLOUDBERRY_BIND=0.0.0.0:8080 \
    CLOUDBERRY_YTDLP=yt-dlp \
    HOME=/home/cloudberry
WORKDIR /app
VOLUME /data
EXPOSE 8080
# Starts as root only to fix /data ownership, then drops to uid 10001 (see entrypoint).
ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["cloudberry"]
