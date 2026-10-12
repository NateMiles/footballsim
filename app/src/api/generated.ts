// Generated from crates/fs-api by tests/bindings.rs. Do not edit.
// Regenerate: UPDATE_BINDINGS=1 cargo test -p fs-api --test bindings

export const PROTOCOL_VERSION = 9;

export type ApiErrorBody = { code: ErrorCode, 
/**
 * For people.
 */
message: string, };

export type Assignment = { player: PlayerRef, club: ClubRef | null, 
/**
 * 0-100.
 */
knowledge: number, };

export type AttributeGroup = { name: string, attributes: Array<AttributeValue>, };

export type AttributeValue = { name: string, value: number, low: number, high: number, };

export type BenchmarkStatus = { 
/**
 * Whether a test world is loaded.
 */
active: boolean, players: number, clubs: number, leagues: number, cups: number, date: string, 
/**
 * Matches played so far in all competitions.
 */
matches: number, 
/**
 * Memory the core uses, where the platform tells.
 */
memoryBytes: number | null, 
/**
 * Threads the simulation can use (one in a browser).
 */
threads: number, };

export type BoardView = { 
/**
 * Finish at this position or better.
 */
target: number, 
/**
 * Such as `"Finish in the top half"`.
 */
objective: string, 
/**
 * 0-100.
 */
confidence: number, };

export type CareerRow = { 
/**
 * Such as `"2026/27"`.
 */
season: string, club: ClubRef | null, ability: number, stats: SeasonStats, };

export type Category = "match" | "injury" | "contract" | "transfer" | "youth" | "season" | "club" | "finance" | "board" | "staff" | "scouting" | "cup" | "people";

export type ClubChoice = { club: ClubRef, league: LeagueRef, 
/**
 * 1 (minnows) to 5 (giants), by reputation within the world.
 */
stars: number, 
/**
 * Average ability of the best eleven, 0-100.
 */
squadAbility: number, };

export type ClubChoices = { clubs: Array<ClubChoice>, };

export type ClubRef = { id: number, name: string, shortName: string, colors: Colors, };

export type CoefficientRow = { rank: number, nation: string, code: string, 
/**
 * The sum of the recent seasons.
 */
points: number, 
/**
 * Points per season, oldest first.
 */
seasons: Array<number>, };

export type Colors = { primary: string, secondary: string, };

export type Commentary = { 
/**
 * Seconds since kickoff.
 */
t: number, clock: string, kind: CommentaryKind, side: Side | null, text: string, };

export type CommentaryKind = "goal" | "chance" | "save" | "card" | "substitution" | "injury" | "foul" | "setPiece" | "play" | "whistle" | "interval" | "shootout";

export type Comparison = { 
/**
 * Such as `"Attack"`.
 */
area: string, 
/**
 * -2 (they are much weaker) to +2 (much stronger).
 */
edge: number, 
/**
 * Such as `"Their attack is stronger than yours"`.
 */
verdict: string, };

export type CompetitionKind = "league" | "cup";

export type CompetitionRef = { kind: CompetitionKind, 
/**
 * A league or cup ID, by `kind`.
 */
id: number, name: string, shortName: string, };

export type CompetitionSeason = { 
/**
 * `None` for all competitions together.
 */
competition: CompetitionRef | null, played: number, won: number, drawn: number, lost: number, goalsFor: number, goalsAgainst: number, xgFor: number, xgAgainst: number, shotsFor: number, shotsAgainst: number, 
/**
 * Mean share, 0-1.
 */
possession: number, 
/**
 * Players who appeared, most minutes first.
 */
players: Array<PlayerLine>, 
/**
 * The matches, newest first.
 */
matches: Array<Fixture>, };

export type CompetitionsView = { nations: Array<NationCompetitions>, continents: Array<ContinentView>, };

export type ContinentView = { name: string, cups: Array<CupRef>, 
/**
 * Best first.
 */
ranking: Array<CoefficientRow>, };

export type Continued = { date: string, stopped: StopReason, newMessages: number, matchDay: MatchDay | null, };

export type Contract = { 
/**
 * Per week.
 */
wage: number, expires: string, appearanceBonus: number, goalBonus: number, releaseClause: number | null, };

export type ContractTerms = { 
/**
 * Per week.
 */
wage: number, years: number, status: SquadStatus, appearanceBonus: number, goalBonus: number, signingFee: number, releaseClause: number | null, };

export type CupRef = { id: number, name: string, shortName: string, 
/**
 * The nation, or the continent for continental cups.
 */
region: string, continental: boolean, };

export type CupStage = { name: string, 
/**
 * Knockout ties (empty for a group stage).
 */
ties: Array<Tie>, 
/**
 * Groups (empty for a knockout round).
 */
groups: Array<GroupView>, 
/**
 * Clubs through without playing.
 */
byes: Array<ClubRef>, };

export type CupStatus = { cup: CupRef, 
/**
 * Such as `"Quarter-finals"`, `"Out in the Second round"` or
 * `"Winners"`.
 */
status: string, 
/**
 * Still in it (or won it).
 */
alive: boolean, };

