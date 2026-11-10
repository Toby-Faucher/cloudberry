#!/usr/bin/env bash
# Build Cloudberry locally and run it on a Docker host over SSH.
# Usage: scripts/deploy.sh [host] [port]   (defaults: 10.0.0.163, 2279 = "CBRY" on a phone keypad)
# Needs: cargo-zigbuild + zig (static musl build), npm, ssh access with docker.
set -euo pipefail
HOST=${1:-10.0.0.163}
PORT=${2:-2279}
DEST=/opt/stacks/cloudberry
cd "$(dirname "$0")/.."

./scripts/build-wasm.sh
(cd web && npm ci --silent && npm run build)
cargo zigbuild -p server --release --target x86_64-unknown-linux-musl

STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT
mkdir -p "$STAGE/web"
cp target/x86_64-unknown-linux-musl/release/cloudberry docker-entrypoint.sh "$STAGE/"
cp -r web/build "$STAGE/web/build"
cp deploy/Dockerfile.prebuilt "$STAGE/Dockerfile"
cp compose.yaml "$STAGE/compose.yaml"
echo "CLOUDBERRY_PORT=$PORT" > "$STAGE/.env"

# Replace the app files but never touch ./data (music + db volume).
tar -C "$STAGE" -cz . | ssh "$HOST" "mkdir -p $DEST && rm -rf $DEST/web && tar -xz -C $DEST && chmod 755 $DEST"
# Keep the running image as :previous for a rollback (docs/versions.md). None yet on a first deploy.
ssh "$HOST" "docker tag cloudberry-cloudberry:latest cloudberry-cloudberry:previous 2>/dev/null || true"
ssh "$HOST" "cd $DEST && docker compose up -d --build && docker compose ps"
echo "Cloudberry: http://$HOST:$PORT"
