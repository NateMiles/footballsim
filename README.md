# footballsim

A football management game aiming to beat Football Manager on three fronts:
**speed** (simulate whole worlds in seconds), **depth** (more systems, modelled
more honestly), and **openness** (transparent stats, moddable data, scriptable
scenarios).

Status: **milestone 2 (season loop) complete.** The world now runs
season after season: league calendars, promotion and relegation,
injuries, suspensions, player development and ageing, contracts, youth
intakes, retirements and AI squad management, on top of two calibrated
match engines. It stays stable for decades (see
[docs/WORLD.md](docs/WORLD.md)). There is no UI yet; that is milestone 3.

| Engine | Cost per match | Use |
| --- | --- | --- |
| Tier 1, event-level | 0.31 ms | Followed matches, predictions |
| Tier 2, statistical | 8.7 µs | Distant leagues |

Both match real top-flight statistics over 40 unseen seasons; see
[docs/CALIBRATION.md](docs/CALIBRATION.md).

## Try it

Needs Rust (the pinned version installs automatically through `rustup`)
and Node 22 for the WebAssembly check.

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
| [docs/CALIBRATION.md](docs/CALIBRATION.md) | Generated: engine statistics against real-football targets |

## Stack in one paragraph

All simulation lives in a **Rust** core library with no UI dependencies. The
same core compiles to native code (desktop, and iOS/iPadOS as a static
library) and to **WebAssembly** (browser, installable PWA on iPad). The UI is
a **TypeScript web app** that talks to the core through a narrow command/query
API. **Tauri 2** wraps that UI as a native app on macOS, Windows, Linux and
iPadOS. Nothing is written in Swift, and nothing is locked to Apple.