export type CupView = { cup: CupRef, 
/**
 * Such as `"2026/27"`.
 */
season: string, 
/**
 * Stages drawn so far, first first.
 */
stages: Array<CupStage>, 
/**
 * Stages still to be drawn.
 */
upcoming: Array<string>, winner: ClubRef | null, 
/**
 * Past winners, most recent first.
 */
history: Array<CupWinner>, 
/**
 * The other cups the viewer can switch to.
 */
cups: Array<CupRef>, };

export type CupWinner = { season: string, winner: ClubRef | null, runnerUp: ClubRef | null, };

export type Duty = "defend" | "support" | "attack";

export type DynamicsRow = { player: PlayerRef, age: number, personality: string, status: SquadStatus, playingTime: number, expected: number, happiness: number, label: string, 
/**
 * The biggest reason, if any.
 */
reason: MoodReason | null, captain: boolean, influential: boolean, request: RequestView | null, };

export type DynamicsView = { captain: PlayerRef | null, 
/**
 * The most influential players, most influential first.
 */
leaders: Array<PlayerRef>, 
/**
 * Mean happiness of senior players.
 */
happiness: number, 
/**
 * Such as `"Good"` or `"Tense"`.
 */
atmosphere: string, rows: Array<DynamicsRow>, promises: Array<PromiseView>, statuses: Array<StatusInfo>, talks: Array<TalkInfo>, };

export type ErrorCode = "badRequest" | "noGame" | "notFound" | "rejected" | "noMatch" | "badSave";

export type Facilities = { capacity: number, 
/**
 * 1-20.
 */
training: number, youth: number, };

export type FinancesView = { club: ClubRef, balance: number, transferBudget: number, 
/**
 * Weekly.
 */
wageBudget: number, 
/**
 * Weekly, players.
 */
wageBill: number, 
/**
 * Weekly, staff.
 */
staffBill: number, season: Ledger, 
/**
 * Newest first.
 */
history: Array<SeasonAccounts>, facilities: Facilities, };

export type Fixture = { competition: CompetitionRef, 
/**
 * Index into the competition's fixtures this season.
 */
index: number, date: string, home: ClubRef, away: ClubRef, result: FixtureResult | null, 
/**
 * Cup round and leg, such as `"Semi-finals, 2nd leg"` or
 * `"Group stage, Group B"`.
 */
round: string | null, 
/**
 * Played at a neutral ground.
 */
neutral: boolean, };

export type FixtureKey = { kind: CompetitionKind, id: number, index: number, };

export type FixtureResult = { 
/**
 * Including extra time.
 */
goals: [number, number], xg: [number, number], scorers: Array<Scorer>, extraTime: boolean, 
/**
 * Penalties scored in a shootout, home then away.
 */
penalties: [number, number] | null, };

export type FixturesView = { league: LeagueRef, club: ClubRef | null, fixtures: Array<Fixture>, };

export type FocusInfo = { focus: TrainingFocus, name: string, description: string, };

export type Formation = "4-4-2" | "4-3-3" | "4-2-3-1" | "3-5-2" | "5-3-2" | "4-1-4-1";

export type FormationFamiliarity = { formation: Formation, 
/**
 * 0-100; 90 and above costs nothing.
 */
familiarity: number, 
/**
 * The formation the team plays now.
 */
current: boolean, };

export type FormationInfo = { formation: Formation, 
/**
 * Eleven slots; slot 0 is the goalkeeper.
 */
slots: Array<SlotInfo>, };

export type Frame = { 
/**
 * Seconds since kickoff.
 */
t: number, 
/**
 * x, y and height in metres.
 */
ball: [number, number, number], players: Array<PlayerFrame>, };

export type GroupView = { 
/**
 * Such as `"Group A"`.
 */
name: string, rows: Array<TableRow>, fixtures: Array<Fixture>, };

export type Honour = { competition: string, seasons: Array<string>, };

export type InboxView = { messages: Array<Message>, unread: number, };

export type IndividualTraining = { "type": "position", position: string, } | { "type": "attribute", attribute: string, };

export type Injury = { 
/**
 * `"minor"`, `"moderate"`, `"major"` or `"serious"`.
 */
severity: string, until: string, };

export type Instruction = { "type": "substitute", off: number, on: number, } | { "type": "changeTactic", tactic: Tactic, } | { "type": "shout", shout: Shout, };

export type InstructionsView = { 
/**
 * By formation slot, for the formation the team plays now.
 */
roles: Array<SlotRoleView>, 
/**
 * `None` leaves the choice to the best player for it on the pitch.
 */
penaltyTaker: number | null, freeKickTaker: number | null, cornerTaker: number | null, 
/**
 * For the next match only.
 */
marking: Array<MarkingView>, markings: Array<MarkingInfo>, 
/**
 * At most this many opposition instructions.
 */
maxMarking: number, };

export type IntensityInfo = { intensity: TrainingIntensity, name: string, description: string, };

export type Job = { club: ClubRef, league: LeagueRef, 
/**
 * 1-5.
 */
stars: number, 
/**
 * Whether the board would take the viewer.
 */
wouldHire: boolean, };

