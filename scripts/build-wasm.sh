#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/rust-env.sh
cargo build --locked --release --target wasm32-unknown-unknown -p blocksim-wasm
wasm-bindgen --target web --out-dir web/src/wasm --out-name blocksim target/wasm32-unknown-unknown/release/blocksim_wasm.wasm
