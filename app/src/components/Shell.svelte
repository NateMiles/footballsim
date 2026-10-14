<!-- The frame around every screen: the navigation rail (a tab bar on
     narrow screens), the date and Continue. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { useGame } from '../state/game.svelte';
  import { href, type Route } from '../state/router.svelte';
  import Crest from './Crest.svelte';
  import Icon, { type IconName } from './Icon.svelte';
  import Portrait from './Portrait.svelte';

  let { children }: { children: Snippet } = $props();
  const game = useGame();
  const o = $derived(game.overview);

  type Item = { route: Route; label: string; icon: IconName };
  type Group = { title: string; items: Item[] };
  const employed: Group[] = [
    {
      title: 'Club',
      items: [
        { route: { name: 'home' }, label: 'Home', icon: 'home' },
        { route: { name: 'inbox' }, label: 'Inbox', icon: 'inbox' },
        { route: { name: 'squad' }, label: 'Squad', icon: 'squad' },
        { route: { name: 'tactics' }, label: 'Tactics', icon: 'tactics' },
        { route: { name: 'training' }, label: 'Training', icon: 'training' },
        { route: { name: 'dynamics' }, label: 'Dressing room', icon: 'dressing' },
        { route: { name: 'press' }, label: 'Press room', icon: 'mic' },
      ],
    },
    {
      title: 'Football',
      items: [
        { route: { name: 'news' }, label: 'News', icon: 'news' },
        { route: { name: 'fixtures' }, label: 'Fixtures', icon: 'fixtures' },
        { route: { name: 'table' }, label: 'Table', icon: 'table' },
        { route: { name: 'stats' }, label: 'Stats', icon: 'stats' },
        { route: { name: 'cup' }, label: 'Cups', icon: 'cups' },
        { route: { name: 'competitions' }, label: 'Competitions', icon: 'competitions' },
      ],
    },
    {
      title: 'Office',
      items: [
        { route: { name: 'transfers' }, label: 'Transfers', icon: 'transfers' },
        { route: { name: 'staff' }, label: 'Staff', icon: 'staff' },
        { route: { name: 'finances' }, label: 'Finances', icon: 'finances' },
        { route: { name: 'saves' }, label: 'Saves', icon: 'saves' },
      ],
    },
  ];
  const unemployed: Group[] = [
    {
      title: 'Career',
      items: [
        { route: { name: 'home' }, label: 'Home', icon: 'home' },
        { route: { name: 'inbox' }, label: 'Inbox', icon: 'inbox' },
        { route: { name: 'jobs' }, label: 'Jobs', icon: 'jobs' },
        { route: { name: 'news' }, label: 'News', icon: 'news' },
        { route: { name: 'competitions' }, label: 'Competitions', icon: 'competitions' },
        { route: { name: 'saves' }, label: 'Saves', icon: 'saves' },
      ],
    },
  ];
  const groups = $derived(o?.club ? employed : unemployed);
  const items = $derived(groups.flatMap((g) => g.items));
  /** On phones and portrait iPad: the first four as tabs, then "More". */
  const PRIMARY = 4;
  let more = $state(false);
  const current = $derived(game.router.route.name);
  const active = (item: Item) =>
    current === item.route.name ||
    (item.route.name === 'squad' && current === 'player') ||
    (item.route.name === 'dynamics' && current === 'talk') ||
    (item.route.name === 'stats' && current === 'analysis') ||
    (item.route.name === 'fixtures' && current === 'prepare');

  const dateParts = $derived.by(() => {
    if (!o) return null;
    const d = new Date(`${o.date}T12:00:00Z`);
    return {
      day: d.getUTCDate(),
      month: d.toLocaleString('en-GB', { month: 'short', timeZone: 'UTC' }),
      year: d.getUTCFullYear(),
    };
  });
</script>

