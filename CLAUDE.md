# footballsim

A football management sim. Rust simulation core, later a TypeScript UI and
Tauri shell. Read `docs/PLAN.md` and `docs/ARCHITECTURE.md` before larger
changes.

## Commands

```bash
cargo test --workspace                          # all tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
scripts/determinism-check.sh [--size small --days 60]  # native vs WASM hashes
cargo bench -p fs-bench                         # world benchmarks
cargo run --release -p fs-cli -- squad --seed 3 # look at a generated squad
cargo run --release -p fs-cli -- run --size medium --days 30  # timing
```

## Crates

- `fs-core`: IDs, deterministic RNG, portable maths, calendar, state hashing.
- `fs-world`: world state, players (column storage), commands, daily tick,
  snapshots.
- `fs-gen`: procedural fictional world, fixed scenarios.
- `fs-cli`: headless runner; also built for `wasm32-wasip1`.
- `fs-bench`: criterion benchmarks.

## Rules that keep the simulation deterministic

- Randomness only from `fs_core::Rng`, derived with
  `world.seed().derive(Stream::X, a, b)`. Never share one RNG across
  parallel work; derive one per entity or match.
- Maths functions (`sin`, `exp`, `ln`, `powf`...) only via `fs_core::math`.
  Clippy enforces this through `clippy.toml`.
- No `HashMap`/`HashSet` in simulation code. Use `BTreeMap`, `IdVec` or
  plain vectors.
- Parallel loops go through `fs_world::par`, which falls back to sequential
  code on WebAssembly.
- Golden-value tests in `fs-core` pin the RNG and hash outputs. If one fails,
  every saved game and replay would change: fix the code, do not update the
  value unless that change is intended.

## Rules that keep online leagues possible

- The world changes only through `Game::submit(actor, Command)`. Commands
  are validated fully before any mutation and recorded in the log.
- Nothing assumes a single human manager.
- Serialised enums (`Command`, `Attr`, `Position`, `Stream`...) are
  append-only. Changing serialised types means bumping
  `snapshot::FORMAT_VERSION`.
