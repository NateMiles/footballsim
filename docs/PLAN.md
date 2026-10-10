# Game Plan

## 1. Vision

A football management sim that is faster, deeper and more honest than Football
Manager (FM).

- **Faster.** A full matchday across every active league resolves in about a
  second. A whole season in "holiday mode" takes under a minute, even on an
  iPad. Speed is a feature: it is what makes the Monte Carlo and "what if"
  tools in section 3 possible.
- **Deeper.** More systems are simulated explicitly instead of being faked
  with dice rolls: player physiology, scouting uncertainty, a real transfer
  market, club finances, staff and boardroom politics.
- **More honest.** The game shows you the numbers. Expected goals, pass
  networks and outcome probabilities are first-class UI, not hidden behind a
  "match engine is a black box" curtain.

## 2. Pillars

1. **Simulation first, UI second.** The simulation is a headless library that
   can run a whole world with no UI attached. Every UI screen is a view over
   that library.
2. **Deterministic.** Same save + same seed = same result, on every platform.
   This gives replays, reproducible bug reports, regression tests and fair
   online leagues for free.
3. **Level of detail (LOD).** Matches and clubs the player cares about are
   simulated in full detail. Distant ones use cheaper models calibrated to
   produce the same statistical outcomes.
4. **Data-driven and moddable.** Leagues, rules, competitions, attributes and
   names are data files, not code.
5. **Portable.** One codebase for desktop, browser and iPad.

## 3. Differentiators vs Football Manager

These are the reasons to play this instead of FM. Items marked **(speed)**
only exist because the engine is fast.

### Information and analysis
- **Match predictor (speed).** Before kickoff, run the fixture thousands of
  times with your chosen tactic and see the win/draw/loss distribution. Swap
  the tactic and compare.
- **Season projections (speed).** Monte Carlo the rest of the season to show
  title, relegation and European-place odds, updated every matchday.
- **Tactic lab (speed).** Batch-test a tactic against a range of opponent
  styles overnight in seconds, with a report of where it leaks chances.
- **Real analytics.** xG, xA, expected threat (xT), PPDA, field tilt, pass
  networks, shot maps, pressure heatmaps, per-player on/off splits. All
  exportable as CSV.
- **Scouting fog of war.** Attributes are true hidden values. What you see is
  an estimate with a confidence range that narrows as you scout more, with
  better scouts, or by playing against the player. Two clubs can genuinely
  disagree about a player.

### Depth
- **Physiology model.** Fitness, fatigue, sharpness and injury risk are driven
  by load (minutes, sprints, travel, recovery days), not a single condition
  bar. Rotation actually matters.
- **Agent-based transfer market.** Every AI club has a budget, needs, a
  scouting picture and a negotiating style. Prices emerge from supply and
  demand. Agents and player wage expectations respond to market heat.
- **Club economics.** Matchday, broadcast, commercial and prize income;
  amortisation; wage-to-revenue; financial fair play style rules defined per
  league in data.
- **Multiple career roles.** Manager, director of football, or owner. As owner
  you hire the manager and set the budget; as director you run recruitment
  and the AI picks the team.
- **Staff with opinions.** Coaches and analysts have their own tactical
  preferences, and their advice is visibly biased by them.
- **Men's and women's football** in the same world, sharing systems.

### Openness and replayability
- **Scenario editor.** Start from any state: "save a relegated club with
  £0 budget", "Leicester 2015", or your own fiction.
- **Modding.** Data packs for leagues, rules, names, kits (text and images,
  no code). Later, a sandboxed scripting layer for custom events.
