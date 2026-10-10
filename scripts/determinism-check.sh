#!/usr/bin/env bash
# Runs the fixed determinism scenario natively and as WebAssembly (WASI,
# under Node) and fails if the hashes differ.
#
# Usage: scripts/determinism-check.sh [extra fs-cli args, e.g. --size small]
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build --quiet --release -p fs-cli
cargo build --quiet --release -p fs-cli --target wasm32-wasip1

native=$(./target/release/fs-cli determinism "$@")
# --single-threaded-gc: Node 22's parallel garbage collector crashed (SIGSEGV)
# on large WASI heaps in our cloud sandbox. The flag only changes how Node
# collects its own garbage, not what the WebAssembly program computes.
wasm=$(node --no-warnings --single-threaded-gc scripts/run-wasi.mjs target/wasm32-wasip1/release/fs-cli.wasm determinism "$@")

echo "native:"; echo "$native"
echo "wasm:";   echo "$wasm"
if [[ "$native" != "$wasm" ]]; then
  echo "DETERMINISM FAILURE: native and WebAssembly results differ" >&2
  exit 1
fi
echo "OK: native and WebAssembly agree"
