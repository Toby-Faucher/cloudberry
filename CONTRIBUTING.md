# Contributing

Cloudberry is a small project for a few friends; issues and PRs are welcome, just keep changes
focused and in the spirit of [PRODUCT.md](PRODUCT.md): sync first, tiny footprint.

## Setup

| Tool | Why |
|---|---|
| Rust stable + `rustup target add wasm32-unknown-unknown` | server and the two WASM crates |
| Node 22 (`web/.npmrc` sets `engine-strict`; the Dockerfile uses `node:22`) | SvelteKit frontend |
| `ffmpeg` | audio extraction and loudness measurement |
| `yt-dlp` and `deno` | downloads; yt-dlp needs a JS runtime for YouTube challenges |
| `cargo-zigbuild` + `zig` (optional) | only for `scripts/deploy.sh` |

```sh
./scripts/build-wasm.sh              # web/static/visualizer.wasm + smear.wasm
cargo run -p server                  # :8080, data in ./data
cd web && npm install && npm run dev # :5173 with proxy to :8080
```

For UI-only work, `cd web && node scripts/mock-server.mjs` replaces the Rust server.

## Tests

```sh
cargo test --workspace                    # server (incl. WebSocket integration), visualizer, smear
cd web && npm run check && npx vitest run # svelte-check + unit tests
node crates/visualizer/scripts/smoke.mjs  # raw WASM ABI smoke tests (after build-wasm.sh)
node crates/smear/scripts/smoke.mjs
```

## Style

- Rust: `cargo fmt --all` and `cargo clippy --workspace --all-targets` clean.
- Web: tabs, Svelte 5 runes, plain CSS with tokens (no Tailwind); `npm run check` clean.
- Respect `prefers-reduced-motion` and keyboard use in anything visual.
- Nothing visual may cost audio timing: keep heavy work off the main thread or out of hot paths.

## Commits and PRs

- Short imperative subject (`Server: add poke rate limit`), a body for the why if it isn't obvious.
- One topic per PR; say how you tested it and add screenshots for UI changes
  (`scripts/screenshots.mjs` regenerates the README ones).
- Releases and version numbers: [docs/versions.md](docs/versions.md).
- Protocol changes: update the [design spec](docs/superpowers/specs/2026-11-07-cloudberry-design.md)
  in the same PR.
