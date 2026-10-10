// Generated from crates/fs-api by tests/bindings.rs. Do not edit.
// Regenerate: UPDATE_BINDINGS=1 cargo test -p fs-api --test bindings

export const PROTOCOL_VERSION = 1;

export type ApiErrorBody = { code: ErrorCode, 
/**
 * For people.
 */
message: string, };

export type AttributeGroup = { name: string, attributes: Array<AttributeValue>, };

export type AttributeValue = { name: string, value: number, };

export type CareerRow = { 
/**
 * Such as `"2026/27"`.
 */
season: string, club: ClubRef | null, ability: number, stats: SeasonStats, };

export type Category = "match" | "injury" | "contract" | "transfer" | "youth" | "season" | "club";

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

export type Colors = { primary: string, secondary: string, };

export type Commentary = { 
/**
 * Seconds since kickoff.
 */
t: number, clock: string, kind: CommentaryKind, side: Side | null, text: string, };

export type CommentaryKind = "goal" | "chance" | "save" | "card" | "substitution" | "injury" | "foul" | "setPiece" | "play" | "whistle";

export type Continued = { date: string, stopped: StopReason, newMessages: number, matchDay: MatchDay | null, };

export type Contract = { 
/**
 * Per week.
 */
wage: number, expires: string, };

export type ErrorCode = "badRequest" | "noGame" | "notFound" | "rejected" | "noMatch" | "badSave";

export type Fixture = { league: number, 
/**
 * Index into the league season's fixtures.
 */
index: number, date: string, home: ClubRef, away: ClubRef, result: FixtureResult | null, };

export type FixtureResult = { goals: [number, number], xg: [number, number], scorers: Array<Scorer>, };

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

export type InboxView = { messages: Array<Message>, unread: number, };

export type Injury = { 
/**
 * `"minor"`, `"moderate"`, `"major"` or `"serious"`.
 */
severity: string, until: string, };

export type Instruction = { "type": "substitute", off: number, on: number, } | { "type": "changeTactic", tactic: Tactic, } | { "type": "shout", shout: Shout, };

export type LeagueRef = { id: number, name: string, nation: string, 
/**
 * 1 for a nation's top division.
 */
level: number, };

export type Lineup = { 
/**
 * Eleven players by formation slot.
 */
starters: Array<number>, bench: Array<number>, };

export type Link = { "type": "player", player: number, } | { "type": "fixture", league: number, index: number, } | { "type": "table", league: number, } | { "type": "squad" } | { "type": "tactics" };

export type LiveChunk = { frames: Array<Frame>, commentary: Array<Commentary>, events: Array<MatchEvent>, score: [number, number], 
/**
 * Such as `"45+2"`.
 */
clock: string, 
/**
 * Seconds since kickoff, counting stoppage time.
 */
elapsed: number, finished: boolean, stats: [TeamStats, TeamStats], lineups: [LiveLineup, LiveLineup], };

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
ratings: [Array<PlayerRating>, Array<PlayerRating>], };

export type Message = { id: number, date: string, category: Category, title: string, body: string, read: boolean, link: Link | null, };

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
season: string, manager: ManagerInfo, club: ClubRef, league: LeagueRef | null, position: number | null, nextFixture: Fixture | null, lastFixture: Fixture | null, 
/**
 * Last five league results, oldest first: `"W"`, `"D"` or `"L"`.
 */
form: Array<string>, unread: number, 
/**
 * Today's match, waiting to be watched.
 */
matchDay: MatchDay | null, watchMatches: boolean, delegateSquad: boolean, };

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

export type PlayerView = { id: number, name: string, firstName: string, lastName: string, age: number, birthDate: string, nation: string, club: ClubRef | null, number: number | null, 
/**
 * `"left"`, `"right"` or `"both"`.
 */
foot: string, heightCm: number, weightKg: number, position: string, 
/**
 * Familiarity with every position, 0-20, in [`PositionInfo`] order.
 */
positionRatings: Array<number>, attributes: Array<AttributeGroup>, ability: number, potential: number, fitness: number, sharpness: number, morale: number, injury: Injury | null, suspension: number, contract: Contract | null, 
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

export type Request = { "type": "hello" } | { "type": "newWorld", seed: number, size: WorldSize, } | { "type": "startCareer", club: number, firstName: string, lastName: string, } | { "type": "save" } | { "type": "load", data: string, } | { "type": "overview" } | { "type": "inbox" } | { "type": "markRead", id: number | null, } | { "type": "squad", club: number | null, } | { "type": "player", player: number, } | { "type": "tactics" } | { "type": "setTactic", tactic: Tactic | null, } | { "type": "setLineup", lineup: Lineup | null, } | { "type": "fixtures", league: number | null, club: number | null, } | { "type": "table", league: number | null, } | { "type": "setWatchMatches", watch: boolean, } | { "type": "setDelegation", squad: boolean, } | { "type": "offerContract", player: number, wage: number, years: number, } | { "type": "releasePlayer", player: number, } | { "type": "continue" } | { "type": "startMatch" } | { "type": "liveTick", until: number, } | { "type": "liveInstruct", instruction: Instruction, } | { "type": "finishMatch" } | { "type": "skipMatch" };

export type Response = { "type": "hello", protocol: number, version: string, 
/**
 * Whether a game is loaded.
 */
inGame: boolean, } | { "type": "clubChoices", choices: ClubChoices, } | { "type": "saved", save: SaveGame, } | { "type": "overview", overview: Overview, } | { "type": "inbox", inbox: InboxView, } | { "type": "squad", squad: SquadView, } | { "type": "player", player: PlayerView, } | { "type": "tactics", tactics: TacticsView, } | { "type": "fixtures", fixtures: FixturesView, } | { "type": "table", table: TableView, } | { "type": "continued", continued: Continued, } | { "type": "liveStart", start: LiveStart, } | { "type": "liveChunk", chunk: LiveChunk, } | { "type": "matchReport", report: MatchReport, } | { "type": "error", error: ApiErrorBody, };

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

export type Scorer = { side: Side, player: PlayerRef, };

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

export type SquadView = { club: ClubRef, players: Array<SquadPlayer>, };

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

export type TableZone = "champion" | "promotion" | "relegation";

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

export type WorldSize = "tiny" | "small" | "medium";
