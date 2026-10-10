#!/usr/bin/env bash
# Builds the simulation for the browser into app/src/wasm/pkg.
#
# Needs the wasm32-unknown-unknown target (rust-toolchain.toml installs
# it) and wasm-bindgen-cli at the same version as the wasm-bindgen crate:
#   cargo install wasm-bindgen-cli --version 0.2.129 --locked
set -euo pipefail
cd "$(dirname "$0")/.."

want=$(grep -E '^wasm-bindgen = ' Cargo.toml | sed -E 's/.*"=?([0-9.]+)".*/\1/')
have=$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || true)
if [[ "$have" != "$want" ]]; then
  echo "wasm-bindgen-cli $want is needed (found '${have:-none}'):" >&2
  echo "  cargo install wasm-bindgen-cli --version $want --locked" >&2
  exit 1
fi

cargo build -p fs-wasm --target wasm32-unknown-unknown --profile wasm
wasm-bindgen --target web --out-dir app/src/wasm/pkg \
  target/wasm32-unknown-unknown/wasm/fs_wasm.wasm
ls -lh app/src/wasm/pkg/fs_wasm_bg.wasm
