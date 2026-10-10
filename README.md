# footballsim

A football management game aiming to beat Football Manager on three fronts:
**speed** (simulate whole worlds in seconds), **depth** (more systems, modelled
more honestly), and **openness** (transparent stats, moddable data, scriptable
scenarios).

Status: **milestone 0 (foundations) complete.** The Rust workspace,
deterministic core, world generator, command log, save snapshots,
benchmarks and CI are in place. There is no match engine or UI yet; that
starts in milestone 1.

## Try it

Needs Rust (the pinned version installs automatically through `rustup`)
and Node 22 for the WebAssembly check.

```bash
cargo test --workspace
cargo run --release -p fs-cli -- squad --seed 3          # a generated squad
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

## Stack in one paragraph

All simulation lives in a **Rust** core library with no UI dependencies. The
same core compiles to native code (desktop, and iOS/iPadOS as a static
library) and to **WebAssembly** (browser, installable PWA on iPad). The UI is
a **TypeScript web app** that talks to the core through a narrow command/query
API. **Tauri 2** wraps that UI as a native app on macOS, Windows, Linux and
iPadOS. Nothing is written in Swift, and nothing is locked to Apple.
