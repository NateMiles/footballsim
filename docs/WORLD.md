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
| July to early June | Cup rounds, midweek: continental qualifying in July and August, national cups on Wednesdays, continental cups on Tuesdays and Thursdays, finals in late May and early June (section 11) | `cup::daily` |
| 20 August, 10 January | Transfer-window squad housekeeping for AI clubs | `squad::maintain_squads` |
| 1 January | Human managers warned about contracts ending in June | `squad::warn_expiring` |
| 1 June | AI clubs renew contracts worth keeping; humans warned again | `squad::renew_contracts` |
| 1st of every month | Sponsorship, television, wages and running costs; boards review the table and the bank; happiness reviewed, transfer requests | `economy::monthly_accounts`, `board::monthly`, `happiness::daily` |
| 1st-28th of every month | Player development, each club's players on the club's own day | `tick::development` |
| Every day | Training (formations drilled), recovery, injury healing, training injuries, scouting, completed deals, vacant jobs filled; AI clubs due a tactic review (section 12) | `tick::afternoon` |
| After every match | Gate receipts; board confidence; playing time | `economy::gate_receipts`, `board::after_match` |

Each day runs in two parts: the morning's calendar business
(`tick::morning`), then matches, recovery and development
(`tick::afternoon`). A watched match day pauses between them (section 5).

Fixtures are a double round robin with the club order shuffled each
season. Rounds are spread evenly over the Saturdays from the second
weekend of August to late May; Wednesday rounds are added only for leagues
too large to fit.

## 2. Leagues

- A world's structure is data: a `WorldFormat` (`format.rs`) lists each
  nation's divisions (name, size, clubs swapped with the division below)
  and cups, and the continents with their cups. `fs-cli format` writes the
  default one as JSON and `--format FILE` plays an edited one; a format is
  validated before a world is built from it.
- By default each nation's clubs are ranked by reputation and split into
  divisions of 20 (`GenConfig::clubs_per_league`), with three clubs going
  up and three down between adjacent divisions.
- A league whose clubs include a human-managed club plays its matches with
  the tier 1 event engine; every other league uses tier 2 (section 12).

## 3. Players over time

**Matches.** Players who play lose fitness and gain fatigue and sharpness.
Results move the whole squad's morale; unused players get restless.
Injuries come from the match engine, with severities from minor (3-12
days) to serious (two to seven months). Injury-prone players stay out
longer. Red cards bring a one-match ban for a second yellow or three for a
straight red; every fifth yellow of the season brings a one-match ban.
Injured and suspended players are never picked.

**Training injuries** happen on any day, more often for injury-prone and
fatigued players, and more often under hard training.

**Training** (`training.rs`). Each club has a schedule: a focus and an
intensity. AI clubs train balanced at normal intensity, which is exactly
the world without training; only human managers change it.

| Focus | Effect |
| --- | --- |
| Balanced | Every attribute grows alike |
| Fitness, Attacking, Defending | The focus's attributes grow 1.6 times as fast as the rest; total growth is unchanged |
| Tactics | Mental and positional attributes emphasised; the formation in use is drilled twice as fast |
| Recovery | Growth ×0.8, recovery +25%, training injuries ×0.8, drilling halved |

| Intensity | Growth | Recovery | Training injuries | Drill per day |
| --- | --- | --- | --- | --- |
| Light | ×0.85 | +15% | ×0.7 | 9 |
| Normal | ×1 | ×1 | ×1 | 12 |
| Hard | ×1.12 | -15% | ×1.5 | 15 |

A player can also train individually: an attribute (it grows 1.5 times as
fast, the rest 3% slower) or a position (familiarity rises a point a
month, two with luck for adaptable players, up to natural). Individual
training ends when the player leaves.

**Tactical familiarity.** A club knows each formation 0-1000. The one it
plays gains the day's drill; the others lose 2 a day down to 300. From 900
up a shape costs nothing; below it, every player's match form is reduced,
by up to 5% for a shape the team has never played (`PlayerSnapshot::
cohesion`). New worlds start with each club knowing its formation fully.
AI clubs weigh familiarity when they choose a formation, so they rarely
change shape on a whim.

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

**Personality** (`personality.rs`). A label read from the hidden traits
(model professional, volatile, ambitious, loyal, professional, leader,
unambitious, casual, big-game, nervous, balanced). It only names what the
traits do elsewhere. Managers see it for their own players and for players
their scouts know at least 60%.

**Squad status and playing time** (`happiness.rs`). Every player at a club
expects a role: key player (70% of the minutes), regular starter (45%),
squad rotation (16%), backup (5%) or prospect (2%). New worlds and, each
summer, AI clubs give the five best of the club's first-choice eleven key
status, the other six regular status and the bench rotation, except the
reserve keeper. Everyone else is a backup, or a prospect if under 21.
Newcomers get the highest role whose current holders they are at least as
good as. After each match, each squad member's share of the minutes
updates a moving average over about six matches; injured and suspended
players are left out of it.