export type JobsView = { 
/**
 * The viewer's reputation, 0-100.
 */
reputation: number, employed: boolean, jobs: Array<Job>, };

export type KeyPlayer = { player: PlayerRef, reason: string, };

export type Kick = { side: Side, taker: number, scored: boolean, };

export type LeagueRef = { id: number, name: string, nation: string, 
/**
 * 1 for a nation's top division.
 */
level: number, };

export type Ledger = { matchday: number, broadcast: number, commercial: number, prizeMoney: number, playerSales: number, playerWages: number, staffWages: number, playerPurchases: number, runningCosts: number, investment: number, 
/**
 * Appearance and goal bonuses.
 */
bonuses: number, 
/**
 * Signing fees and agents' fees.
 */
signingFees: number, income: number, expenses: number, };

export type Lineup = { 
/**
 * Eleven players by formation slot.
 */
starters: Array<number>, bench: Array<number>, };

export type Link = { "type": "player", player: number, } | { "type": "schedule" } | { "type": "table", league: number, } | { "type": "cup", cup: number, } | { "type": "squad" } | { "type": "tactics" } | { "type": "transfers" } | { "type": "staff" } | { "type": "jobs" } | { "type": "finances" } | { "type": "dynamics" };

export type ListedPlayer = { player: PlayerRef, listing: Listing, };

export type Listing = "transfer" | "loan";

export type LiveChunk = { frames: Array<Frame>, commentary: Array<Commentary>, events: Array<MatchEvent>, score: [number, number], 
/**
 * Such as `"45+2"`.
 */
clock: string, 
/**
 * Seconds since kickoff, counting stoppage time.
 */
elapsed: number, finished: boolean, stats: [TeamStats, TeamStats], lineups: [LiveLineup, LiveLineup], 
/**
 * The shootout, in the chunk that ends a match with one.
 */
shootout: Shootout | null, };

export type LiveLineup = { tactic: Tactic, 
/**
 * By formation slot; empty after a sending off.
 */
onPitch: Array<OnPitch | null>, bench: Array<number>, substitutionsLeft: number, shout: Shout | null, };

export type LivePlayerInfo = { player: PlayerRef, side: Side, };

export type LiveStart = { fixture: Fixture, 
/**
 * The side the viewer manages.
 */
side: Side, pitch: Pitch, 
/**
 * Frames per second of match time.
 */
frameHz: number, 
/**
 * Everyone in either squad today.
 */
players: Array<LivePlayerInfo>, lineups: [LiveLineup, LiveLineup], shouts: Array<Shout>, };

export type Loan = { player: PlayerRef, parent: ClubRef, borrower: ClubRef, until: string, wageShare: number, };

export type ManagerInfo = { id: number, name: string, };

export type Marking = "tight" | "closeDown";

export type MarkingChoice = { player: number, marking: Marking, };

export type MarkingInfo = { marking: Marking, name: string, description: string, };

export type MarkingView = { player: PlayerRef, club: ClubRef | null, marking: Marking, };

export type MatchAnalysisView = { fixture: Fixture, goals: [number, number], stats: [TeamStats, TeamStats], shots: Array<ShotView>, xg: Array<XgPoint>, 
/**
 * The home side's share of possession in each 15-minute spell.
 */
possession: Array<number>, sides: [SideAnalysisView, SideAnalysisView], players: [Array<PlayerLine>, Array<PlayerLine>], };

export type MatchDay = { fixture: Fixture, side: Side, };

export type MatchEvent = { "type": "goal", minute: number, side: Side, scorer: number, assist: number | null, 
/**
 * `"openPlay"`, `"header"`, `"longShot"`, `"penalty"`,
 * `"freeKick"` or `"corner"`.
 */
kind: string, xg: number, } | { "type": "penaltyMissed", minute: number, side: Side, taker: number, } | { "type": "card", minute: number, side: Side, player: number, red: boolean, } | { "type": "substitution", minute: number, side: Side, off: number, on: number, } | { "type": "injury", minute: number, side: Side, player: number, };

export type MatchReport = { fixture: Fixture, stats: [TeamStats, TeamStats], events: Array<MatchEvent>, 
/**
 * Everyone who played, by side.
 */
ratings: [Array<PlayerRating>, Array<PlayerRating>], shootout: Shootout | null, };

export type Message = { id: number, date: string, category: Category, title: string, body: string, read: boolean, link: Link | null, };

export type Mood = { 
/**
 * -100 to 100.
 */
happiness: number, 
/**
 * Such as `"Happy"` or `"Very unhappy"`.
 */
label: string, 
/**
 * Largest first.
 */
reasons: Array<MoodReason>, };

export type MoodReason = { text: string, 
/**
 * Points of happiness, positive or negative.
 */
effect: number, };

export type NationCompetitions = { nation: string, code: string, leagues: Array<LeagueRef>, cups: Array<CupRef>, };

