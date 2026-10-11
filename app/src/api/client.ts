import type {
  ApiErrorBody,
  ClubChoices,
  Continued,
  ErrorCode,
  FinancesView,
  FixturesView,
  InboxView,
  Instruction,
  JobsView,
  Lineup,
  Listing,
  OfferKind,
  OfferResponse,
  PlayerSearch,
  SearchView,
  StaffView,
  TransfersView,
  LiveChunk,
  LiveStart,
  MatchReport,
  Overview,
  PlayerView,
  Request,
  Response,
  SaveGame,
  SquadView,
  Tactic,
  TableView,
  TacticsView,
  WorldSize,
} from './generated';
import { PROTOCOL_VERSION } from './generated';
import type { Transport } from './transport';

type ResponseOf<T extends Response['type']> = Extract<Response, { type: T }>;

/** A request the core refused. */
export class ApiError extends Error {
  readonly code: ErrorCode;

  constructor(body: ApiErrorBody) {
    super(body.message);
    this.name = 'ApiError';
    this.code = body.code;
  }
}

/**
 * Typed access to the core. One method per request; each resolves with
 * the view the request is answered with, or rejects with an [`ApiError`].
 */
export class Client {
  private queue: Promise<unknown> = Promise.resolve();

  constructor(private readonly transport: Transport) {}

  /**
   * Sends a request and checks the response type. Requests are sent one
   * at a time, in order, so a slow one (a week of simulation) never races
   * a later one.
   */
  async send<T extends Response['type']>(request: Request, expect: T): Promise<ResponseOf<T>> {
    const run = async () => {
      const json = await this.transport.call(JSON.stringify(request));
      const response = JSON.parse(json) as Response;
      if (response.type === 'error') throw new ApiError(response.error);
      if (response.type !== expect) {
        throw new Error(`Expected a ${expect} response to ${request.type}, got ${response.type}`);
      }
      return response as ResponseOf<T>;
    };
    const result = this.queue.then(run, run);
    this.queue = result.catch(() => undefined);
    return result;
  }

  async hello(): Promise<{ inGame: boolean; version: string }> {
    const r = await this.send({ type: 'hello' }, 'hello');
    if (r.protocol !== PROTOCOL_VERSION) {
      throw new Error(`The core speaks protocol ${r.protocol}; this UI needs ${PROTOCOL_VERSION}.`);
    }
    return { inGame: r.inGame, version: r.version };
  }

  async newWorld(seed: number, size: WorldSize): Promise<ClubChoices> {
    return (await this.send({ type: 'newWorld', seed, size }, 'clubChoices')).choices;
  }

  async startCareer(club: number, firstName: string, lastName: string): Promise<Overview> {
    return (await this.send({ type: 'startCareer', club, firstName, lastName }, 'overview')).overview;
  }

  async save(): Promise<SaveGame> {
    return (await this.send({ type: 'save' }, 'saved')).save;
  }

  async load(data: string): Promise<Overview> {
    return (await this.send({ type: 'load', data }, 'overview')).overview;
  }

  async overview(): Promise<Overview> {
    return (await this.send({ type: 'overview' }, 'overview')).overview;
  }

  async inbox(): Promise<InboxView> {
    return (await this.send({ type: 'inbox' }, 'inbox')).inbox;
  }

  async markRead(id: number | null): Promise<InboxView> {
    return (await this.send({ type: 'markRead', id }, 'inbox')).inbox;
  }

  async squad(club: number | null = null): Promise<SquadView> {
    return (await this.send({ type: 'squad', club }, 'squad')).squad;
  }

  async player(player: number): Promise<PlayerView> {
    return (await this.send({ type: 'player', player }, 'player')).player;
  }

  async tactics(): Promise<TacticsView> {
    return (await this.send({ type: 'tactics' }, 'tactics')).tactics;
  }

  async setTactic(tactic: Tactic | null): Promise<TacticsView> {
    return (await this.send({ type: 'setTactic', tactic }, 'tactics')).tactics;
  }

  async setLineup(lineup: Lineup | null): Promise<TacticsView> {
    return (await this.send({ type: 'setLineup', lineup }, 'tactics')).tactics;
  }

  async fixtures(league: number | null = null, club: number | null = null): Promise<FixturesView> {
    return (await this.send({ type: 'fixtures', league, club }, 'fixtures')).fixtures;
  }

