/**
 * Hash routes, so the back button, reloads and bookmarks work everywhere
 * (including inside the Tauri shell and as an installed web app).
 */
export type Route =
  | { name: 'home' }
  | { name: 'inbox' }
  | { name: 'squad'; club?: number }
  | { name: 'player'; id: number }
  | { name: 'tactics' }
  | { name: 'fixtures'; league?: number }
  | { name: 'table'; league?: number }
  | { name: 'cup'; cup?: number }
  | { name: 'competitions' }
  | { name: 'match' }
  | { name: 'saves' }
  | { name: 'transfers'; tab?: TransferTab }
  | { name: 'staff' }
  | { name: 'finances' }
  | { name: 'jobs' };

export type TransferTab = 'search' | 'offers' | 'scouting' | 'squad' | 'news';
const TABS: TransferTab[] = ['search', 'offers', 'scouting', 'squad', 'news'];

export function parse(hash: string): Route {
  const [name, arg] = hash.replace(/^#\/?/, '').split('/');
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
      return { name };
    case 'cup':
      return { name, cup: id };
    case 'transfers':
      return { name, tab: TABS.find((t) => t === arg) };
    case 'squad':
      return { name, club: id };
    case 'player':
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
      return `#/player/${route.id}`;
    case 'squad':
      return route.club === undefined ? '#/squad' : `#/squad/${route.club}`;
    case 'fixtures':
    case 'table':
      return route.league === undefined ? `#/${route.name}` : `#/${route.name}/${route.league}`;
    case 'transfers':
      return route.tab === undefined ? '#/transfers' : `#/transfers/${route.tab}`;
    case 'cup':
      return route.cup === undefined ? '#/cup' : `#/cup/${route.cup}`;
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
