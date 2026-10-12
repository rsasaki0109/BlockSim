#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/rust-env.sh
# The Rust source may have changed since installation; reuse Cargo's valid outputs.
./scripts/build-wasm.sh
cd web
exec npm run dev -- --port 5173 --strictPort
