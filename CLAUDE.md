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
cargo run --release -p fs-cli -- career --seasons 10           # season loop + world health
cargo run --release -p fs-cli -- economy --size small --seasons 10  # money by reputation band
cargo run --release -p fs-lab --example profile_season         # medium-world day timings
cargo run --release -p fs-cli -- match --home 0 --away 5       # match report
cargo run --release -p fs-cli -- predict --home 0 --away 1     # Monte Carlo
scripts/calibration-report.sh                   # regenerate docs/CALIBRATION.md
cargo run --release -p fs-lab --example tune -- slope=3.0      # tier 1 tuning
cargo run --release -p fs-cli -- fit-tier2 --seasons 40 --seed 1000 \
  --out crates/fs-match/src/fitted.rs           # refit tier 2 to tier 1
UPDATE_BINDINGS=1 cargo test -p fs-api --test bindings  # regenerate TS types

# Web app (in app/; see docs/APP.md)
npm run wasm && npm run dev                     # core to WASM, dev server
npm run check && npm test                       # types, unit tests
npm run build && npm run e2e                    # Playwright career test
# Here, Playwright needs CHROMIUM=/opt/pw-browsers/chromium-1194/chrome-linux/chrome
```

## Crates

- `fs-core`: IDs, attributes, deterministic RNG, portable maths, calendar,
  state hashing, parallel helpers.
- `fs-match`: match engines (tier 1 event, tier 2 statistical), team
  selection, formations.
- `fs-world`: world state, players (column storage), commands, daily tick,
  snapshots, leagues and calendar (`league`, `competition`, `season`),
  matchdays (`matchday`), player factory and development (`people`), AI
  squad management (`squad`), money (`finance`, `economy`), staff
  (`staff`), scouting (`scouting`), transfers and loans (`transfer`, AI
  in `market`), boards and jobs (`board`). Rules are described in
  `docs/WORLD.md`.
- `fs-save`: save container (header, deflated sections, checksums,
  migrations).
- `fs-lab`: season simulation, calibration report, tier 2 fitting,
  prediction.
- `fs-gen`: procedural fictional world, fixed scenarios.
- `fs-api`: the only boundary to any UI: JSON protocol, view models
  (`dto`, mirrored to `app/src/api/generated.ts`), queries, inbox, live
  match sessions with commentary, saves.
- `fs-wasm`: `fs-api` session for the browser (wasm-bindgen).
- `fs-cli`: headless runner; also built for `wasm32-wasip1`.
- `fs-bench`: criterion benchmarks.
- `app/`: Svelte + TypeScript UI; `app/src-tauri`: native shell (its own
  Cargo workspace).

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

## Rules for the match engine

- After changing tier 1 (`tier1.rs`, `skills.rs`, `strength.rs`,
  formations, selection), rerun `scripts/calibration-report.sh`; every
  check must pass. Then refit tier 2 with `fit-tier2 --out
  crates/fs-match/src/fitted.rs`. CI fails if either is stale.
- Tune against the default tuning seeds; the report uses seeds 5000+ so
  it stays an out-of-sample check.
- No tactic slider may be worth more than 4 points a season either way.

## Rules for the world simulation

- After changing season-loop rules (development, youth, contracts,
  retirement, squad AI, money, staff, the market, boards), run
  `fs-cli career --seasons 30` and `fs-cli career --size small --seasons
  10`, and check the health lines (ages, ability, economy, market,
  sackings) stay within the targets in `docs/WORLD.md`; the
  `ten_seasons_keep_the_world_healthy` test enforces the core of them.
- Effects of staff and facilities are relative to what a club of that
  standing usually has, so the world's overall level holds. Keep new
  effects centred the same way.
- Generator changes alter the calibration league: rerun
  `scripts/calibration-report.sh` and the tier 2 fit.

## Rules that keep online leagues possible

- The world changes only through `Game::submit(actor, Command)`. Commands
  are validated fully before any mutation and recorded in the log.
- Nothing assumes a single human manager.
- Serialised enums (`Command`, `Attr`, `Position`, `Stream`...) are
  append-only. Changing serialised types means bumping
  `snapshot::FORMAT_VERSION`.

## Rules for the UI

- The UI reaches the core only through `fs-api` requests. Never expose
  core types to TypeScript; add a view model in `fs_api::dto`.
- After changing the protocol or a view model, regenerate the bindings;
  bump `PROTOCOL_VERSION` when old clients would break.
- A watched match must be the match the world plays: nothing may change
  a team's match input between `StartMatch` and `FinishMatch`.