export type NegotiationView = { player: PlayerRef, 
/**
 * The club they play for now (none for a free agent).
 */
club: ClubRef | null, 
/**
 * Whether this is a renewal.
 */
renewal: boolean, 
/**
 * The agent's opening ask.
 */
ask: ContractTerms, 
/**
 * Whether they insist on a release clause.
 */
wantsClause: boolean, 
/**
 * Such as `"Reasonable"` or `"Hard bargainer"`.
 */
agent: string, 
/**
 * The agent's fee as a percentage of the contract's wages.
 */
agentFeePercent: number, 
/**
 * Offers left before the agent walks away.
 */
offersLeft: number, 
/**
 * Talks broke down; they reopen on this day.
 */
closedUntil: string | null, 
/**
 * The player wants to leave and will not renew.
 */
wantsOut: boolean, 
/**
 * Weekly wages the club pays now and may pay in all.
 */
wageBill: number, wageBudget: number, statuses: Array<StatusInfo>, };

export type NetworkLink = { from: number, to: number, count: number, };

export type NetworkNode = { player: PlayerRef, 
/**
 * 0-5 from their own goal line.
 */
band: number, 
/**
 * 0-2 from their left.
 */
lane: number, involvements: number, };

export type Offer = { id: number, date: string, player: PlayerRef, buyer: ClubRef, seller: ClubRef, kind: OfferKind, 
/**
 * The contract offered to the player (transfers).
 */
wage: number, years: number, 
/**
 * Percent of the player's next fee the seller would keep.
 */
sellOn: number, status: OfferStatus, respondBy: string, };

export type OfferKind = { "type": "transfer", fee: number, } | { "type": "loan", wageShare: number, };

export type OfferResponse = { "type": "accept" } | { "type": "reject" } | { "type": "counter", fee: number, };

export type OfferStatus = { "type": "pending" } | { "type": "countered", fee: number, } | { "type": "agreed" } | { "type": "completed" } | { "type": "refused", reason: string, };

export type OnPitch = { id: number, slot: number, 
/**
 * 0-1.
 */
energy: number, yellow: boolean, };

export type OppositionReport = { club: ClubRef, 
/**
 * The match this report is for, if they are next.
 */
fixture: Fixture | null, league: LeagueRef | null, position: number | null, 
/**
 * Their likely tactic.
 */
tactic: Tactic, 
/**
 * How they play, in words.
 */
style: Array<string>, 
/**
 * Their likely eleven, by formation slot.
 */
likelyXi: Array<ReportPlayer>, 
/**
 * Last five results in every competition, oldest first.
 */
form: Array<string>, 
/**
 * This season in every competition.
 */
record: Record, keyPlayers: Array<KeyPlayer>, 
/**
 * Regulars who are injured or suspended.
 */
absent: Array<KeyPlayer>, 
/**
 * Area by area against your team.
 */
comparison: Array<Comparison>, 
/**
 * Expected goals in the match: theirs, then yours.
 */
expectedGoals: [number, number] | null, 
/**
 * How well your scouts know their players, 0-100.
 */
knowledge: number, };

export type Overview = { date: string, 
/**
 * Such as `"Saturday"`.
 */
weekday: string, 
/**
 * Such as `"2026/27"`.
 */
season: string, manager: ManagerInfo, 
/**
 * The viewer's club; none while out of work.
 */
club: ClubRef | null, league: LeagueRef | null, position: number | null, nextFixture: Fixture | null, lastFixture: Fixture | null, 
/**
 * Last five league results, oldest first: `"W"`, `"D"` or `"L"`.
 */
form: Array<string>, unread: number, 
/**
 * Today's match, waiting to be watched.
 */
matchDay: MatchDay | null, watchMatches: boolean, delegateSquad: boolean, board: BoardView | null, balance: number | null, transferBudget: number | null, windowOpen: boolean, 
/**
 * The club's cups this season.
 */
cups: Array<CupStatus>, };

export type Personality = { name: string, description: string, };

export type Pitch = { length: number, width: number, };

export type PlayerFrame = { id: number, side: Side, x: number, y: number, 
/**
 * Radians; 0 faces the away goal.
 */
facing: number, pose: Pose, };

export type PlayerLine = { player: PlayerRef, started: boolean, minutes: number, goals: number, assists: number, shots: number, xg: number, passes: number, passesCompleted: number, keyPasses: number, dribblesWon: number, tackles: number, interceptions: number, 
/**
 * Average for season lines.
 */
rating: number, };

export type PlayerPeople = { status: SquadStatus, 
/**
 * Recent share of the club's minutes, percent.
 */
playingTime: number, 
/**
 * What the status expects, percent.
 */
expected: number, mood: Mood, captain: boolean, influential: boolean, request: RequestView | null, promises: Array<PromiseView>, lastTalk: string | null, 
/**
 * Whether a talk is possible today.
 */
canTalk: boolean, };

export type PlayerRating = { player: PlayerRef, minutes: number, goals: number, assists: number, rating: number, };

export type PlayerRef = { id: number, name: string, 
/**
 * Initial and surname, for tight spaces.
 */
shortName: string, number: number | null, 
/**
 * Best position code, such as `"DC"`.
 */
position: string, };