**Happiness**, from -100 to 100, is reviewed on the 1st of each month for
every player at a club, as a sum of visible reasons:

| Reason | Effect |
| --- | --- |
| Playing time | Down to -40 well short of the status's share (more for ambitious players); +6 for key players and regulars who get it |
| Wages | Down to -20 for established players paid under half their worth; +4 when paid well over it |
| Ambition | Down to -30 for ambitious players far better than the team |
| Prestige | +4 for adults at a club well above their level |
| Results | -8 to +12 by league place against the board's target, from six matches in |
| Future | -6 to -14 when the contract ends within six months (more for loyal players) |
| Captaincy | +8 |
| Transfer list | -6 to -20 for players listed without asking (more for loyal players) |
| Dressing room | 30% of the mood of the three most influential players (leadership, ability, age, years at the club, the armband) |
| Memories | Talks, promises, requests, demotions and a lost armband, for one to eight months |

Happiness sets where morale drifts back to each day (5000 plus 15 per
point), so it reaches matches through morale. Clubs without a captain
name the leader among their best and most experienced players.

**Talks and promises** (`talk.rs`). A manager can speak to each player
once a week: praise or criticise form, reassure, promise more football (a
status's share of the minutes within two months) or promise a better
contract (within two months). Reactions follow the truth (form over at
least three matches, playing time, the contract) and the personality:
criticising a player in form always angers; professional players take
criticism of poor form as a challenge, volatile players react strongly.
A player who remembers a broken promise does not believe reassurance or
new promises. Promises are checked when due (a football promise is put
back while the player is injured): kept ones are remembered for four
months, broken ones for eight.

**Transfer requests.** A player at or below -30 for three monthly reviews
in a row may ask to leave (less often when loyal or professional). AI
clubs list them at once; a human manager lists them or refuses, which the
player remembers for six months and does not ask again meanwhile. Taking
an agreed sale off the list counts as a refusal.

## 4. The 1 July rollover

In order:

1. Prize money; accounts archived; boards judge the season (sackings),
   club and manager reputations move.
2. Archive every league season (final table, top scorer); apply promotion
   and relegation. Archive every cup edition and add the season's
   continental results to the nations' coefficients.
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
10. AI tactics refreshed; AI clubs reset squad statuses and clubs
    without a captain name one; new league fixtures created; cup entrants
    decided (continental places from the tables and cups just finished)
    and first rounds drawn; boards set targets; facilities wear and
    boards invest spare money; budgets set.

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

## 11. Cups and continental competitions

Code: `cup.rs` (editions, draws, ties, groups, qualification,
coefficients), `fixture.rs` (`FixtureRef`, which names a league or cup
match everywhere a match is referred to), formats in `format.rs`, default
formats in `fs-gen/src/formats.rs`.

**Formats.** A cup lists its entries (which clubs, and the stage they
join) and its stages. A stage is a knockout round (one or two legs, extra
time or not, neutral ground or not, seeded or open draw, and how many go
through) or a group stage (groups, size, how many advance, how often each
pair meets), with a date rule per match day: the first given weekday on or
after a day of the year. Entries are:

- every club of the cup's nation in a range of divisions;
- for nations ranked in a range by coefficient, the clubs that finished in
  a range of places in their top division last season;
- for nations ranked in a range, last season's winner of their main cup.

**A season of a cup.** At the season start the entrants are decided and
the first stage drawn; each later stage is drawn the day after the
previous one finishes. A knockout round sends through half its field, or
first brings it down to a power of two (the best-seeded clubs, by
reputation, get the byes); a round can name its number of winners instead
(continental qualifying). Seeded draws pair the strongest half with the
weakest, group winners against runners-up, keeping clubs from the same
group and nation apart where they can. Group draws use pots by seeding
and keep clubs of one nation apart. A cup match never shares a day with
another of a club's matches: the draw moves it a day later. The last
match of a two-legged tie and every single-match tie must produce a
winner: aggregate score, then extra time if the stage plays it, then
penalties (`docs/MATCH_ENGINE.md`). Finals are at neutral grounds, where
the crowd (both clubs' fans, up to 80,000) pays both clubs half.

**Default structure.** Every nation has a national cup (prestige 40):
with three or more divisions the lower divisions start, and the top two
join once the field is down to about half their number (eight rounds in a
100-club nation, five in the tiny world). Single matches with extra time
on Wednesdays from August to April; the final on the Sunday after the
league ends. Nations are grouped into continents of at most 30 (the
strongest spread evenly); each continent of four or more nations plays:

| Cup | Prestige | Entrants | Shape |
| --- | --- | --- | --- |
| Champions Cup | 100 | top 4 of the four best nations, top 3 of the next two, top 2 of the next four, champions of the rest | two-legged qualifying rounds in July and August, eight groups of four (Sep-Dec, Tuesdays), two-legged round of 16, quarter- and semi-finals, a one-match final in early June |
| Continental Shield | 60 | the next league places (two for the best six nations, one for the rest) and every national cup winner not already in the Champions Cup | as above on Thursdays, final in late May |

The strongest nations' clubs go straight into the group stage until three
quarters of it is filled; the rest play qualifying rounds sized to fill it
exactly. A club plays in at most one continental cup a season, the more
prestigious; a place whose club is already in passes to the next club in
that nation's table. Before any season has finished, "last season's
table" is the order of reputation.

**Coefficients.** Each continental match earns a club's nation two points
for a win and one for a draw, plus one for every stage after the first
that the club plays in; a nation's season score is its total divided by
its number of entrants. A coefficient is the sum of the last five seasons
and ranks the nations for places. New worlds start with five seasons
equal to the nations' reputations, so the first ranking follows
reputation.

**Money and standing.** Each stage pays every club in it (continental
group stages pay about 1% of the best clubs' revenue potential; national
cup money doubles round by round to 1% for the winner), plus a winner's
prize. Boards are pleased by progress and annoyed by exits, much more so
when a smaller club knocks them out; winning raises board confidence and
the club's and manager's reputations by the cup's prestige. Players'
season statistics count every competition; league top scorers count
league goals. Development counts cup matches when working out playing
time.

## 12. Level of detail

`importance.rs` rates each club from where the human managers are: high
for human clubs and their leagues, medium for the rest of their nations
and for clubs in the same continental cups, low for everyone else. League
matches use tier 1 at high importance and tier 2 otherwise; cup matches
use tier 1 when a human club plays. AI clubs review their tactic every 30,
60 or 90 days by importance, each on its own days. Work that used to fall
on the 1st of the month (development, tactics) is spread over the month,
so no day carries it all; money and boards stay on the 1st.

## 13. Health targets

A world must stay believable for decades. `fs-cli career` prints health
lines each season; the `ten_seasons_keep_the_world_healthy` test enforces
the core of it. A 30-season run of the tiny world currently shows:

| Measure | Target | Observed |
| --- | --- | --- |
| First-team (18+) mean age | 25-27 | 25.8 to 26.7 throughout |
| First-team age mix (18-20 / 21-23 / 24-27 / 28-31 / 32+) | Stable | About 17 / 19 / 26 / 25 / 13 % |
| Top-division best-XI ability | Within 5% of the start | 841 to 866 (start 849) |
| Active players | Stable | 690 to 780 for a 20-club world |
| Top-scorer goals | 22-36 | About 31 |
| Player wages / income (mean) | 35-80% | About 52% (cup money included) |
| Clubs in debt | Under 20% | 0% |
| Managers sacked per season | Some, not most | 0 to 10 of 20 |
| Very unhappy players (-30 or below) | Under 10% | 1 to 7% |
| Transfer requests per club and season | About one | 9 to 32 in a 20-club world |

In the small world (120 leagues, 26 cups) over ten seasons, 2-3% of clubs
are in debt, about 15-18% of managers are sacked each season, and a
season sees 1,100-1,700 transfers, 1,400-1,800 loans and about 3,300
transfer requests (1.4 per club), with 5-6% of players very unhappy. Two slow drifts
predate the cups and are being watched: wages take 52% of income in the
first season and 64% by the tenth, and the strongest nation's top
division gets about 3% better (853 to 880) as its richer clubs buy
abroad. Continental cups are mostly won by the strongest nation's clubs,
with others breaking through. The tiny world has a single league and no
lower divisions to sell to, so it trades far less.

New worlds are generated with this steady-state age mix and with full
academies, so they do not lurch toward it over their first decade.

## 14. Performance

One season on the 4-core cloud build machine, with cups:

| World | Players | Season (4 threads) | Season (1 thread) |
| --- | --- | --- | --- |
| Tiny (1 league, 1 cup) | ~700 | 0.16 s | |
| Small (120 leagues, 26 cups) | ~84,000 | ~6.5 s | ~10.6 s |
| Medium (300 leagues, 64 cups) | ~210,000 | ~15 s | ~36 s |

The browser runs the core on one thread. The biggest costs in a medium
season are the matches (team selection and the engines), the AI transfer
market, player development and AI tactic reviews. The 1 July rollover is
the slowest day at about 1 s in the medium world; quiet days take about
22 ms. `cargo run --release -p fs-lab --example profile_season
--features fs-world/profile [small|medium]` breaks a season down by stage;
`docs/ARCHITECTURE.md` section 6 compares everything with the budgets.
