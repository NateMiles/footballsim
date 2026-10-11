# Architecture

## 1. The iPad question

**Goal:** run well on iPad without locking the project to Swift or to Apple.

**Decision:** Rust simulation core + TypeScript web UI, delivered two ways:

| Delivery | How it runs on iPad | Needs | When |
| --- | --- | --- | --- |
| **PWA** (progressive web app) | Core compiled to WebAssembly, runs in a Web Worker inside Safari. "Add to Home Screen" gives a full-screen app icon. | Any web host. No Mac, no Apple account. | From M3 |
| **Native app via Tauri 2** | Core compiled to native ARM64 and linked into an iPadOS app. UI runs in the system WebView. | A Mac with Xcode (available). A free Apple ID gives 7-day installs on your own iPad; the paid developer account gives year-long installs, TestFlight and the App Store. | Dev builds from M3, polished in M6 |

Why this beats the alternatives:

- **Rust** compiles ahead of time to native code on every platform, including
  iOS, so the simulation runs at C++-class speed. It has no garbage collector
  pauses, has safe multithreading (`rayon`), and compiles to WebAssembly with
  first-class tooling. It is the fastest portable option that is still pleasant
  to write.
- **Swift-only** would tie everything to Apple platforms. Swift itself is not
  slow, but a Swift core could not run in a browser or on Windows.
- **C#/Unity** works on iPad, but Unity's UI tooling is poor for a
  spreadsheet-heavy management game, and IL2CPP builds are slow to iterate.
- **A web UI** is the best tool for dense tables, filters, charts and
  responsive layouts, which is most of a management game. The match view is
  a canvas/WebGL layer, which web tech handles well in 2D.
- **Escape hatch.** Because the core exposes a narrow API, a native SwiftUI
  or other UI could be bolted on later (via UniFFI bindings) without touching
  simulation code. The UI choice is reversible; the core choice is the one
  that matters.

WASM vs native on iPad: the native build is faster (real threads, SIMD, no
sandbox memory cap), so heavy features such as large worlds and the tactic lab
will be tuned for native. The PWA must stay fully playable on a medium-sized
world.

## 2. System overview

```
+-------------------------------------------------------------+
|  UI (TypeScript, Svelte, Vite)                             |
|  screens, tables, charts, 2D match renderer (PixiJS/canvas) |
+------------------------------+------------------------------+
                               | commands / queries / events
                               | (serialised, versioned API)
+------------------------------v------------------------------+
|  Host adapter                                               |
|  - WASM: wasm-bindgen + Web Worker (+ worker pool)          |
|  - Native: Tauri commands, core runs on its own threads     |
+------------------------------+------------------------------+
                               |
+------------------------------v------------------------------+
|  Rust core (no UI, no I/O assumptions)                      |
|  world state | scheduler | match engine | AI | persistence  |
+-------------------------------------------------------------+
```