export type PlayerRole = "goalkeeper" | "sweeperKeeper" | "centralDefender" | "ballPlayingDefender" | "fullBack" | "wingBack" | "defensiveMidfielder" | "ballWinner" | "deepPlaymaker" | "centralMidfielder" | "boxToBox" | "playmaker" | "wideMidfielder" | "winger" | "insideForward" | "wideForward" | "attackingMidfielder" | "shadowStriker" | "striker" | "targetMan" | "poacher" | "falseNine";

export type PlayerSearch = { 
/**
 * Best-position codes to include; empty for any.
 */
positions: Array<string>, minAge: number | null, maxAge: number | null, 
/**
 * Estimated value at most this.
 */
maxValue: number | null, 
/**
 * Estimated ability at least this (0-100).
 */
minAbility: number | null, 
/**
 * Only players their clubs have listed.
 */
listedOnly: boolean, 
/**
 * Only players without a club.
 */
freeAgentsOnly: boolean, 
/**
 * Name contains this (case-insensitive).
 */
name: string | null, };

export type PlayerView = { id: number, name: string, firstName: string, lastName: string, age: number, birthDate: string, nation: string, club: ClubRef | null, number: number | null, 
/**
 * `"left"`, `"right"` or `"both"`.
 */
foot: string, heightCm: number, weightKg: number, position: string, 
/**
 * Familiarity with every position, 0-20, in [`PositionInfo`] order.
 */
positionRatings: Array<number>, attributes: Array<AttributeGroup>, 
/**
 * What the viewer's club knows about the player, 0-100. At 100 the
 * numbers are exact; below, they are the scouts' estimates.
 */
knowledge: number, 
/**
 * Estimated ability and potential, 0-100, with ranges.
 */
ability: number, abilityLow: number, abilityHigh: number, potential: number, potentialLow: number, potentialHigh: number, 
/**
 * Estimated market value.
 */
value: number, 
/**
 * What the club would sell for, if it is an AI club and would sell.
 */
askingPrice: number | null, listing: Listing | null, loan: Loan | null, 
/**
 * Whether the viewer's scouts are watching the player.
 */
scouting: boolean, 
/**
 * The viewer's open offer for the player.
 */
offer: Offer | null, fitness: number, sharpness: number, morale: number, injury: Injury | null, suspension: number, contract: Contract | null, 
/**
 * What they would ask for in a new contract, per week.
 */
wageDemand: number, season: SeasonStats, career: Array<CareerRow>, retired: boolean, 
/**
 * Whether the viewer manages this player.
 */
own: boolean, 
/**
 * Their character, once the viewer knows them well enough.
 */
personality: Personality | null, 
/**
 * Mood, status and promises, for the player's own manager.
 */
people: PlayerPeople | null, 
/**
 * A former club's share of their next fee.
 */
sellOn: SellOnView | null, };

export type Pose = "standing" | "running" | "passing" | "shooting" | "heading" | "tackling" | "diving" | "celebrating";

export type PositionInfo = { code: string, name: string, };

export type Prediction = { fixture: Fixture, 
/**
 * The tactic your side used.
 */
tactic: Tactic, runs: number, 
/**
 * Chances from the home side's view, 0-1.
 */
homeWin: number, draw: number, awayWin: number, goals: [number, number], xg: [number, number], 
/**
 * The likeliest scores, most likely first.
 */
scores: Array<ScoreChance>, };

export type ProjectionRow = { club: ClubRef, 
/**
 * Position today.
 */
position: number, points: number, expectedPoints: number, expectedPosition: number, 
/**
 * Chance of finishing in each zone, in [`ProjectionsView::zones`]
 * order, 0-1.
 */
zones: Array<number>, 
/**
 * Chance of each final position, first first.
 */
positions: Array<number>, };

export type ProjectionZone = { 
/**
 * Such as `"Champions"`, `"Champions Cup"`, `"Relegation"`.
 */
name: string, from: number, to: number, cup: CupRef | null, };

export type ProjectionsView = { league: LeagueRef, season: string, 
/**
 * Seasons simulated.
 */
runs: number, 
/**
 * Likeliest finish first.
 */
rows: Array<ProjectionRow>, 
/**
 * Places that lead somewhere, best first.
 */
zones: Array<ProjectionZone>, };

export type PromiseView = { player: PlayerRef, 
/**
 * `"football"` or `"contract"`.
 */
kind: string, text: string, made: string, due: string, 
/**
 * Whether it would count as kept if it fell due today.
 */
onTrack: boolean, };

export type Reaction = "delighted" | "pleased" | "determined" | "indifferent" | "sceptical" | "annoyed" | "angry";

export type Record = { played: number, won: number, drawn: number, lost: number, scored: number, conceded: number, };

export type ReportPlayer = { player: PlayerRef, 
/**
 * The position they would play.
 */
position: string, 
/**
 * Estimated ability, 0-100.
 */
ability: number, 
/**
 * Goals this season, every competition.
 */
goals: number, };