  async table(league: number | null = null): Promise<TableView> {
    return (await this.send({ type: 'table', league }, 'table')).table;
  }

  async setWatchMatches(watch: boolean): Promise<Overview> {
    return (await this.send({ type: 'setWatchMatches', watch }, 'overview')).overview;
  }

  async setDelegation(squad: boolean): Promise<Overview> {
    return (await this.send({ type: 'setDelegation', squad }, 'overview')).overview;
  }

  async offerContract(player: number, wage: number, years: number): Promise<PlayerView> {
    return (await this.send({ type: 'offerContract', player, wage, years }, 'player')).player;
  }

  async releasePlayer(player: number): Promise<SquadView> {
    return (await this.send({ type: 'releasePlayer', player }, 'squad')).squad;
  }

  async continueOn(): Promise<Continued> {
    return (await this.send({ type: 'continue' }, 'continued')).continued;
  }

  async startMatch(): Promise<LiveStart> {
    return (await this.send({ type: 'startMatch' }, 'liveStart')).start;
  }

  async liveTick(until: number): Promise<LiveChunk> {
    return (await this.send({ type: 'liveTick', until }, 'liveChunk')).chunk;
  }

  async instruct(instruction: Instruction): Promise<LiveChunk> {
    return (await this.send({ type: 'liveInstruct', instruction }, 'liveChunk')).chunk;
  }

  async finishMatch(): Promise<MatchReport> {
    return (await this.send({ type: 'finishMatch' }, 'matchReport')).report;
  }

  async skipMatch(): Promise<Overview> {
    return (await this.send({ type: 'skipMatch' }, 'overview')).overview;
  }

  async finances(): Promise<FinancesView> {
    return (await this.send({ type: 'finances' }, 'finances')).finances;
  }

  async staff(): Promise<StaffView> {
    return (await this.send({ type: 'staff' }, 'staff')).staff;
  }

  async hireStaff(staff: number, wage: number, years: number): Promise<StaffView> {
    return (await this.send({ type: 'hireStaff', staff, wage, years }, 'staff')).staff;
  }

  async releaseStaff(staff: number): Promise<StaffView> {
    return (await this.send({ type: 'releaseStaff', staff }, 'staff')).staff;
  }

  async transfers(): Promise<TransfersView> {
    return (await this.send({ type: 'transfers' }, 'transfers')).transfers;
  }

  async searchPlayers(search: PlayerSearch): Promise<SearchView> {
    return (await this.send({ type: 'searchPlayers', search }, 'search')).search;
  }

  async makeOffer(player: number, kind: OfferKind, wage: number, years: number): Promise<TransfersView> {
    return (await this.send({ type: 'makeOffer', player, kind, wage, years }, 'transfers')).transfers;
  }

  async respondToOffer(offer: number, response: OfferResponse): Promise<TransfersView> {
    return (await this.send({ type: 'respondToOffer', offer, response }, 'transfers')).transfers;
  }

  async acceptCounter(offer: number): Promise<TransfersView> {
    return (await this.send({ type: 'acceptCounter', offer }, 'transfers')).transfers;
  }

  async withdrawOffer(offer: number): Promise<TransfersView> {
    return (await this.send({ type: 'withdrawOffer', offer }, 'transfers')).transfers;
  }

  async setListing(player: number, listing: Listing | null): Promise<PlayerView> {
    return (await this.send({ type: 'setListing', player, listing }, 'player')).player;
  }

  async scoutPlayer(player: number): Promise<PlayerView> {
    return (await this.send({ type: 'scoutPlayer', player }, 'player')).player;
  }

  async cancelScouting(player: number): Promise<PlayerView> {
    return (await this.send({ type: 'cancelScouting', player }, 'player')).player;
  }

  async jobs(): Promise<JobsView> {
    return (await this.send({ type: 'jobs' }, 'jobs')).jobs;
  }

  async applyForJob(club: number): Promise<Overview> {
    return (await this.send({ type: 'applyForJob', club }, 'overview')).overview;
  }

  async resign(): Promise<Overview> {
    return (await this.send({ type: 'resign' }, 'overview')).overview;
  }

  close(): void {
    this.transport.close();
  }
}
