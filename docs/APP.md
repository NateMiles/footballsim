# The App

How the user interface is built, and how to run it: in a browser, as an
installed web app on iPad, and as a native app on macOS and iPad.

## 1. Layers

```
app/src/screens, components      Svelte screens. Know only view models.
app/src/match                    Match playback and renderers.
app/src/state                    App state, routes, view loading.
app/src/api/client.ts            Typed client: one method per request.
app/src/api/transport.ts         Transport: carries JSON to a core.
  ├─ worker-transport.ts         → web worker → fs-wasm → fs-api
  └─ tauri-transport.ts          → Tauri `call` → fs-api (native)
crates/fs-api                    Protocol, view models, session.
crates/fs-world, fs-match, ...   The simulation.
```

Rules that keep it modular:

- **One boundary.** The UI talks to the core only through `fs-api`'s
  protocol: `Request` in, `Response` out, JSON tagged by `type`. The
  view models in `fs_api::dto` mirror core types rather than exposing
  them, and each is shaped for one screen. The core can change its
  internals freely; the UI only notices if the protocol changes.
- **Generated types.** `app/src/api/generated.ts` is generated from the
  Rust types by `ts-rs`, and a test fails if it is stale. After changing
  a request, response or view model:

  ```bash
  UPDATE_BINDINGS=1 cargo test -p fs-api --test bindings
  ```

  Bump `PROTOCOL_VERSION` when a change breaks old clients; the client
  checks it at start-up.
- **Transports are interchangeable.** The web worker, the Tauri shell
  and (later) an online league server each implement `Transport`.
  Nothing above the client knows which one is in use.
- **Queries take a viewer.** Every view is built from one manager's side
  (`fs_api::query`), ready for per-club scouting knowledge and online
  play.
- **Renderers are interchangeable.** The match screen draws through
  `MatchRenderer` (`app/src/match/renderer.ts`). `Canvas2DRenderer` is
  the first implementation; a WebGL or 3D renderer implements the same
  four methods.
- **The world changes only through commands.** The session turns
  requests into `fs_world::Command`s submitted through `Game::submit`, so
  the command log stays complete.

## 2. A watched match

1. `Continue` advances day by day. On a match day for a manager who
   watches matches, the world stops before kick-off (`MatchDayReady`).
2. `StartMatch` builds the match exactly as the world will play it
   (`matchday::fixture_input`) and starts a `LiveSession` in the core.
3. The match screen asks for chunks ahead of its clock (`LiveTick`),
   interpolates frames, and reveals commentary and goals as the clock
   reaches them. Instructions (`LiveInstruct`) are applied at the core's
   current step, a few match seconds after the moment on screen; the
   screen pauses while the Team panel is open.
4. `FinishMatch` plays out anything unseen and submits `PlayMatch` with
   the instructions given. The world replays the match with them, gets
   the same result, and plays the rest of the day's fixtures.

Cup ties work the same way. A knockout match that is level at full time
plays extra time (the clock runs to 120, and an `interval` commentary
line marks each break) and, if still level, a penalty shootout: its kicks
arrive as `shootout` commentary lines a second apart after the final
whistle, and the scoreboard shows the penalties.

While a match is being watched, the session refuses changes to the team
(tactics, lineup, contracts), so the world's replay cannot differ.
Leaving the match screen and coming back restarts the match from
kick-off. `SkipMatch` lets the assistant manage it instead.

## 3. Saves

A save is an `fs-save` container: a JSON header (what a load screen
shows, versions, and per-section codec, sizes and checksums) followed by
deflated sections for the world snapshot, the command log and the
session's own state (viewer and inbox). The UI carries it base64-encoded. The browser keeps saves in IndexedDB
(`app/src/storage/saves.ts`, behind a `SaveStore` interface). The game
autosaves after starting a career and after every match. The Saves screen
can export a save as a file and import it again, which is also how to move
a game between devices.

Screens for competitions: Fixtures shows the manager's matches in every
competition (or a whole league), Cups shows any cup stage by stage
(knockout ties with aggregates, group tables, byes, past winners), and
Competitions lists every nation's leagues and cups and each continent's
coefficient table. Home shows how the club is doing in each cup; the
club page lists honours.

## 4. Running it

You need Rust (the pinned version installs itself through `rustup`),
Node 22, and `wasm-bindgen-cli` at the version pinned in `Cargo.toml`:

