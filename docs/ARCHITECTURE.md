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
|  UI (TypeScript, Svelte or React, Vite)                     |
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
    fs-core/                 # IDs, entities, attributes, RNG, time, shared types
    fs-match/                # match engine, all fidelity tiers
    fs-world/                # calendar, competitions, training, injuries,
                             # contracts, transfers, finances, youth
    fs-ai/                   # AI managers, directors, boards, agents
    fs-gen/                  # procedural world and name generation
    fs-data/                 # data-pack format, loading, validation
    fs-save/                 # save/load, versioned migrations
    fs-api/                  # command/query/event API used by every host
    fs-wasm/                 # wasm-bindgen wrapper around fs-api
    fs-cli/                  # headless runner: sim seasons, benchmarks, calibration
  app/                       # TypeScript UI
  app/src-tauri/             # Tauri 2 shell (desktop + iPadOS)
  data/                      # base data packs (rules, competitions, name pools)
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
| Small (PWA default) | ~60k | < 150 MB |
| Medium | ~150k | < 300 MB |
| Large (native only) | ~500k | < 800 MB |

## 5. Simulation scheduling and LOD

The world advances in **days**. Each day:

1. Process scheduled events (fixtures, transfer deadlines, board meetings).
2. Run matches for the day, in parallel, each at the fidelity tier its
   importance deserves.
3. Run per-entity daily updates (training, recovery, injuries, morale) as
   parallel column scans.
4. Run AI decisions on a staggered schedule (not every club thinks every day).
5. Emit events for the UI.

**Importance-based LOD.** Each club and competition gets an importance score:
the user's club and its league are highest; leagues the user scouts or follows
are medium; everything else is low. Importance chooses the match tier and how
often AI and development are updated in detail. Low-detail simulation is
calibrated to match the statistical output of high detail, so the world does
not drift.

## 6. Performance budgets

Targets on a recent iPad running the native build. The CI benchmark suite
tracks these on desktop as a proxy, with a scaling factor measured once on
hardware.

| Operation | Target |
| --- | --- |
| Tier 0 full spatial match (watched) | Real time or faster at any playback speed; full 90 min in < 50 ms when skipped |
| Tier 1 event-level match | < 1 ms |
| Tier 2 statistical match | < 10 µs |
| Daily world tick, medium world | < 50 ms |
| Full matchday, all leagues, medium world | < 1 s |
| One season in holiday mode, medium world | < 60 s |
| Match predictor, 10,000 runs at Tier 1 | < 2 s |
| Player search with filters over 150k players | < 20 ms |

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

- Save format: a versioned binary snapshot of the world (serde + a compact
  binary codec), compressed with zstd. Target under 50 MB for a medium world.
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
| WASM | wasm-bindgen, wasm-pack, Web Workers |
| UI | TypeScript, Vite, Svelte or React (decide at M3), PixiJS for the match view |
| Native shell | Tauri 2 (desktop + iOS) |
| Serialisation | serde, compact binary codec, zstd |
| Benchmarks | criterion |
| CI | GitHub Actions: fmt, clippy, tests, WASM build, determinism check, benchmarks |

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

