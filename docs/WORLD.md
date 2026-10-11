# World Simulation

How the world moves from day to day and season to season. Code lives in
`crates/fs-world`; the generator that builds new worlds is `crates/fs-gen`.

## 1. The calendar

Seasons run from 1 July to 30 June.

| Date | What happens | Code |
| --- | --- | --- |
| 1 July | Season rollover (section 4) | `season::rollover` |
| 1 July - 31 August, January | Transfer windows (section 9) | `market::daily` |
| Early August to late May | League matchdays, mostly Saturdays | `matchday::play_today` |
| 20 August, 10 January | Transfer-window squad housekeeping for AI clubs | `squad::maintain_squads` |
| 1 January | Human managers warned about contracts ending in June | `squad::warn_expiring` |
| 1 June | AI clubs renew contracts worth keeping; humans warned again | `squad::renew_contracts` |
| 1st of every month | Player development; AI tactics refreshed; sponsorship, television, wages and running costs; boards review the table and the bank | `tick::development`, `economy::monthly_accounts`, `board::monthly` |
| Every day | Recovery, injury healing, training injuries, scouting, completed deals, vacant jobs filled | `tick::afternoon` |
| After every match | Gate receipts; board confidence | `economy::gate_receipts`, `board::after_match` |

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

1. Prize money; accounts archived; boards judge the season (sackings),
   club and manager reputations move.
2. Archive every league season (final table, top scorer); apply promotion
   and relegation.
3. File each player's season stats in their career history.
4. Loans end; players go back to their clubs.
5. Release players whose contracts ended; staff contracts end, AI clubs
   renew or replace staff.
6. Retirements.
7. Potential drift for young players.
8. Youth intakes.
9. AI squad housekeeping: academy cuts, trimming squads over 30 first-team
   players to 27, then signing free agents to cover positional needs
   (two goalkeepers, seven defenders, seven midfielders, four attackers, 24
   in all). The richest clubs choose first, within their wage caps.
10. AI tactics refreshed; new fixtures created; boards set targets;
    facilities wear and boards invest spare money; budgets set.

## 5. Human managers

Human-managed clubs are left alone by the AI unless the manager delegates
squad management (`Command::SetDelegation`). Otherwise they must renew
contracts themselves (`OfferContract`), or players leave in June. Other
commands: `SetTactic`, `SetLineup`, `ReleasePlayer`, `SetSquadNumber`, the
market (`MakeOffer`, `RespondToOffer`, `AcceptCounter`, `WithdrawOffer`,
`SetListing`), scouting (`ScoutPlayer`, `CancelScouting`), staff
(`HireStaff`, `ReleaseStaff`) and jobs (`ApplyForJob`, `Resign`). A
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

## 6. Money

Every club has a balance, a transfer budget, a weekly wage budget, a
ledger for the season and ten seasons of archived accounts
(`finance.rs`); `economy.rs` moves the money.

**The model is anchored to the wage scale.** A club's *revenue
potential* is what it would need to pay a squad of its standing about 60%
of its income in wages: `3,900 x wage_scale(reputation)` a year. Income
is made of:

- gate receipts per home match (fans by reputation, capped by the
  stadium, a bigger crowd for a big visitor);
- television: 35% of the league's average potential, shared equally;
- prize money at the end of the season, by final position;
- sponsorship and merchandise: whatever the club's standing earns beyond
  the rest;
- player sales.

Spending is player and staff wages (monthly), transfer fees, running
costs (15% of potential) and investment in facilities.

**Budgets.** Each summer the board sets the wage budget at 66% of
potential, stretched a little by savings, and the transfer budget at half
of the money above three months' wages. Half of every sale goes back into
the transfer budget. AI clubs respect the wage budget when signing and
renewing; the best eleven may stretch it by a tenth, and stars the club
cannot afford leave on a free.

**Facilities** (stadium capacity, training ground and academy, 1-20)
wear: each rating has a one-in-five chance of dropping a point each
summer. Boards with more than half a season's potential to spare invest
in one improvement: a bigger stadium if it is full, otherwise the weaker
of the training ground and academy. Facilities matter relative to what a
club of that standing usually has, so the world's overall level holds.

**Safety net.** A club down to 16 players signs the cheapest free agents
it needs, whatever its budget, so every club can always field a team.

## 7. Staff

Staff are entities like players (`staff.rs`): assistant managers,
coaches, scouts and physios, each with six ratings from 1 to 20 and a
contract. A club's departments are rated from its best few in each role
(10 is average, a missing post counts as 4):

| Department | Effect |
| --- | --- |
| Coaching (best three coaches) | Growth of senior players: +-12% at the extremes; under-21s use youth coaching, +-18% |
| Training ground | Growth: +-6% |
| Scouting (best two scouts) | How much the club knows about players, and how wrong its estimates are |
| Medical (best two physios) | Training injury risk +-15%, time out injured +-25% |
| Academy facilities | Youth intake potential: about 8 points per rating point above what the club usually has |

