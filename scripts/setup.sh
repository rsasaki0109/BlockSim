#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/rust-env.sh
mkdir -p "$BLOCKSIM_TOOL_ROOT"
# Reuse a system Rust installation when available, otherwise install outside checkout.
if ! command -v rustup >/dev/null 2>&1; then
  export RUSTUP_HOME="$BLOCKSIM_TOOL_ROOT/rustup"
  export CARGO_HOME="$BLOCKSIM_TOOL_ROOT/cargo"
  export PATH="$CARGO_HOME/bin:$PATH"
  installer="$(mktemp "$BLOCKSIM_TOOL_ROOT/rustup-init.XXXXXX")"
  trap 'rm -f "$installer"' EXIT
  curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 https://sh.rustup.rs -o "$installer"
  sh "$installer" -y --no-modify-path --profile minimal --default-toolchain 1.90.0
fi
if ! rustup run 1.90.0 rustc --version >/dev/null 2>&1; then
  rustup toolchain install 1.90.0 --profile minimal
fi
if ! rustup run 1.90.0 rustfmt --version >/dev/null 2>&1; then
  rustup component add --toolchain 1.90.0 rustfmt
fi
if ! rustup run 1.90.0 cargo-clippy --version >/dev/null 2>&1; then
  rustup component add --toolchain 1.90.0 clippy
fi
installed_targets="$(rustup target list --toolchain 1.90.0 --installed)"
if [[ "$installed_targets" != *wasm32-unknown-unknown* ]]; then
  rustup target add --toolchain 1.90.0 wasm32-unknown-unknown
fi
if [[ "$(wasm-bindgen --version 2>/dev/null || true)" != 'wasm-bindgen 0.2.100' ]]; then
  cargo install wasm-bindgen-cli --version 0.2.100 --locked
fi
node --version
npm --version
(cd web && npm ci --cache "$BLOCKSIM_TOOL_ROOT/npm" --no-audit --no-fund)
./scripts/build-wasm.sh
(cd web && npm run build)
