# footballsim

A football management game aiming to beat Football Manager on three fronts:
**speed** (simulate whole worlds in seconds), **depth** (more systems, modelled
more honestly), and **openness** (transparent stats, moddable data, scriptable
scenarios).

Status: **milestone 4 (management depth) complete.** Pick a club in a
generated world and manage it: squad, tactics and team selection, a
transfer and loan market where every club judges players by its own
scouting, backroom staff, club finances, and a board that sacks managers
who miss its targets, with a job market to find the next club. Every
match is watchable in a 2D view where you make substitutions, change
tactics and shout from the touchline. It runs in the browser (the simulation compiled to
WebAssembly), installs on iPad as a web app, and builds as a native macOS
and iPad app with Tauri. Underneath, the world runs season after season
(see [docs/WORLD.md](docs/WORLD.md)) on two calibrated match engines.

| Engine | Cost per match | Use |
| --- | --- | --- |
| Tier 1, event-level | 0.31 ms | Followed matches, predictions |
| Tier 2, statistical | 8.7 µs | Distant leagues |

Both match real top-flight statistics over 40 unseen seasons; see
[docs/CALIBRATION.md](docs/CALIBRATION.md).

## Try it

Needs Rust (the pinned version installs automatically through `rustup`)
and Node 22. To play in the browser:

```bash
cargo install wasm-bindgen-cli --version 0.2.129 --locked
cd app && npm ci && npm run wasm && npm run dev   # http://localhost:5173
```

[docs/APP.md](docs/APP.md) covers installing on iPad and the native
macOS and iPad builds. The simulation also runs headless:

```bash
cargo test --workspace
cargo run --release -p fs-cli -- career --seasons 10     # ten seasons, tables and world health
cargo run --release -p fs-cli -- squad --seed 3          # a generated squad
cargo run --release -p fs-cli -- match --home 0 --away 5 # one match, full report
cargo run --release -p fs-cli -- predict --home 0 --away 1 --runs 10000
scripts/calibration-report.sh                            # regenerate docs/CALIBRATION.md
cargo run --release -p fs-cli -- gen --size medium       # 155k-player world
cargo run --release -p fs-cli -- run --size medium --days 30
scripts/determinism-check.sh                             # native vs WebAssembly
```

## Docs

| Doc | What it covers |
| --- | --- |
| [docs/PLAN.md](docs/PLAN.md) | Vision, differentiators vs FM, feature scope, roadmap |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Tech stack, iPad strategy, crate layout, data model, performance budgets |
| [docs/MATCH_ENGINE.md](docs/MATCH_ENGINE.md) | Match engine design, fidelity tiers, calibration |
| [docs/WORLD.md](docs/WORLD.md) | Season loop: calendar, development, contracts, youth, health targets |
| [docs/APP.md](docs/APP.md) | UI architecture, the API boundary, running on web, iPad and macOS |
| [docs/CALIBRATION.md](docs/CALIBRATION.md) | Generated: engine statistics against real-football targets |

## Stack in one paragraph

All simulation lives in a **Rust** core library with no UI dependencies. The
same core compiles to native code (desktop, and iOS/iPadOS as a static
library) and to **WebAssembly** (browser, installable PWA on iPad). The UI is
a **TypeScript web app** that talks to the core through a narrow command/query
API. **Tauri 2** wraps that UI as a native app on macOS, Windows, Linux and
iPadOS. Nothing is written in Swift, and nothing is locked to Apple.
