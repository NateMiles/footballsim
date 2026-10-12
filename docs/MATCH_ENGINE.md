# Match Engine

The match engine is the heart of the game and the hardest part to get right.
It must be **believable** when watched, **statistically realistic** in
aggregate, and **fast** enough to run whole worlds and Monte Carlo
predictions.

## 1. Three fidelity tiers

All three tiers read the same inputs (two lineups, tactics, player
attributes, fitness, conditions, seed) and produce the same output shape
(score, events, per-player stats, ratings). They differ in how much they
simulate.

| Tier | Used for | Model | Target cost | Status |
| --- | --- | --- | --- | --- |
| **0: Spatial** | The match you watch, replays, tactic lab deep runs | 22 agents + ball on a 2D pitch, 10 Hz decisions, ball physics with height | < 50 ms per 90 min when skipped | Planned (M3) |
| **1: Event** | Other matches in followed leagues, match predictor | Possession chains through pitch zones; each action resolved from attribute contests | < 1 ms | Built: 0.31 ms |
| **2: Statistical** | Distant leagues, long-range projections | Team strength ratings to expected goals, goals and key events drawn from distributions | < 10 µs | Built: 8.7 µs |

Timings are single-threaded on the 4-core cloud build machine.

### How the tiers stay consistent

1. **Tier 1 is calibrated against real football.** Its constants
   (`tier1::Params`) are tuned until a season simulation matches
   top-flight averages and no tactic slider is a free win (section 5).
2. **Tier 2 is fitted to tier 1.** `fs-cli fit-tier2` simulates 40 tier 1
   seasons and fits tier 2's goal model by Poisson regression, writing the
   result to `crates/fs-match/src/fitted.rs`. CI regenerates it and fails
   if it is stale.
3. **When tier 0 arrives,** it becomes the reference: tier 0 is calibrated
   against real football, tier 1 is re-tuned to match tier 0, and tier 2 is
   refitted to tier 1.

## 2. Tier 0: spatial simulation (planned)

### World
- Pitch 105 x 68 m. Positions and velocities in metres and m/s.
- Ball has 3D position, spin-free drag, bounce and roll friction. Ball
  physics steps at 50 Hz; agent decisions at 10 Hz.
- Each player has physical limits from attributes: top speed, acceleration,
  turning, jump reach, and an energy budget that drains with sprints.

### Team shape
- A tactic defines, per role, an **anchor position** for each phase of play
  (in possession, out of possession, transitions), expressed relative to the
  ball's position and the team's defensive line height and width.
- Off-ball players steer toward their anchor, adjusted by role behaviour:
  overlaps, underlaps, runs in behind, dropping deep, man-marking, pressing
  triggers.

### Decisions (ball carrier and nearby players)
A utility-based AI scores the available actions each decision tick:

- Pass to each teammate (ground, lofted, through ball), weighted by
  interception risk from defender positions and the expected threat (xT) gain
  at the target.
- Dribble or carry into space.
- Shoot, scored by an xG model of the shot location and pressure.
- Hold, shield, or win a foul.
- Cross, cut back, switch play.

Attributes shape both **perception** (decision-making and vision limit which
options a player notices, and how accurately they judge risk) and
**execution** (passing, technique and first touch add noise to the outcome).
Tactical instructions shift utility weights: "play out from the back" lowers
the value of long balls, "shoot on sight" raises shot utility.

Defenders choose to press, cover, mark or hold the line using the same
utility approach, with pressing intensity and triggers set by tactics.

### Contests
Tackles, aerial duels, shots against the keeper, and 50-50s are resolved
with probability models from the attributes involved, the geometry (angle,
distance, body orientation) and fatigue.

### Set pieces
Corners, free kicks and throw-ins use routine templates from tactics with
role assignments, then hand back to open play.

### Performance notes
- Only the ball carrier and a few nearby players run the full option scan
  each tick. Everyone else runs cheap steering.