export type Request = { "type": "hello" } | { "type": "newWorld", seed: number, size: WorldSize, } | { "type": "startCareer", club: number, firstName: string, lastName: string, } | { "type": "save" } | { "type": "load", data: string, } | { "type": "overview" } | { "type": "inbox" } | { "type": "markRead", id: number | null, } | { "type": "squad", club: number | null, } | { "type": "player", player: number, } | { "type": "tactics" } | { "type": "setTactic", tactic: Tactic | null, } | { "type": "setLineup", lineup: Lineup | null, } | { "type": "fixtures", league: number | null, club: number | null, } | { "type": "table", league: number | null, } | { "type": "schedule", club: number | null, } | { "type": "cup", cup: number | null, } | { "type": "competitions" } | { "type": "oppositionReport", club: number | null, } | { "type": "predict", tactic: Tactic | null, runs: number, } | { "type": "projections", league: number | null, } | { "type": "setWatchMatches", watch: boolean, } | { "type": "setDelegation", squad: boolean, } | { "type": "offerContract", player: number, wage: number, years: number, } | { "type": "releasePlayer", player: number, } | { "type": "continue" } | { "type": "startMatch" } | { "type": "liveTick", until: number, } | { "type": "liveInstruct", instruction: Instruction, } | { "type": "finishMatch" } | { "type": "skipMatch" } | { "type": "finances" } | { "type": "staff" } | { "type": "hireStaff", staff: number, wage: number, years: number, } | { "type": "releaseStaff", staff: number, } | { "type": "transfers" } | { "type": "searchPlayers", search: PlayerSearch, } | { "type": "makeOffer", player: number, kind: OfferKind, wage: number, years: number, 
/**
 * A full contract package for a transfer, replacing `wage` and
 * `years`.
 */
terms?: ContractTerms, 
/**
 * Percent of the player's next fee the seller keeps.
 */
sellOn?: number, } | { "type": "respondToOffer", offer: number, response: OfferResponse, } | { "type": "acceptCounter", offer: number, } | { "type": "withdrawOffer", offer: number, } | { "type": "setListing", player: number, listing: Listing | null, } | { "type": "scoutPlayer", player: number, } | { "type": "cancelScouting", player: number, } | { "type": "jobs" } | { "type": "applyForJob", club: number, } | { "type": "resign" } | { "type": "training" } | { "type": "setTraining", focus: TrainingFocus, intensity: TrainingIntensity, } | { "type": "setIndividualTraining", player: number, training: IndividualTraining | null, } | { "type": "dynamics" } | { "type": "talkToPlayer", player: number, talk: Talk, } | { "type": "answerTransferRequest", player: number, accept: boolean, } | { "type": "setCaptain", player: number, } | { "type": "setSquadStatus", player: number, status: SquadStatus, } | { "type": "matchAnalysis", fixture: FixtureKey, } | { "type": "seasonStats", club: number | null, } | { "type": "contractTalks", player: number, } | { "type": "negotiateContract", player: number, terms: ContractTerms, } | { "type": "setInstructions", roles: Array<RoleChoice>, penaltyTaker: number | null, freeKickTaker: number | null, cornerTaker: number | null, marking: Array<MarkingChoice>, } | { "type": "benchmarkStart", size: WorldSize, } | { "type": "benchmarkRun", days: number, } | { "type": "benchmarkEnd" };

export type RequestView = { date: string, 
/**
 * Whether the club agreed to sell.
 */
accepted: boolean, };

export type Response = { "type": "hello", protocol: number, version: string, 
/**
 * Whether a game is loaded.
 */
inGame: boolean, } | { "type": "clubChoices", choices: ClubChoices, } | { "type": "saved", save: SaveGame, } | { "type": "overview", overview: Overview, } | { "type": "inbox", inbox: InboxView, } | { "type": "squad", squad: SquadView, } | { "type": "player", player: PlayerView, } | { "type": "tactics", tactics: TacticsView, } | { "type": "fixtures", fixtures: FixturesView, } | { "type": "table", table: TableView, } | { "type": "schedule", schedule: ScheduleView, } | { "type": "cup", cup: CupView, } | { "type": "competitions", competitions: CompetitionsView, } | { "type": "oppositionReport", report: OppositionReport, } | { "type": "prediction", prediction: Prediction, } | { "type": "projections", projections: ProjectionsView, } | { "type": "continued", continued: Continued, } | { "type": "liveStart", start: LiveStart, } | { "type": "liveChunk", chunk: LiveChunk, } | { "type": "matchReport", report: MatchReport, } | { "type": "finances", finances: FinancesView, } | { "type": "staff", staff: StaffView, } | { "type": "transfers", transfers: TransfersView, } | { "type": "search", search: SearchView, } | { "type": "jobs", jobs: JobsView, } | { "type": "benchmark", benchmark: BenchmarkStatus, } | { "type": "training", training: TrainingView, } | { "type": "dynamics", dynamics: DynamicsView, } | { "type": "negotiation", negotiation: NegotiationView, } | { "type": "matchAnalysis", analysis: MatchAnalysisView, } | { "type": "seasonStats", stats: SeasonStatsView, } | { "type": "talks", result: TalksResult, } | { "type": "talk", result: TalkResult, } | { "type": "error", error: ApiErrorBody, };

