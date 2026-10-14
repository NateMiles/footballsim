/**
 * Hash routes, so the back button, reloads and bookmarks work everywhere
 * (including inside the Tauri shell and as an installed web app).
 */
export type Route =
  | { name: 'home' }
  | { name: 'inbox' }
  | { name: 'squad'; club?: number }
  | { name: 'player'; id: number }
  | { name: 'talk'; id: number }
  | { name: 'tactics' }
  | { name: 'fixtures'; league?: number }
  | { name: 'table'; league?: number }
  | { name: 'cup'; cup?: number }
  | { name: 'competitions' }
  | { name: 'prepare' }
  | { name: 'training' }
  | { name: 'dynamics' }
  | { name: 'news' }
  | { name: 'press' }
  | { name: 'stats' }
  | { name: 'analysis'; kind: 'league' | 'cup'; id: number; index: number }
  | { name: 'match' }
  | { name: 'saves' }
  | { name: 'transfers'; tab?: TransferTab }
  | { name: 'staff' }
  | { name: 'finances' }
  | { name: 'jobs' };

export type TransferTab = 'search' | 'offers' | 'scouting' | 'squad' | 'news';
const TABS: TransferTab[] = ['search', 'offers', 'scouting', 'squad', 'news'];

export function parse(hash: string): Route {
  const [name, arg, ...rest] = hash.replace(/^#\/?/, '').split('/');
  const n = arg === undefined || arg === '' ? undefined : Number(arg);
  const id = n !== undefined && Number.isInteger(n) && n >= 0 ? n : undefined;
  switch (name) {
    case 'inbox':
    case 'tactics':
    case 'match':
    case 'saves':
    case 'staff':
    case 'finances':
    case 'jobs':
    case 'competitions':
    case 'prepare':
    case 'training':
    case 'dynamics':
    case 'stats':
    case 'news':
    case 'press':
      return { name };
    case 'analysis': {
      const [id, index] = rest.map(Number);
      if ((arg === 'league' || arg === 'cup') && Number.isInteger(id) && Number.isInteger(index)) {
        return { name, kind: arg, id, index };
      }
      return { name: 'home' };
    }
    case 'cup':
      return { name, cup: id };
    case 'transfers':
      return { name, tab: TABS.find((t) => t === arg) };
    case 'squad':
      return { name, club: id };
    case 'player':
    case 'talk':
      return id === undefined ? { name: 'home' } : { name, id };
    case 'fixtures':
    case 'table':
      return { name, league: id };
    default:
      return { name: 'home' };
  }
}

export function href(route: Route): string {
  switch (route.name) {
    case 'player':
    case 'talk':
      return `#/${route.name}/${route.id}`;
    case 'squad':
      return route.club === undefined ? '#/squad' : `#/squad/${route.club}`;
    case 'fixtures':
    case 'table':
      return route.league === undefined ? `#/${route.name}` : `#/${route.name}/${route.league}`;
    case 'transfers':
      return route.tab === undefined ? '#/transfers' : `#/transfers/${route.tab}`;
    case 'cup':
      return route.cup === undefined ? '#/cup' : `#/cup/${route.cup}`;
    case 'analysis':
      return `#/analysis/${route.kind}/${route.id}/${route.index}`;
    default:
      return `#/${route.name}`;
  }
}

export class Router {
  route = $state<Route>(parse(location.hash));

  constructor() {
    addEventListener('hashchange', () => {
      this.route = parse(location.hash);
    });
  }

  go(route: Route): void {
    const target = href(route);
    if (location.hash === target) this.route = parse(target);
    else location.hash = target;
  }
}

/** The analysis screen for a fixture. */
export function analysisHref(f: { competition: { kind: 'league' | 'cup'; id: number }; index: number }): string {
  return href({ name: 'analysis', kind: f.competition.kind, id: f.competition.id, index: f.index });
}
