#!/usr/bin/env bash
# Build the WASM crates (visualizer, smear) and copy them to web/static/.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -p visualizer -p smear --target wasm32-unknown-unknown --release
mkdir -p web/static
for name in visualizer smear; do
	cp "target/wasm32-unknown-unknown/release/$name.wasm" "web/static/$name.wasm"
	echo "web/static/$name.wasm ($(wc -c < "web/static/$name.wasm") bytes)"
done