export type RoleChoice = { role: PlayerRole, duty: Duty, };

export type RoleInfo = { role: PlayerRole, name: string, description: string, };

export type SaveGame = { 
/**
 * The save itself, base64.
 */
data: string, 
/**
 * For listing saves without loading them.
 */
summary: SaveSummary, };

export type SaveSummary = { date: string, manager: string, club: string, };

export type ScheduleView = { club: ClubRef, fixtures: Array<Fixture>, };

export type ScoreChance = { goals: [number, number], chance: number, };

export type Scorer = { side: Side, player: PlayerRef, };

export type SearchRow = { player: PlayerRef, club: ClubRef | null, age: number, nation: string, 
/**
 * Estimated ability, 0-100, with its range.
 */
ability: number, abilityLow: number, abilityHigh: number, potential: number, 
/**
 * 0-100.
 */
knowledge: number, 
/**
 * Estimated value.
 */
value: number, wageDemand: number, listing: Listing | null, };

export type SearchView = { rows: Array<SearchRow>, 
/**
 * Matches before the limit.
 */
total: number, };

export type SeasonAccounts = { 
/**
 * Such as `"2026/27"`.
 */
season: string, ledger: Ledger, closingBalance: number, };

export type SeasonStats = { appearances: number, starts: number, minutes: number, goals: number, assists: number, yellows: number, reds: number, cleanSheets: number, 
/**
 * Average match rating, if they played.
 */
rating: number | null, };

export type SeasonStatsView = { club: ClubRef, 
/**
 * All competitions first.
 */
competitions: Array<CompetitionSeason>, };

export type SelectablePlayer = { id: number, name: string, shortName: string, number: number | null, position: string, available: boolean, fitness: number, 
/**
 * How well they would play each position today, 0-100, in
 * [`PositionInfo`] order.
 */
suitability: Array<number>, };

export type Selection = "starter" | "bench" | "reserve";

export type SellOnView = { club: ClubRef, percent: number, };

export type Shootout = { kicks: Array<Kick>, 
/**
 * Home then away.
 */
score: [number, number], };

export type ShotView = { minute: number, side: Side, player: PlayerRef, 
/**
 * `"shot"`, `"header"`, `"penalty"` or `"free kick"`.
 */
kind: string, band: number, lane: number, xg: number, 
/**
 * `"goal"`, `"saved"`, `"missed"` or `"blocked"`.
 */
outcome: string, };

export type Shout = "encourage" | "demandMore" | "calmDown" | "tightenUp" | "pushForward";

export type Side = "home" | "away";

export type SideAnalysisView = { club: ClubRef, 
/**
 * Actions by zone: six bands of three lanes, in the side's frame.
 */
touches: Array<Array<number>>, nodes: Array<NetworkNode>, 
/**
 * The most frequent connections.
 */
links: Array<NetworkLink>, };

export type SliderInfo = { 
/**
 * Field name in [`Tactic`].
 */
key: string, name: string, 
/**
 * What -2 and +2 mean.
 */
low: string, high: string, };

export type SlotInfo = { 
/**
 * Index into [`PositionInfo`] order.
 */
position: number, x: number, y: number, };

export type SlotRoleView = { 
/**
 * Position code of the slot, such as `"ML"`.
 */
position: string, role: PlayerRole, duty: Duty, 
/**
 * Roles the slot can take, standard first.
 */
options: Array<RoleInfo>, 
/**
 * Whether the slot takes a duty (every slot but the keeper's).
 */
hasDuty: boolean, };

export type SquadPlayer = { id: number, name: string, number: number | null, age: number, 
/**
 * Nation code, such as `"ENG"`.
 */
nation: string, 
/**
 * Best position code.
 */
position: string, 
/**
 * Every position they can play well, best first.
 */
positions: Array<string>, 
/**
 * Current ability, 0-100.
 */
ability: number, 
/**
 * Potential ability, 0-100.
 */
potential: number, 
/**
 * Percentages.
 */
fitness: number, sharpness: number, morale: number, injury: Injury | null, 
/**
 * Matches still to serve.
 */
suspension: number, contract: Contract | null, season: SeasonStats, selection: Selection, 
/**
 * Under 18: an academy player.
 */
academy: boolean, };

export type SquadStatus = "key" | "regular" | "rotation" | "backup" | "prospect";

export type SquadView = { club: ClubRef, players: Array<SquadPlayer>, 
/**
 * Trophies won while the world has been running.
 */
honours: Array<Honour>, };

export type StaffAttributes = { coaching: number, youthCoaching: number, judgingAbility: number, judgingPotential: number, physiotherapy: number, management: number, };

export type StaffMember = { id: number, name: string, role: StaffRole, age: number, nation: string, attributes: StaffAttributes, 
/**
 * The rating that matters for the role, 1-20.
 */
rating: number, 
/**
 * Current weekly wage, for employed staff.
 */
wage: number | null, expires: string | null, 
/**
 * What they ask for, per week.
 */
demand: number, 
/**
 * Whether they would join the viewer's club.
 */
wouldJoin: boolean, };