- Spatial queries use a coarse grid over the pitch; with 22 agents, brute
  force is often faster than fancier structures. Benchmark both.
- Fixed-size arrays, no heap allocation inside the match loop.
- Many matches run in parallel, one per thread, each with its own RNG stream.

## 3. Tier 1: event simulation (built)

Code: `crates/fs-match/src/tier1.rs`.

### Pitch and possession
- Six bands from the team in possession's own goal (band 5 is the
  opponent's box) by three lanes. Turnovers mirror the zone for the other
  side.
- Each formation slot has a typical location in the attacking shape.
  Players near a zone are more often the ball carrier, receiver or
  defender there (Gaussian weights per zone, rebuilt after every lineup
  change).

### Actions
The carrier picks short pass, long ball, through ball, cross, dribble or
shot, weighted by band, lane, tactic and their own skills. Each action is
a logistic contest between the carrier's skill and a nearby defender's:

- **Short pass.** Direction is chosen first; forward passes gain a band but
  fail more often. Failures are loose balls, interceptions or throw-ins.
- **Long ball.** Skips two bands; contested in the air.
- **Through ball.** Played to a runner; success is a big chance (about 0.3
  xG before skill). Can be caught offside.
- **Cross.** From wide areas; success is a header at goal.
- **Dribble.** Beats a defender to advance a band, or in the box raises
  the next shot's quality. Failed dribbles are tackles or fouls.
- **Shot.** xG comes from the zone and shot type, scaled by the shooter's
  skill. The keeper's skill relative to an average top-flight keeper sets
  the conversion. Shots can be blocked, saved (sometimes for a corner) or
  missed.

### Set pieces, discipline, injuries
Fouls happen in contests and away from the ball, scaled by the defender's
aggression and dirtiness. Cards depend on temperament and danger; booked
players are cautious. Fouls in the box can be penalties; central fouls
outside it can be direct free kicks. Corners produce headers, short
corners or clearances. Injuries are rare, more likely for injury-prone
and tired players, and force a substitution.