{#snippet link(item: Item, tab: boolean)}
  <a
    href={href(item.route)}
    class="nav-item"
    class:active={active(item)}
    class:tab
    aria-current={active(item) ? 'page' : undefined}
    onclick={() => (more = false)}
  >
    <Icon name={item.icon} />
    <span class="label">{item.label}</span>
    {#if item.route.name === 'inbox' && o && o.unread > 0}
      <span class="badge">{o.unread}</span>
    {:else if item.route.name === 'press' && o && o.pressWaiting > 0}
      <span class="badge">{o.pressWaiting}</span>
    {/if}
  </a>
{/snippet}

<div class="shell">
  <aside class="rail" aria-label="Main">
    <a class="brand" href="#/home" aria-label="footballsim home">
      <span class="mark"><Icon name="ball" size={18} stroke={1.6} /></span>
      <span class="word">football<b>sim</b></span>
    </a>
    <nav>
      {#each groups as g (g.title)}
        <div class="group">
          <span class="group-title">{g.title}</span>
          {#each g.items as item (item.label)}
            {@render link(item, false)}
          {/each}
        </div>
      {/each}
    </nav>
    {#if o}
      <div class="manager">
        <span class="avatar"><Portrait look={o.manager.look} size={38} ring shirt="#1f2a3a" trim="#d4ff3f" /></span>
        <span class="who">
          <strong>{o.manager.name}</strong>
          <span class="dim">{o.season}</span>
        </span>
      </div>
    {/if}
  </aside>

  <header class="topbar">
    {#if o}
      <a class="club" href="#/home">
        {#if o.club}
          <Crest club={o.club} size={34} />
          <span class="club-text">
            <span class="name">{o.club.name}</span>
            {#if o.league}<span class="league">{o.league.name}</span>{/if}
          </span>
        {:else}
          <span class="name">{o.manager.name}</span>
          <span class="pill">Out of work</span>
        {/if}
      </a>
    {/if}
    <span class="spacer"></span>
    {#if game.busy}
      <span class="busy" role="status"><span class="spinner"></span>{game.busy}…</span>
    {/if}
    {#if o && dateParts}
      <span class="ticket" title={o.weekday}>
        <span class="t-day">{dateParts.day}</span>
        <span class="t-rest"><span>{dateParts.month}</span><span class="dim">{o.weekday.slice(0, 3)} · {dateParts.year}</span></span>
      </span>
    {/if}
    <button class="primary continue" class:matchday={!!o?.matchDay} disabled={game.busy !== null} onclick={() => game.continueOn()}>
      <span>{o?.matchDay ? 'Match day' : 'Continue'}</span>
      <Icon name="arrow" size={18} stroke={2.2} />
    </button>
  </header>

  <nav class="tabbar" aria-label="Sections">
    {#each items.slice(0, PRIMARY) as item (item.label)}
      {@render link(item, true)}
    {/each}
    {#if items.length > PRIMARY}
      <button
        class="nav-item tab more"
        class:active={more || items.slice(PRIMARY).some(active)}
        onclick={() => (more = !more)}
        aria-expanded={more}
      >
        <Icon name="more" />
        <span class="label">More</span>
      </button>
    {/if}
  </nav>
  {#if more}
    <button class="scrim" aria-label="Close menu" onclick={() => (more = false)}></button>
    <div class="sheet" role="menu">
      {#each items.slice(PRIMARY) as item (item.label)}
        <a role="menuitem" href={href(item.route)} class:active={active(item)} onclick={() => (more = false)}>
          <Icon name={item.icon} />
          {item.label}
        </a>
      {/each}
    </div>
  {/if}

  <main>
    {#if game.error}
      <div class="error" role="alert">
        {game.error}
        <button class="link" onclick={() => (game.error = null)}>Dismiss</button>
      </div>
    {/if}
    {@render children()}
  </main>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 236px 1fr;
    grid-template-rows: auto 1fr;
    grid-template-areas: 'rail header' 'rail main';
    height: 100%;
  }

  /* The rail. */
  .rail {
    grid-area: rail;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: max(18px, env(safe-area-inset-top)) 14px 14px;
    border-right: 1px solid var(--line);
    background: linear-gradient(180deg, rgba(16, 23, 34, 0.85), rgba(7, 11, 17, 0.6));
    overflow-y: auto;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 2px 8px 14px;
    color: var(--text);
  }

  .mark {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 10px;
    background: var(--accent);
    color: var(--accent-ink);
    box-shadow: 0 6px 18px -6px var(--accent-glow);
  }

  .word {
    font-family: var(--display);
    font-stretch: 115%;
    font-weight: 600;
    font-size: 1.1rem;
    letter-spacing: -0.01em;
  }

  .word b {
    font-weight: 900;
    color: var(--accent);
  }

  .rail nav {
    display: grid;
    gap: 16px;
  }

  .group {
    display: grid;
    gap: 2px;
  }

  .group-title {
    padding: 0 12px 4px;
    font-family: var(--display);
    font-stretch: 80%;
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.18em;
    color: var(--text-faint);
  }

  .nav-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 40px;
    padding: 0 12px;
    border-radius: 10px;
    color: var(--text-dim);
    font-weight: 500;
    transition:
      background 0.15s,
      color 0.15s;
  }

  .nav-item:hover {
    color: var(--text);
    background: var(--bg-hover);
  }

  .nav-item.active {
    color: var(--text);
    background: linear-gradient(90deg, var(--accent-soft), rgba(212, 255, 63, 0.02));
  }

  .nav-item.active::before {
    content: '';
    position: absolute;
    left: -14px;
    top: 9px;
    bottom: 9px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--accent);
    box-shadow: 0 0 12px var(--accent-glow);
  }

  .nav-item.active :global(.icon) {
    color: var(--accent);
  }

  .badge {
    margin-left: auto;
    min-width: 20px;
    padding: 0 6px;
    border-radius: 10px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 0.72rem;
    font-weight: 800;
    line-height: 20px;
    text-align: center;
  }

  .manager {
    margin-top: auto;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 8px 4px;
    border-top: 1px solid var(--line);
  }

  .avatar {
    display: grid;
    place-items: center;
    border-radius: 50%;
    box-shadow: 0 0 0 1px var(--line-strong);
  }

  .who {
    display: grid;
    line-height: 1.25;
    font-size: 0.85rem;
  }

  /* The top bar. */
  .topbar {
    grid-area: header;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 24px;
    padding-top: max(12px, env(safe-area-inset-top));
    border-bottom: 1px solid var(--line);
    background: rgba(7, 11, 17, 0.55);
    backdrop-filter: blur(14px) saturate(140%);
    -webkit-backdrop-filter: blur(14px) saturate(140%);
    z-index: 5;
  }

  .club {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--text);
    min-width: 0;
  }

  .club-text {
    display: grid;
    line-height: 1.2;
    min-width: 0;
  }

  .name {
    font-family: var(--display);
    font-stretch: 108%;
    font-weight: 800;
    font-size: 1.05rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .league {
    font-size: 0.78rem;
    color: var(--text-dim);
  }

  .busy {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-dim);
    font-size: 0.88rem;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid var(--line-strong);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* The date as a matchday ticket. */
  .ticket {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 14px 4px 6px;
    border: 1px dashed var(--line-strong);
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.02);
  }

  .t-day {
    display: grid;
    place-items: center;
    min-width: 34px;
    height: 34px;
    border-radius: 8px;
    background: var(--bg-elevated);
    font-family: var(--display);
    font-weight: 800;
    font-stretch: 90%;
    font-size: 1.15rem;
  }

  .t-rest {
    display: grid;
    line-height: 1.15;
    font-size: 0.8rem;
    font-weight: 600;
  }

  .continue {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 0 18px 0 22px;
    font-family: var(--display);
    font-stretch: 105%;
    font-size: 0.98rem;
    letter-spacing: 0.01em;
  }

  .continue.matchday {
    animation: pulse 2.2s var(--ease) infinite;
  }

  @keyframes pulse {
    50% {
      box-shadow: 0 0 0 6px rgba(212, 255, 63, 0.12), 0 8px 28px -8px var(--accent-glow);
    }
  }

  main {
    grid-area: main;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
  }

  .error {
    margin: 14px 24px 0;
    padding: 10px 16px;
    border-radius: var(--radius);
    background: var(--bad-soft);
    color: var(--bad);
    display: flex;
    gap: 12px;
    justify-content: space-between;
  }

  .tabbar,
  .sheet,
  .scrim {
    display: none;
  }

  /* Portrait iPad and phones: tabs along the bottom. */
  @media (max-width: 900px) {
    .shell {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr auto;
      grid-template-areas: 'header' 'main' 'tabs';
    }

    .rail {
      display: none;
    }

    .topbar {
      padding: 10px 14px;
      padding-top: max(10px, env(safe-area-inset-top));
      gap: 10px;
    }

    .tabbar {
      grid-area: tabs;
      display: flex;
      justify-content: space-around;
      padding: 6px 6px max(6px, env(safe-area-inset-bottom));
      border-top: 1px solid var(--line);
      background: rgba(7, 11, 17, 0.8);
      backdrop-filter: blur(14px);
      -webkit-backdrop-filter: blur(14px);
    }

    .nav-item.tab {
      flex: 1;
      flex-direction: column;
      justify-content: center;
      gap: 3px;
      min-height: 50px;
      padding: 4px 2px;
      font-size: 0.7rem;
      font-weight: 600;
      border: 0;
      background: none;
    }

    .nav-item.tab.active {
      background: none;
    }

    .nav-item.tab.active::before {
      left: 30%;
      right: 30%;
      top: -7px;
      bottom: auto;
      width: auto;
      height: 3px;
      border-radius: 0 0 3px 3px;
    }

    .nav-item.tab .badge {
      position: absolute;
      top: 2px;
      left: calc(50% + 6px);
      margin: 0;
    }

    .scrim {
      display: block;
      position: fixed;
      inset: 0;
      border: 0;
      border-radius: 0;
      background: rgba(0, 0, 0, 0.45);
      z-index: 9;
    }

    .sheet {
      display: grid;
      position: fixed;
      right: 10px;
      left: 10px;
      bottom: calc(72px + env(safe-area-inset-bottom));
      padding: 8px;
      gap: 2px;
      grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
      background: var(--glass);
      backdrop-filter: blur(20px);
      -webkit-backdrop-filter: blur(20px);
      border: 1px solid var(--line-strong);
      border-radius: var(--radius-lg);
      box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
      z-index: 10;
      animation: rise 0.2s var(--ease) both;
    }

    .sheet a {
      display: flex;
      gap: 10px;
      align-items: center;
      min-height: var(--tap);
      padding: 0 12px;
      border-radius: var(--radius);
      color: var(--text-dim);
    }

    .sheet a.active {
      color: var(--accent);
      background: var(--accent-soft);
    }

    .league,
    .t-rest .dim {
      display: none;
    }
  }

  @media (max-width: 560px) {
    .club-text,
    .ticket {
      display: none;
    }
  }
</style>