export type StaffRatings = { coaching: number, youthCoaching: number, scoutingAbility: number, scoutingPotential: number, physiotherapy: number, management: number, };

export type StaffRole = "assistantManager" | "coach" | "scout" | "physio";

export type StaffView = { club: ClubRef, staff: Array<StaffMember>, ratings: StaffRatings, 
/**
 * Free staff, best first.
 */
available: Array<StaffMember>, 
/**
 * The most of each role a club may employ, in role order.
 */
limits: Array<number>, };

export type StatusInfo = { status: SquadStatus, name: string, 
/**
 * Expected share of the club's minutes, percent.
 */
expected: number, };

export type StopReason = "matchDay" | "messages" | "week";

export type TableRow = { position: number, club: ClubRef, played: number, won: number, drawn: number, lost: number, scored: number, conceded: number, goalDifference: number, points: number, 
/**
 * Last five results, oldest first: `"W"`, `"D"` or `"L"`.
 */
form: Array<string>, zone: TableZone | null, };

export type TableView = { league: LeagueRef, 
/**
 * Such as `"2026/27"`.
 */
season: string, rows: Array<TableRow>, 
/**
 * Every league, for switching.
 */
leagues: Array<LeagueRef>, };

export type TableZone = "champion" | "promotion" | "relegation" | "qualify";

export type Tactic = { formation: Formation, mentality: number, tempo: number, width: number, pressing: number, defensiveLine: number, directness: number, };

export type TacticsView = { club: ClubRef, tactic: Tactic, 
/**
 * True when the manager chose the tactic, false when the assistant
 * picks it.
 */
ownTactic: boolean, 
/**
 * True when the manager picked the lineup.
 */
ownLineup: boolean, 
/**
 * The team that would play today: player per formation slot.
 */
starters: Array<number>, bench: Array<number>, players: Array<SelectablePlayer>, formations: Array<FormationInfo>, positions: Array<PositionInfo>, sliders: Array<SliderInfo>, benchSize: number, instructions: InstructionsView, };

export type Talk = "praise" | "criticise" | "reassure" | "promiseFootball" | "promiseContract";

export type TalkInfo = { talk: Talk, name: string, description: string, };

export type TalkResult = { reaction: Reaction, message: string, player: PlayerView, };

export type TalksOutcome = { "type": "agreed" } | { "type": "countered", counter: ContractTerms, } | { "type": "brokenOff", until: string, };

export type TalksResult = { outcome: TalksOutcome, 
/**
 * What the agent said.
 */
message: string, negotiation: NegotiationView, };

export type TeamStats = { 
/**
 * 0-1.
 */
possession: number, shots: number, shotsOnTarget: number, xg: number, passes: number, passesCompleted: number, crosses: number, tackles: number, fouls: number, yellows: number, reds: number, corners: number, offsides: number, saves: number, };

export type Tie = { 
/**
 * Home side of the first leg first.
 */
clubs: [ClubRef, ClubRef], legs: Array<Fixture>, 
/**
 * Goals over both legs, in `clubs` order, once a two-legged tie has
 * started.
 */
aggregate: [number, number] | null, winner: number | null, };

export type TrainingFocus = "balanced" | "fitness" | "attacking" | "defending" | "tactics" | "recovery";

export type TrainingIntensity = "light" | "normal" | "hard";

export type TrainingPlayer = { player: PlayerRef, age: number, individual: IndividualTraining | null, 
/**
 * For position training: familiarity with that position, 0-20.
 */
progress: number | null, 
/**
 * 0-100.
 */
fitness: number, injured: boolean, };

export type TrainingView = { focus: TrainingFocus, intensity: TrainingIntensity, focuses: Array<FocusInfo>, intensities: Array<IntensityInfo>, 
/**
 * The schedule's effects against balanced, normal training, in
 * percent: player growth, daily recovery, training injuries.
 */
growth: number, recovery: number, injuryRisk: number, 
/**
 * Familiarity gained per week in the current formation, 0-100 scale.
 */
drillPerWeek: number, familiarity: Array<FormationFamiliarity>, players: Array<TrainingPlayer>, 
/**
 * Position codes and attribute names for individual training.
 */
positions: Array<string>, attributes: Array<string>, };

export type TransferRecord = { date: string, player: PlayerRef, from: ClubRef, to: ClubRef, fee: number, loan: boolean, };

export type TransfersView = { windowOpen: boolean, 
/**
 * When the window next opens or closes.
 */
windowChanges: string, transferBudget: number, 
/**
 * Offers for the viewer's players, newest first.
 */
incoming: Array<Offer>, 
/**
 * The viewer's offers, newest first.
 */
outgoing: Array<Offer>, 
/**
 * The viewer's listed players.
 */
listed: Array<ListedPlayer>, loansOut: Array<Loan>, loansIn: Array<Loan>, scouting: Array<Assignment>, 
/**
 * Scouts available to watch players at once.
 */
scouts: number, 
/**
 * The biggest recent moves in the world.
 */
recent: Array<TransferRecord>, };

export type WorldSize = "tiny" | "small" | "medium";

export type XgPoint = { minute: number, home: number, away: number, };
