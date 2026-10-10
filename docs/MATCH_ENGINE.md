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

| Tier | Used for | Model | Target cost |
| --- | --- | --- | --- |
| **0: Spatial** | The match you watch, replays, tactic lab deep runs | 22 agents + ball on a 2D pitch, 10 Hz decisions, ball physics with height | < 50 ms per 90 min when skipped |
| **1: Event** | Other matches in followed leagues, match predictor | Possession chains through pitch zones; each action resolved from attribute contests | < 1 ms |
| **2: Statistical** | Distant leagues, long-range projections | Team strength ratings to expected goals, goals and key events drawn from distributions | < 10 µs |

Tiers 1 and 2 are **calibrated against Tier 0**: run Tier 0 on many matchups,
then fit Tier 1 and 2 parameters so their outcome distributions match. Tier 0
is calibrated against real-world football statistics. This keeps the whole
world consistent no matter which tier played a given match.

## 2. Tier 0: spatial simulation

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

## 3. Tier 1: event simulation

- The pitch is split into a grid of zones (for example 12 x 8).
- A possession is a chain of actions: each step picks an action from
  tactic-weighted probabilities, then resolves it with an attribute contest
  between the involved players (sampled by role and zone).
- Shots resolve through the same xG and keeper models as Tier 0.
- Produces believable event logs (passes, shots, fouls, cards, subs,
  injuries) and per-player stats, so commentary and match reports still work.

## 4. Tier 2: statistical simulation

- Each team gets attacking and defensive strength from its lineup, tactic
  fit, fitness and morale.
- Expected goals for each side come from those strengths plus home advantage
  and conditions. Goals are drawn from a distribution fitted to Tier 0 output
  (a bivariate Poisson-family model handles the draw rate correctly).
- Scorers, assisters, cards and injuries are drawn by role-weighted
  probabilities so player season stats still look right.

## 5. Calibration targets

Starting targets, taken from top-flight European league averages. The
calibration suite in CI asserts each tier stays within tolerance.

| Statistic | Target range |
| --- | --- |
| Goals per match | 2.5 to 3.0 |
| Home win / draw / away win | about 45% / 25% / 30% |
| Shots per team per match | 11 to 14 |
| Shots on target ratio | 30% to 36% |
| Pass completion | 78% to 85% |
| Fouls per match | 20 to 26 |
| Yellow cards per match | 3 to 5 |
| Possession spread for strong vs weak team | up to about 70/30 |
| Final table points, champion | 80 to 95 in a 38-game season |

Open event data such as StatsBomb Open Data can refine these and supply
spatial targets (shot location maps, pass length distributions) for Tier 0.

## 6. Build order

1. **Tier 2** first (days of work). It unblocks the world simulation in M2.
2. **Tier 1** next. It gives believable event logs and a real use of
   tactics and attributes.
3. **Tier 0** last, alongside the 2D match view in M3, then iterate on
   realism for as long as the game exists.
4. Calibration harness is built with Tier 2 and grows with each tier.

## 7. Renderer-neutral output (3D readiness)

The match view is 2D for now. A 3D view should be possible later without
touching the engine, so the engine never draws anything. It emits a
**frame stream** that any renderer can consume.

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