### Fatigue and substitutions
Energy drains each minute by stamina, pressing and tempo, and faster for
the side without the ball. Skills scale with energy. The AI makes up to
five substitutions in three windows (60', 70', 80'), replacing the
tiredest outfielders with the best bench fit.

### Game state
Trailing sides raise their mentality from 60' and again from 80'.
Comfortable leaders sit back. An attacking mentality creates more chances
but leaves space behind, which the opponent exploits near goal. This
trade-off is what keeps mentality balanced.

### Roles, duties and opposition instructions
A team sheet carries `Instructions` (`roles.rs`) beside the tactic: a role
and duty per formation slot, set-piece takers, and up to three
instructions on opposition players for one match. Every slot has a
standard role (central defender, full-back, wide midfielder, striker and
so on) which with a support duty changes nothing, so AI teams and the
calibration play exactly as before.

Other roles and duties have an `Effect`: multipliers on the carrier's
action weights (a winger crosses and dribbles more, a playmaker looks for
through balls, a poacher shoots), a shift of the player's attacking
position forward or back, a pull inside in the final third, more or less
presence in the box, and more or less defending. The engine compares each
zone's attacking presence and defensive coverage with what standard roles
would give, and adds `presence_effect × (presence - 1)` for the side in
possession and `cover_effect × (coverage - 1)` against it to every pass,
cross and dribble contest. Pushing players on therefore creates more at
one end and costs cover at the other.

Chosen set-piece takers take penalties, free kicks and corners while on
the pitch; otherwise the best on the pitch does. A tightly marked player
gets the ball less often, wins fewer contests and gets worse chances, but
draws more fouls and every other carrier gains a little as the shape
stretches. Closing a player down hurries them but tires the team.

### Knockout ties
A cup match that must produce a winner carries a `Decider`: the goals
each side scored in earlier legs, and whether extra time is played. In a
second leg, game state reacts to the aggregate score rather than the
match score. If the tie is level at full time, two 15-minute halves of
extra time follow (ends swap each half, players recover a little, and
each side gets one more substitution in one more window); if it is still
level, a penalty shootout (`shootout.rs`) settles it: the best takers go
first, keepers last, best of five and then sudden death. A kick's chance
uses the same taker and keeper skills as an in-match penalty, a little
lower under pressure. Matches without a decider play exactly as before,
so the calibration is unaffected.

## 4. Tier 2: statistical simulation (built)

Code: `crates/fs-match/src/tier2.rs`, constants in `fitted.rs`.

- Each side's expected goals are `exp(beta . features)`. The features
  are: home, attack against opposing defence, control difference,
  finishing against opposing keeper, and both teams' six tactic sliders.
- The score is drawn from two Poissons with a small Dixon-Coles
  correction for low-score draws.
- Shots, xG, passes, possession, fouls, cards, corners, scorers,
  assisters, substitutions and ratings are drawn so season totals match
  tier 1.
- A level knockout tie gets extra time at 30% of the usual goal rate per
  minute, then the shared penalty shootout. Both use a second random
  stream, so the first 90 minutes are the same with or without a
  decider.

**Known limitation.** Tier 2 matches tier 1 season by season (all checks in
`docs/CALIBRATION.md` pass), but individual fixtures can differ. The mean
gap in win/draw/loss probability is about 2 points, and the worst of the
ten reference fixtures differs by about 7. Tier 2's few team summaries
cannot see every interaction tier 1 simulates. This is acceptable for
distant leagues. The match predictor uses tier 1 for that reason.

## 5. Calibration (done)

`scripts/calibration-report.sh` writes `docs/CALIBRATION.md`. Each tier
plays 40 fresh 20-club seasons on seeds not used for tuning, and each
statistic is checked against a target range:

| Statistic | Target range |
| --- | --- |
| Goals per match | 2.5 to 3.0 |
| Home win / draw / away win % | 42-48 / 22-28 / 27-33 |
| 0-0 % | 5 to 10 |
| Shots per team | 11 to 14 |
| Shots on target % | 30 to 36 |
| xG per shot; goals per xG | 0.09 to 0.12; 0.92 to 1.08 |
| Passes per team; completion % | 400 to 520; 78 to 85 |
| Fouls; yellows; reds per match | 20 to 26; 3 to 5; 0.05 to 0.2 |
| Corners per team | 4 to 6 |
| Injuries per match (time-loss) | 0.4 to 0.9 |
| 95th percentile possession % | 62 to 72 |
| Champion points; bottom club points | 80 to 95; 15 to 32 |
| Top scorer's league goals | 22 to 36 |
| Season points per step of each tactic slider | -4 to +4 |
| Season points from each role, duty and opposition instruction | -4 to +4 |

The tactic rows matter as much as the realism rows. They caught that
direct long-ball play was worth about 13 points a season, a dominant
strategy, until passing risk, chasing fatigue and mentality exposure were
modelled.

Roles, duties and opposition instructions are measured by
`fs-cli choices`: every fixture of ten calibration seasons is replayed
with the same random stream with the choice made by the home side, then
by the away side, and the points gained over the standard setup are
scaled to a season. All are within about 2.5 points either way.

Tuning tool: `cargo run --release -p fs-lab --example tune -- key=value`.

Remaining known gaps:
- Crosses complete about 13% of the time, against about 22% in reality.
  Headers from crosses are tuned for shot volume instead.
- Tackles per team are about half the real count. This is a stat-labelling
  gap; the contests happen but most are recorded as interceptions or loose
  balls.
- Draw rate sits in the lower half of its range, at 22-24%.

Added in M2: the top-scorer check (strikers took too large a share until
midfielders were allowed to arrive in the box), the injuries check (the
first rate was about five times too low), and directness now affects only
long balls, not the share of forward short passes.

## 6. Build order

1. ~~Tier 2~~ and ~~Tier 1~~, with the calibration harness: done in M1.
2. ~~The 2D match view~~ in M3, driven by tier 1 (see section 7). Tier 0
   was deferred: a watched match must be the match the world records, and
   tier 1 is the calibrated engine. Building tier 0 first would have meant
   either showing an uncalibrated engine or re-tuning everything before
   anyone could play.
3. **Tier 0** later, then re-tune tier 1 to it. The frame stream, action
   log and live-instruction interfaces are engine-neutral, so the match
   screen will not change.
4. Keep iterating on realism for as long as the game exists, with the
   calibration report as the guard rail.

## 7. Renderer-neutral output (3D readiness)

The match view is 2D for now. A 3D view should be possible later without
touching the engine, so the engine never draws anything. It emits a
**frame stream** that any renderer can consume.

### What is built (M3)
- Tier 1 can be stepped (`tier1::LiveMatch`) and records an **action log**
  (`fs_match::log::ActionRecord`: who did what, from which zone to which,
  with what outcome and timing). Stepping and batch play give identical
  results.
- A **director** (`fs_match::view::Director`) turns the action log into
  frames at 5 Hz: formation anchors shifted with the ball, the actors
  moving to the play, ball arcs with height, celebrations and kick-offs.
  `view::LiveView` combines engine and director.
- **Instructions** (substitutions, tactic changes, touchline shouts) are
  keyed to the engine step at which they were given
  (`fs_match::TimedInstruction`). Replaying a match input with the same
  instructions reproduces the watched match exactly, which is how the
  world records it (see WORLD.md section 5).
- Commentary is generated from the action log and match events in
  `fs-api` (`commentary.rs`), so any future engine that emits the log
  gets it for free.

The frame contents below are the target; the current frames carry
position, facing and pose for each player and the ball's position with
height. Velocities, action timing and quantisation arrive with tier 0
and replays.

### Post-match analysis
`analysis.rs` turns an action log into a `MatchAnalysis`: every shot
(minute, taker, kind, zone, xG, outcome), each side's actions by zone,
average positions and completed passes between players, and possession
in 15-minute spells. `tier1::play_recorded` plays a match exactly as
`play` does and keeps the log. The world records human-managed clubs'
tier 1 matches this way (`fs_world::records`), and the app draws the
shot map, xG timeline, pass networks and zone maps from them.

### Frame stream contents
- **Header:** pitch dimensions, team colours and kit IDs, player IDs, shirt
  numbers, heights.
- **Frames** at a fixed rate (for example 10 Hz), each with:
  - per player: position (x, y), velocity, body facing, and an **action
    state** such as running, passing, shooting, tackling, heading, diving,
    celebrating, injured;
  - ball: position (x, y, z) and velocity;
  - the action's start time and target, so a 3D renderer can time
    animations.
- **Events:** goals, fouls, cards, subs, set-piece starts, with frame
  indices.
- Values are quantised (centimetres, small integer angles) so a full match
  stays compact in memory and in replays.

### How each renderer uses it
- **2D (now).** Interpolates positions between frames, draws ball height as
  a shadow offset and size change, and uses action state for simple icons
  or highlights.
- **3D (later).** Uses the same interpolation, maps action states to
  animations, and adds cameras. The likely options are three.js or
  Babylon.js inside the existing web UI, which also runs in the iPad
  WebView. A native engine view is possible but not planned.

### UI contract
The UI defines one renderer interface: load the header, push frames, seek,
and set playback speed. The 2D renderer is the first implementation; a 3D
renderer is a second implementation of the same interface, selectable in
settings.

### Engine requirements this creates
The engine must track body facing and action timing, and must simulate the
ball in 3D. Tier 0 already needs all three for realism, so the cost is close
to zero.

