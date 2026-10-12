#!/usr/bin/env bash
# Source this file to activate tools installed by setup.sh; preserve other installs.
BLOCKSIM_TOOL_ROOT="${BLOCKSIM_TOOL_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/.blocksim-tools}"
export BLOCKSIM_TOOL_ROOT
if [[ -x "$BLOCKSIM_TOOL_ROOT/cargo/bin/rustup" ]]; then
  export RUSTUP_HOME="$BLOCKSIM_TOOL_ROOT/rustup"
  export CARGO_HOME="$BLOCKSIM_TOOL_ROOT/cargo"
  export PATH="$CARGO_HOME/bin:$PATH"
fi
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
