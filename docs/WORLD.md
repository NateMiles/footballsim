# World Simulation

How the world moves from day to day and season to season. Code lives in
`crates/fs-world`; the generator that builds new worlds is `crates/fs-gen`.

## 1. The calendar

Seasons run from 1 July to 30 June.

| Date | What happens | Code |
| --- | --- | --- |
| 1 July | Season rollover (section 4) | `season::rollover` |
| Early August to late May | League matchdays, mostly Saturdays | `matchday::play_today` |
| 20 August, 10 January | Transfer-window squad housekeeping for AI clubs | `squad::maintain_squads` |
| 1 January | Human managers warned about contracts ending in June | `squad::warn_expiring` |
| 1 June | AI clubs renew contracts worth keeping; humans warned again | `squad::renew_contracts` |
| 1st of every month | Player development; AI tactics refreshed | `tick::development` |
| Every day | Recovery, injury healing, training injuries | `tick::afternoon` |

Each day runs in two parts: the morning's calendar business
(`tick::morning`), then matches, recovery and development
(`tick::afternoon`). A watched match day pauses between them (section 5).

Fixtures are a double round robin with the club order shuffled each
season. Rounds are spread evenly over the Saturdays from the second
weekend of August to late May; Wednesday rounds are added only for leagues
too large to fit.

## 2. Leagues

- Each nation's clubs are ranked by reputation and split into divisions of
  20 (`GenConfig::clubs_per_league`).
- Three clubs go up and three go down between adjacent divisions.
- A league whose clubs include a human-managed club plays its matches with
  the tier 1 event engine; every other league uses tier 2.

## 3. Players over time

**Matches.** Players who play lose fitness and gain fatigue and sharpness.
Results move the whole squad's morale; unused players get restless.
Injuries come from the match engine, with severities from minor (3-12
days) to serious (two to seven months). Injury-prone players stay out
longer. Red cards bring a one-match ban for a second yellow or three for a
straight red; every fifth yellow of the season brings a one-match ban.
Injured and suspended players are never picked.

**Training injuries** happen on any day, more often for injury-prone and
fatigued players.

**Development** happens monthly:
- Below their hidden potential, players grow at a steady rate that would
  take them from a 250 baseline to their potential in about eight years.
  Growth is fastest up to 21, tails off to 27, and is scaled by
  professionalism and playing time.
- Physical attributes decline from 30 and other attributes from 33.
  Mental attributes still creep up between 26 and 32.
- Each summer, every under-24's potential moves a little at random, and
  youngsters who barely played lose some.

**Academy.** Clubs take in two to four 15- or 16-year-olds each summer.
Players under 18 are academy players: they can be picked but do not count
toward squad size or positional needs. At 18-21, AI clubs release
youngsters whose potential falls well short of the first team.

**Contracts.** Every player at a club has a weekly wage and an end date
(always 30 June). Wage demands rise steeply with ability (and with
potential for under-23s). Each club has a wage cap set by its reputation.
Contracts that are not renewed end on 30 June and the player becomes a
free agent.

**Retirement.** It starts at 31 and is near-certain by 37. It comes sooner
for free agents, and anyone unattached for two summers retires.

## 4. The 1 July rollover

In order:

1. Archive every league season (final table, top scorer); apply promotion
   and relegation.
2. File each player's season stats in their career history.
3. Release players whose contracts ended.
4. Retirements.
5. Potential drift for young players.
6. Youth intakes.
7. AI squad housekeeping: academy cuts, trimming squads over 30 first-team
   players to 27, then signing free agents to cover positional needs
   (two goalkeepers, seven defenders, seven midfielders, four attackers, 24
   in all). The richest clubs choose first, within their wage caps.
8. AI tactics refreshed; new fixtures created.

## 5. Human managers

Human-managed clubs are left alone by the AI unless the manager delegates
squad management (`Command::SetDelegation`). Otherwise they must renew
contracts themselves (`OfferContract`), or players leave in June. Other
commands: `SetTactic`, `SetLineup`, `ReleasePlayer`, `SetSquadNumber`. A
contract offer is accepted when the wage is at least 90% of the player's
demand.

A new game appoints its human with `AppointHuman` (system only), which
replaces the club's AI manager. A chosen lineup is honoured slot by slot;
starters who are injured, suspended or gone are replaced by the most
suitable available player, and the bench is topped up.

**Watched match days.** A manager with `watch_matches` set (`SetWatchMatches`)
manages their matches live:

1. `Advance` stops after the morning of a day on which they play, with
   `Event::MatchDayReady`. No match anywhere is played yet
   (`World::pending`).
2. The host shows the match from `matchday::fixture_input`, using
   `fs_match::view::LiveView`. The manager's side has no automatic
   substitutions; their substitutions, tactic changes and shouts are
   recorded against engine steps.
3. `PlayMatch { instructions }` reports them. Once every watching manager
   has reported, the day's matches are played, the watched ones with
   their instructions, so the result is exactly the one watched.

While the day is pending, a club playing today can change its team only
if its own manager, alone, is still to watch the match; anything else
that would alter a match being shown is refused. `Advance` during a
pending day plays it first, with the assistant managing unwatched matches.
When two watching humans meet, each submits their own side's
instructions; showing both live together is left to online play.

## 6. Health targets

A world must stay believable for decades. `fs-cli career` prints health
lines each season; the `ten_seasons_keep_the_world_healthy` test enforces
the core of it. A 30-season run of the tiny world currently shows:

| Measure | Target | Observed |
| --- | --- | --- |
| First-team (18+) mean age | 25-27 | 25.6 to 26.6 throughout |
| First-team age mix (18-20 / 21-23 / 24-27 / 28-31 / 32+) | Stable | About 18 / 19 / 25 / 24 / 14 % |
| Top-division best-XI ability | Within 5% of the start | Within about 2% |
| Active players | Stable | About 700 for a 20-club world |
| Top-scorer goals | 22-36 | About 29 |

New worlds are generated with this steady-state age mix and with full
academies, so they do not lurch toward it over their first decade.

## 7. Performance

One season on the 4-core cloud build machine:

| World | Players | Season |
| --- | --- | --- |
| Tiny (1 league) | ~700 | 0.1 s |
| Small (120 leagues) | ~85,000 | ~3.7 s |
| Medium (300 leagues) | ~210,000 | ~9 s |

The 1 July rollover is the slowest day at about 1.7 s in the medium world.
`cargo run --release -p fs-lab --example profile_season` shows the
breakdown.
