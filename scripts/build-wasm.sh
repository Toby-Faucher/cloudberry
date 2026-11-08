#!/usr/bin/env bash
# Build the visualizer WASM and copy it to web/static/visualizer.wasm.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -p visualizer --target wasm32-unknown-unknown --release
mkdir -p web/static
cp target/wasm32-unknown-unknown/release/visualizer.wasm web/static/visualizer.wasm
echo "web/static/visualizer.wasm ($(wc -c < web/static/visualizer.wasm) bytes)"