Staff wages scale with the level of club that employs staff of that
quality, like player wages. Staff only join clubs near their own standing.
AI clubs renew staff worth keeping each summer and fill vacancies from
the free pool (generating newcomers when nobody suitable is free); human
managers hire, renew and release their own (`HireStaff`, `ReleaseStaff`,
three months' wages as severance).

## 8. Scouting

Attributes are true hidden values. Each club sees an estimate
(`scouting.rs`): the truth plus an error fixed for that club and player
(so numbers do not flicker), scaled by how little the club knows and by
how good its scouts are at judging. Ranges narrow as knowledge grows.

Knowledge (0-100) comes from proximity (40 for players in the club's
league, 22 in its nation, 8 elsewhere), plus up to 24 for a strong scouting
department, plus what the club learns directly: each match against a
player adds 15 (for human clubs), and scouting assignments add 2-8 a day
per scout. Each scout watches one player at a time; a report arrives when
knowledge reaches 95. AI clubs judge transfer targets by their own
estimates, so they overpay for some players and miss others, and two clubs
can disagree about the same player.

## 9. Transfers and loans

Windows run from 1 July to 31 August and through January (`transfer.rs`,
`market.rs`). A deal runs in steps, so nothing waits on a person:

1. A buyer bids: a fee and a contract for the player, or a loan to the
   end of the season with the borrower paying a share of the wages.
2. The selling club answers: AI clubs the next day, human managers within
   three days (no answer is a rejection). It accepts, refuses, or names
   its price, which the buyer can accept.
3. The player weighs personal terms: players want more money to step down
   to a smaller club and will not drop far unless they are surplus.
4. Agreed deals complete in the next daily update, even if the window has
   just closed.

**Valuation.** A player's value is a multiple of a year's wage demand: 4x
under 21, 3.5x to 24, 3x to 28, then falling to 0.5x past 32; a third of
that with under a year left on the contract. Selling clubs ask 1.6x for
their best eleven (more late in the window), 1.25x for squad players and
0.9x for players they have listed, and will not sell from a thin squad.

**AI clubs** list surplus players for sale and promising youngsters for
loan when a window opens. Each window day about one in eight looks for a
signing to fill a gap or replace a weak starter: listed players first,
then a sample of players of about the right level at clubs not far above
its own, judged by its own estimates, within budget and wage budget.
Clubs that cannot afford a fee borrow listed youngsters.

## 10. Boards and jobs

Each summer a board sets a target position from the club's reputation
rank in its league, with some slack (`board.rs`). Confidence (0-100)
moves with every result against what a club with that target usually
averages, with the table each month (once eight games are played), and
with deep debt; it drifts back toward 60. Below 20 the manager is
sacked, human or AI. A season that ends far below target ends it too.

Vacant clubs are run by the AI. After a week the board appoints the best
available AI manager it can attract, or a newcomer. A human manager out
of work can apply for any vacancy (`ApplyForJob`); boards hire managers
whose reputation is within 3,000 of the club's (and anyone for small
clubs). Humans can also resign. Managers' reputations follow their
results; club reputations move a tenth of the way each season toward what
their finish implies, so promotion grows a club and relegation shrinks
it over seasons.

The world rule `sack_humans` (on by default) can switch off sackings of
human managers.

## 11. Health targets

A world must stay believable for decades. `fs-cli career` prints health
lines each season; the `ten_seasons_keep_the_world_healthy` test enforces
the core of it. A 30-season run of the tiny world currently shows:

| Measure | Target | Observed |
| --- | --- | --- |
| First-team (18+) mean age | 25-27 | 25.8 to 26.7 throughout |
| First-team age mix (18-20 / 21-23 / 24-27 / 28-31 / 32+) | Stable | About 17 / 19 / 26 / 25 / 13 % |
| Top-division best-XI ability | Within 5% of the start | Within about 2% |
| Active players | Stable | 690 to 780 for a 20-club world |
| Top-scorer goals | 22-36 | About 29 |
| Player wages / income (mean) | 35-80% | About 60% (club range 44-73%) |
| Clubs in debt | Under 20% | 0% |
| Managers sacked per season | Some, not most | 1 to 5 of 20 |

In the small world (120 leagues) wages take 53-58% of income, 2-3% of
clubs are in debt, and a season sees about 1,500 transfers and 1,400
loans. The tiny world has a single league and no lower divisions to sell
to, so it trades far less.

New worlds are generated with this steady-state age mix and with full
academies, so they do not lurch toward it over their first decade.

## 12. Performance

One season on the 4-core cloud build machine:

| World | Players | Season |
| --- | --- | --- |
| Tiny (1 league) | ~700 | 0.16 s |
| Small (120 leagues) | ~87,000 | ~6.4 s |
| Medium (300 leagues) | ~217,000 | ~16 s |

Milestone 4 (money, staff, scouting, the market and boards) took the
medium season from about 9 s to 16 s. Most of the increase is the AI
transfer market, which indexes candidates by position group once a day
and searches them in parallel. The 1 July rollover is the slowest day at
about 1.3 s in the medium world; quiet days take about 15 ms.
`cargo run --release -p fs-lab --example profile_season` shows the
breakdown.