- **Deterministic replays.** Any match can be re-watched exactly, or replayed
  from any moment with a substitution changed ("what if I'd made that sub
  at 60'?").
- **Online leagues** (later). Determinism means only manager decisions need
  to be synced, not the simulation.

## 4. Data and licensing

FM's player database is copyrighted research by Sports Interactive, and real
player names, club badges and league marks need licences. The plan is:

- Ship a **procedurally generated fictional world** (names, clubs, leagues,
  histories) that is good enough to play on its own.
- Support **community data packs** through the modding format, so players can
  load their own data. We never ship or scrape FM's database.
- Use public, openly licensed event data (for example StatsBomb Open Data)
  only to **calibrate** the match engine's statistical output, not as game
  content.

## 5. Scope by milestone

Each milestone ends in something playable or measurable. Rough sizes assume
one developer working with AI assistance; they are for sequencing, not
promises.

### M0: Foundations
- Cargo workspace, CI (fmt, clippy, tests), benchmark harness.
- Core ID and entity types, seeded RNG, determinism test (same seed gives a
  byte-identical world hash on native and WASM).
- Command log from day one: every manager decision is a serialisable,
  timestamped command. This is the foundation for replays, saves and future
  online leagues (see ARCHITECTURE.md section 11).

### M1: Headless match engine
- Tier 1 event engine and Tier 2 statistical engine (see MATCH_ENGINE.md).
- CLI: `sim-match --home A --away B --runs 10000` prints result distributions.
- Calibration report comparing output stats against real-football targets.

### M2: One-league world
- Fictional world generator: one nation, 20 clubs, about 600 players.
- Season loop: calendar, fixtures, league table, training, injuries,
  contracts expiring, basic AI squad selection.
- CLI: simulate ten seasons and print tables and player development.

### M3: Playable web UI
- TypeScript UI: inbox, squad, tactics, fixtures, table, player profile.
- Match day: 2D top-down match view (Tier 0 engine), live stats, subs and
  shouts. The view renders from a renderer-neutral frame stream so a 3D
  view can be added later without engine changes (see MATCH_ENGINE.md
  section 7).
- Core compiled to WASM in a Web Worker. Runs in desktop browsers **and
  iPad Safari as an installable PWA**.
- Tauri dev builds for macOS and your own iPad, built from your Mac with
  Xcode. This is the first time it runs natively on the iPad.

### M4: Management depth
- Transfers and loans with the agent-based market, contracts, wages, agents.
- Scouting fog of war, scouting assignments, staff hiring.
- Finances, board expectations, sackings, job market.
- Youth intake and development curves.
- Save and load.

### M5: Scale
- Multi-nation world: 20+ nations, 60+ leagues, cups and continental
  competitions defined in data.
- LOD scheduling across the world, multithreaded daily tick.
- Hit the performance budgets in ARCHITECTURE.md on an iPad.

### M6: Native polish and distribution
- Release-quality Tauri 2 builds for macOS, Windows and iPadOS, with the
  core running natively on all cores.
- iPad-first touch layout pass on every screen.
- TestFlight builds for your own devices. App Store and Steam readiness
  checks if a release is on the table by then.

### M7: Beyond FM
- Match predictor, season projections, tactic lab.
- Replays with branching "what if" from any moment.
- Scenario editor and data-pack modding.
- Director of football and owner careers.

### M8+: Later
- **Online leagues.** Already designed for: see ARCHITECTURE.md section 11.
- **3D match view.** Already designed for: see MATCH_ENGINE.md section 7.
- Scripting for mods, women's football world data, more nations.

## 6. Biggest risks

| Risk | Why it matters | Mitigation |
| --- | --- | --- |
| Match engine realism | It is the game. A fast but silly engine is worthless. | Calibrate against real stats from M1 onward; keep a regression suite of stat targets. |
| UI scope | FM has hundreds of screens. | Build screens on a shared table/filter/card kit; ship fewer, denser screens. |
| iPad memory | iPadOS kills apps that use too much memory; Safari limits WASM memory further. | Compact data layout and memory budgets from day one (see ARCHITECTURE.md). |
| Cross-platform determinism | Float differences break replays and online play. | Avoid platform maths functions in the sim, test native vs WASM hashes in CI. |
| Licensing | Real names and data are a legal problem. | Fictional world plus user-loaded data packs. |
| Scope creep | "More than FM" is unbounded. | Milestones above are strict; M7 features wait until M5 performance is proven. |

## 7. Decisions

| Question | Decision | What it changes |
| --- | --- | --- |
| Personal or release? | Personal for now, possible release later. | Build for fun and speed first, but never take shortcuts that block a release: keep the world fictional, keep third-party assets properly licensed, keep saves versioned. |
| Online leagues? | Not now, but keep the door open. | No online code until M8. Determinism and the command log from M0 make it a later add-on instead of a rewrite. |
| Match view | 2D now, 3D possible later. | The engine already simulates ball height. The renderer reads a neutral frame stream, so a 3D renderer can plug in alongside the 2D one. |
| Mac available? | Yes. | Native iPad dev builds move up from M6 to M3. M6 becomes polish and distribution. |