The UI never touches world state directly. It sends **commands** ("offer
contract", "set tactic", "advance to next event"), runs **queries** ("squad
view for club X", "player search with filters"), and receives **events**
("match finished", "bid received"). Query results are view models shaped for
one screen, so the UI stays thin and the core stays free to change its
internal layout.

## 3. Repository layout (planned)

```
footballsim/
  Cargo.toml                 # workspace
  crates/
    fs-core/                 # built: IDs, attributes, RNG, maths, time, hashing
    fs-match/                # built: match engines (tiers 1 and 2), selection
    fs-world/                # built: world state, commands, season loop,
                             # injuries, development, contracts, youth,
                             # finances, staff, scouting, transfers, boards,
                             # competition formats (data), cups, level of
                             # detail
    fs-gen/                  # built: procedural world and name generation
    fs-lab/                  # built: calibration, tier 2 fitting, prediction
    fs-cli/                  # built: headless runner
    fs-bench/                # built: benchmarks
    fs-ai/                   # planned: richer AI managers and directors
                             # (today's AI lives in fs-world's squad,
                             # market and board modules)
    fs-data/                 # planned: data packs (names, nations, clubs);
                             # competition structures are already data
                             # (fs_world::format, `fs-cli format`)
    fs-save/                 # built: save container, versioned migrations
    fs-api/                  # built: request/response protocol, views, session
    fs-wasm/                 # built: wasm-bindgen wrapper around fs-api
  app/                       # built: Svelte + TypeScript UI (see APP.md)
  app/src-tauri/             # built: Tauri 2 shell (desktop + iPadOS)
  data/                      # planned: base data packs
  docs/
```

## 4. World data model

### Entities and IDs
- Every entity (player, club, staff, competition, match) gets a dense `u32`
  newtype ID, used as an index into arrays. No pointers, no reference counting.
- Relationships are stored as IDs, so the whole world can be snapshotted,
  diffed and hashed cheaply.

### Hot/cold split, struct-of-arrays
The daily tick touches every active player (fitness, morale, training,
development). To make that fast and cache-friendly:

- **Hot data** lives in struct-of-arrays columns: attributes packed as `u8`,
  fitness and fatigue as `u16` or `f32`, current club, position familiarity
  bitmasks. A daily pass is a linear scan over a few columns, which
  parallelises trivially.
- **Cold data** (history, biography, contract clauses, relationships, career
  stats) lives in separate tables loaded and touched only when needed.

### Attributes
- Each player has hidden **true values** on a fine scale (0-1000) for
  technical, mental, physical and goalkeeping attributes, plus hidden traits:
  consistency, big-match temperament, injury proneness, adaptability,
  professionalism, ambition.
- The UI shows the familiar **1-20 scale**, but as a club-specific estimate
  with an uncertainty range from scouting knowledge.
- Physical attributes map to physical units where possible: top speed in m/s,
  acceleration in m/s², stamina as an energy budget. The match engine uses
  these units directly.

### Memory budgets
| World size | Players + staff | Budget |
| --- | --- | --- |
| Small (PWA default) | ~84k | < 150 MB (measured: 72 MB native, 45 MB WASM) |
| Medium | ~210k | < 300 MB (measured: 137 MB native, 94 MB WASM) |
| Large (native only) | ~500k | < 800 MB |

## 5. Simulation scheduling and LOD

The world advances in **days**. Each day (`fs_world::tick`):

1. Calendar business: the 1 July rollover, contract renewals, squad
   housekeeping.
2. Every league and cup fixture due today, played in parallel, each at the
   tier its importance deserves; then the day's cup draws.
3. Per-player passes (recovery, injuries) as parallel column scans with
   per-player random streams.
4. AI decisions on a staggered schedule: about one club in eight looks at
   the transfer market each day; each club's players develop on the club's
   own day of the month; each AI club reviews its tactic every 30, 60 or
   90 days by importance, on its own days.
5. Money and boards on the 1st of the month. Events for the UI.

**Importance-based LOD** (`fs_world::importance`). Every club gets an
importance from where the human managers are, and nothing assumes there is
only one of them:

| Importance | Who | League matches | AI tactic review |
| --- | --- | --- | --- |
| High | human clubs and their leagues | tier 1 (event) | every 30 days |
| Medium | the rest of their nations; clubs in their continental cups | tier 2 | every 60 days |
| Low | everyone else | tier 2 | every 90 days |

Cup matches use tier 1 whenever a human club plays. Tier 2 is fitted to
tier 1 (`docs/CALIBRATION.md`), so leagues played at low detail produce the
same statistics. With the `profile` feature, `fs_world::profile` times each
stage of the tick (`cargo run --release -p fs-lab --example profile_season
--features fs-world/profile`).

## 6. Performance budgets

Targets on a recent iPad running the native build. Measured with
`cargo run --release -p fs-api --example budgets [small|medium]` on the
4-core cloud build machine; the browser column is the same build as
WebAssembly in headless Chromium, on one thread. The app's own performance
test (title screen or Saves screen) measures any device, including an iPad.

| Operation | Target | Native, 4 threads | Browser (WASM, 1 thread) |
| --- | --- | --- | --- |
| Tier 1 event-level match | < 1 ms | 0.31 ms | |
| Tier 2 statistical match | < 10 µs | 8.7 µs | |
| Daily world tick, medium world (quiet day) | < 50 ms | 22 ms over a season; 50 ms in the summer window | |
| Full matchday, all leagues, medium world | < 1 s | 70 ms mean, 120 ms worst | |
| One season in holiday mode, medium world | < 60 s | 15 s (36 s on one thread) | about 39 s |
| One season, small world | | 6.5 s (10.6 s on one thread) | about 15 s |
| Match predictor, 10,000 runs at tier 1 | < 2 s | 0.73 s | |
| Player search over 209k players | < 20 ms | 11 ms filtered by position, 50 ms unfiltered | |
| Memory, small world (budget 150 MB) | | 72 MB resident | 45 MB WASM memory |
| Memory, medium world (budget 300 MB) | | 137 MB resident | 94 MB WASM memory |

The 1 July rollover (about 1 s in the medium world) is the slowest day of
the year. The browser numbers come from a desktop CPU; an iPad's single
core is in the same class, but run the app's performance test to know.

## 7. Determinism

- One RNG algorithm everywhere (a counter-based or PCG-family generator),
  seeded per match and per system from the world seed. Never use OS
  randomness in the sim.
- Parallel work never shares an RNG. Each match or entity derives its own
  stream from `(world_seed, day, entity_id)`, so results do not depend on
  thread scheduling.
- Floating point: basic arithmetic and `sqrt` are IEEE-exact across
  platforms, and Rust never fuses multiply-adds on its own. Functions such as
  `sin`, `cos`, `exp` and `atan2` go through the portable `libm` crate so
  native and WASM agree bit-for-bit.
- No iteration over hash maps in sim code paths (order is not stable).
- CI runs the same seeded season on native and WASM and compares world
  hashes.

## 8. Persistence

- Save format (`fs-save`): a container with a JSON header and deflated
  sections: the world snapshot (serde + postcard), the command log, an
  optional origin world for replays, and host state. Deflate (pure Rust,
  so it also runs in WebAssembly) brings snapshots to about 60% of their
  raw size: about 9 MB for the small world.
- Saves carry a schema version; `fs-save` owns migrations so old saves keep
  loading.
- Replays store only the seed and the manager's commands with timestamps, so
  a match replay is a few kilobytes.
- Storage: files on native, IndexedDB/OPFS in the browser.

## 9. Testing strategy

- **Unit tests** per crate.
- **Determinism tests** as described above.
- **Calibration suite.** Simulate thousands of matches and seasons, then
  assert statistical targets (goals per game, home advantage, shot counts,
  table spreads, age curves, transfer price ranges) fall within tolerance.
  This is the main guard against the engine silently getting worse.
- **Benchmarks** with `criterion`, tracked per commit to catch performance
  regressions.
- **Long-run soak test.** Simulate 50 seasons headless and check the world
  stays healthy: no bankrupt-everyone spirals, no squad-size explosions,
  realistic ability distribution.

## 10. Tooling

| Area | Choice |
| --- | --- |
| Core language | Rust (stable) |
| Parallelism | rayon |
| WASM | wasm-bindgen (CLI pinned to the crate version), Web Workers |
| UI | TypeScript, Vite, Svelte 5; Canvas2D for the match view behind a renderer interface (WebGL or 3D later) |
| Native shell | Tauri 2 (desktop + iOS) |
| Serialisation | serde, compact binary codec, zstd |
| Benchmarks | criterion |
| CI | GitHub Actions: fmt, clippy, tests, WASM build, determinism check, calibration, web app (types, unit, e2e), native shell build |

## 11. Designing for online leagues later

Online leagues are not being built now. These rules apply from M0 so that
adding them later is an add-on, not a rewrite.

### Target model
- **Server-authoritative shared world.** A server runs the same Rust core
  headless and owns the canonical world. Each human manages one club.
- **Turn-like matchdays.** Managers submit commands (team selection, tactics,
  bids) before a deadline. The server advances the world and publishes the
  result. Because the sim is deterministic, clients can replay matches
  locally from the seed and commands instead of downloading video-like data.
- **Clients are never trusted.** The server re-validates every command and
  is the only thing that advances time.

### Rules for the core from day one
1. **Every state change is a command.** The UI never mutates world state.
   Commands are serialisable, versioned and carry the acting manager.
2. **Many humans, not one.** The world holds a list of human-controlled
   clubs. Nothing in the core assumes a single "the player".
3. **Queries take a viewer.** Every query is answered from a given club's
   point of view, because scouting knowledge differs per club. This is
   needed for fog of war anyway, and is exactly what online play needs.
4. **No blocking prompts.** When the sim needs a human decision (a bid, a
   contract demand), it creates a pending decision with a deadline and an
   AI default. Single-player pauses on it; online play lets it time out.
5. **Time advances by request.** Single-player advances when you press
   continue. Online advances when all managers are ready or the deadline
   passes. The scheduler supports both through the same entry point.
6. **UI settings never affect the sim.** Anything that changes outcomes
   lives in world state, so the server and every client agree.

### Later work (M8)
- `fs-server` crate: headless host, league lobby, deadlines, accounts.
- Sync: clients receive command batches and world snapshots per matchday.
- Hosting: a small server is enough, since a matchday for a medium world
  costs about a second of CPU.