```bash
cargo install wasm-bindgen-cli --version 0.2.129 --locked
cd app
npm ci
npm run wasm     # build the core for the browser into src/wasm/pkg
npm run dev      # http://localhost:5173
```

Other scripts: `npm run check` (types), `npm test` (unit tests),
`npm run build` (production build in `dist/`), `npm run e2e` (Playwright
career test on iPad and desktop viewports; run `npm run build` first).
Rerun `npm run wasm` after changing Rust code.

## 5. On iPad as a web app

Any static web host serves `app/dist`. Over HTTPS, Safari can install it:
open the page, tap Share, then **Add to Home Screen**. It then opens full
screen, and the service worker lets it start without a connection.

For a quick try on your home network, without installing:

```bash
cd app && npm run build && npx vite preview --host
```

Open `http://<your-mac's-ip>:4173` on the iPad. Service workers need
HTTPS, so this version will not work offline.

**Measuring performance on the iPad.** The title screen ("Test this
device's performance") and the Saves screen run a performance test: they
generate a small or medium test world beside any game, play 90 days of
its season, and show the time per day, a projected season, the core's
memory and the threads it can use (one in a browser). The protocol behind
it is `BenchmarkStart`, `BenchmarkRun` and `BenchmarkEnd`; the timing is
measured in the app. Compare the numbers with the budgets in
`docs/ARCHITECTURE.md` section 6.

## 6. Native on macOS and iPad (Tauri)

The shell in `app/src-tauri` runs the same UI in the system web view and
the core natively, with real threads. It is its own Cargo workspace,
because it needs Apple's (or Linux's) web view libraries.

### Setup (once, on the Mac)

1. Install Xcode from the App Store, open it once, and accept the
   licence.
2. Install CocoaPods: `brew install cocoapods`.
3. Add the iOS targets to Rust:
   `rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios`
4. Do the steps in section 4 (`npm ci`; `npm run wasm` is not needed for
   the native app but does no harm).

### macOS

```bash
cd app
npm run tauri dev      # runs the app with live reload
npm run tauri build    # app bundle and .dmg in src-tauri/target/release/bundle
```

### iPad

1. In Xcode, open **Settings → Accounts** and sign in with your Apple ID.
   A free account works; installs then last seven days.
2. Create the Xcode project (once; it lands in `src-tauri/gen/apple`
   and can be committed):

   ```bash
   cd app
   npm run tauri ios init
   ```

3. Tell Tauri which signing team to use. Find your team ID in Xcode's
   Accounts pane (or in the project's **Signing & Capabilities** tab),
   then either export it:

   ```bash
   export APPLE_DEVELOPMENT_TEAM=XXXXXXXXXX
   ```

   or add `"developmentTeam": "XXXXXXXXXX"` under `bundle.iOS` in
   `src-tauri/tauri.conf.json`.
4. Connect the iPad by cable, trust the Mac, and turn on **Developer
   Mode** on the iPad (Settings → Privacy & Security), then restart it
   when asked.
5. Run it:

   ```bash
   npm run tauri ios dev          # pick the iPad (or a simulator) when asked
   ```

   During `ios dev` the iPad loads the UI from the Mac over your
   network, so keep them on the same Wi-Fi. For a build that runs on its
   own, open the Xcode project with `npm run tauri ios build -- --open`
   (or open `src-tauri/gen/apple/*.xcodeproj`), choose the iPad as the
   destination and press Run.
6. The first time, the iPad may refuse to open the app: in Settings →
   General → VPN & Device Management, trust your developer certificate.

If a step behaves differently, Tauri's own guides are the reference:
<https://v2.tauri.app/start/prerequisites/> and
<https://v2.tauri.app/develop/>.

## 7. Tests

| What | Where | Run |
| --- | --- | --- |
| Protocol end to end (new game, screens, live match, saves, performance test) | `crates/fs-api/tests/session.rs` | `cargo test -p fs-api` |
| Cups through the protocol (views, schedule, a watched cup tie) | `crates/fs-api/tests/cups.rs` | `cargo test -p fs-api` |
| TypeScript bindings current | `crates/fs-api/tests/bindings.rs` | `cargo test -p fs-api` |
| Watched match days in the world | `crates/fs-gen/tests/live.rs` | `cargo test -p fs-gen` |
| Client, frame timeline | `app/src/**/*.test.ts` | `npm test` |
| A career in the browser, with screenshots | `app/e2e/career.spec.ts` | `npm run e2e` |

CI runs all of them, and also checks that the native shell compiles on
Linux.
