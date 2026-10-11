// Generated from crates/fs-api by tests/bindings.rs. Do not edit.
// Regenerate: UPDATE_BINDINGS=1 cargo test -p fs-api --test bindings

export const PROTOCOL_VERSION = 3;

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

export type Category = "match" | "injury" | "contract" | "transfer" | "youth" | "season" | "club" | "finance" | "board" | "staff" | "scouting" | "cup";

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

export type CompetitionKind = "league" | "cup";

export type CompetitionRef = { kind: CompetitionKind, 
/**
 * A league or cup ID, by `kind`.
 */
id: number, name: string, shortName: string, };

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
wage: number, expires: string, };

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

export type Formation = "4-4-2" | "4-3-3" | "4-2-3-1" | "3-5-2" | "5-3-2" | "4-1-4-1";

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

export type Injury = { 
/**
 * `"minor"`, `"moderate"`, `"major"` or `"serious"`.
 */
severity: string, until: string, };

export type Instruction = { "type": "substitute", off: number, on: number, } | { "type": "changeTactic", tactic: Tactic, } | { "type": "shout", shout: Shout, };

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

export type Kick = { side: Side, taker: number, scored: boolean, };

export type LeagueRef = { id: number, name: string, nation: string, 
/**
 * 1 for a nation's top division.
 */
level: number, };

export type Ledger = { matchday: number, broadcast: number, commercial: number, prizeMoney: number, playerSales: number, playerWages: number, staffWages: number, playerPurchases: number, runningCosts: number, investment: number, income: number, expenses: number, };

export type Lineup = { 
/**
 * Eleven players by formation slot.
 */
starters: Array<number>, bench: Array<number>, };

export type Link = { "type": "player", player: number, } | { "type": "schedule" } | { "type": "table", league: number, } | { "type": "cup", cup: number, } | { "type": "squad" } | { "type": "tactics" } | { "type": "transfers" } | { "type": "staff" } | { "type": "jobs" } | { "type": "finances" };

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

export type NationCompetitions = { nation: string, code: string, leagues: Array<LeagueRef>, cups: Array<CupRef>, };

export type Offer = { id: number, date: string, player: PlayerRef, buyer: ClubRef, seller: ClubRef, kind: OfferKind, 
/**
 * The contract offered to the player (transfers).
 */
wage: number, years: number, status: OfferStatus, respondBy: string, };

export type OfferKind = { "type": "transfer", fee: number, } | { "type": "loan", wageShare: number, };

export type OfferResponse = { "type": "accept" } | { "type": "reject" } | { "type": "counter", fee: number, };

export type OfferStatus = { "type": "pending" } | { "type": "countered", fee: number, } | { "type": "agreed" } | { "type": "completed" } | { "type": "refused", reason: string, };

export type OnPitch = { id: number, slot: number, 
/**
 * 0-1.
 */
energy: number, yellow: boolean, };

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

export type Pitch = { length: number, width: number, };

export type PlayerFrame = { id: number, side: Side, x: number, y: number, 
/**
 * Radians; 0 faces the away goal.
 */
facing: number, pose: Pose, };

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
own: boolean, };

export type Pose = "standing" | "running" | "passing" | "shooting" | "heading" | "tackling" | "diving" | "celebrating";

export type PositionInfo = { code: string, name: string, };

export type Request = { "type": "hello" } | { "type": "newWorld", seed: number, size: WorldSize, } | { "type": "startCareer", club: number, firstName: string, lastName: string, } | { "type": "save" } | { "type": "load", data: string, } | { "type": "overview" } | { "type": "inbox" } | { "type": "markRead", id: number | null, } | { "type": "squad", club: number | null, } | { "type": "player", player: number, } | { "type": "tactics" } | { "type": "setTactic", tactic: Tactic | null, } | { "type": "setLineup", lineup: Lineup | null, } | { "type": "fixtures", league: number | null, club: number | null, } | { "type": "table", league: number | null, } | { "type": "schedule", club: number | null, } | { "type": "cup", cup: number | null, } | { "type": "competitions" } | { "type": "setWatchMatches", watch: boolean, } | { "type": "setDelegation", squad: boolean, } | { "type": "offerContract", player: number, wage: number, years: number, } | { "type": "releasePlayer", player: number, } | { "type": "continue" } | { "type": "startMatch" } | { "type": "liveTick", until: number, } | { "type": "liveInstruct", instruction: Instruction, } | { "type": "finishMatch" } | { "type": "skipMatch" } | { "type": "finances" } | { "type": "staff" } | { "type": "hireStaff", staff: number, wage: number, years: number, } | { "type": "releaseStaff", staff: number, } | { "type": "transfers" } | { "type": "searchPlayers", search: PlayerSearch, } | { "type": "makeOffer", player: number, kind: OfferKind, wage: number, years: number, } | { "type": "respondToOffer", offer: number, response: OfferResponse, } | { "type": "acceptCounter", offer: number, } | { "type": "withdrawOffer", offer: number, } | { "type": "setListing", player: number, listing: Listing | null, } | { "type": "scoutPlayer", player: number, } | { "type": "cancelScouting", player: number, } | { "type": "jobs" } | { "type": "applyForJob", club: number, } | { "type": "resign" } | { "type": "benchmarkStart", size: WorldSize, } | { "type": "benchmarkRun", days: number, } | { "type": "benchmarkEnd" };

export type Response = { "type": "hello", protocol: number, version: string, 
/**
 * Whether a game is loaded.
 */
inGame: boolean, } | { "type": "clubChoices", choices: ClubChoices, } | { "type": "saved", save: SaveGame, } | { "type": "overview", overview: Overview, } | { "type": "inbox", inbox: InboxView, } | { "type": "squad", squad: SquadView, } | { "type": "player", player: PlayerView, } | { "type": "tactics", tactics: TacticsView, } | { "type": "fixtures", fixtures: FixturesView, } | { "type": "table", table: TableView, } | { "type": "schedule", schedule: ScheduleView, } | { "type": "cup", cup: CupView, } | { "type": "competitions", competitions: CompetitionsView, } | { "type": "continued", continued: Continued, } | { "type": "liveStart", start: LiveStart, } | { "type": "liveChunk", chunk: LiveChunk, } | { "type": "matchReport", report: MatchReport, } | { "type": "finances", finances: FinancesView, } | { "type": "staff", staff: StaffView, } | { "type": "transfers", transfers: TransfersView, } | { "type": "search", search: SearchView, } | { "type": "jobs", jobs: JobsView, } | { "type": "benchmark", benchmark: BenchmarkStatus, } | { "type": "error", error: ApiErrorBody, };

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

export type SelectablePlayer = { id: number, name: string, shortName: string, number: number | null, position: string, available: boolean, fitness: number, 
/**
 * How well they would play each position today, 0-100, in
 * [`PositionInfo`] order.
 */
suitability: Array<number>, };

export type Selection = "starter" | "bench" | "reserve";

export type Shootout = { kicks: Array<Kick>, 
/**
 * Home then away.
 */
score: [number, number], };

export type Shout = "encourage" | "demandMore" | "calmDown" | "tightenUp" | "pushForward";

export type Side = "home" | "away";

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
starters: Array<number>, bench: Array<number>, players: Array<SelectablePlayer>, formations: Array<FormationInfo>, positions: Array<PositionInfo>, sliders: Array<SliderInfo>, benchSize: number, };

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
