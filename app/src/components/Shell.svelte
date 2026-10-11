<!-- The frame around every screen: navigation, date and Continue. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { longDate } from '../format';
  import { useGame } from '../state/game.svelte';
  import { href, type Route } from '../state/router.svelte';
  import Crest from './Crest.svelte';

  let { children }: { children: Snippet } = $props();
  const game = useGame();
  const o = $derived(game.overview);

  type Item = { route: Route; label: string; icon: string };
  const employed: Item[] = [
    { route: { name: 'home' }, label: 'Home', icon: '⌂' },
    { route: { name: 'inbox' }, label: 'Inbox', icon: '✉' },
    { route: { name: 'squad' }, label: 'Squad', icon: '☰' },
    { route: { name: 'tactics' }, label: 'Tactics', icon: '◎' },
    { route: { name: 'transfers' }, label: 'Transfers', icon: '⇄' },
    { route: { name: 'staff' }, label: 'Staff', icon: '♟' },
    { route: { name: 'finances' }, label: 'Finances', icon: '£' },
    { route: { name: 'fixtures' }, label: 'Fixtures', icon: '▦' },
    { route: { name: 'table' }, label: 'Table', icon: '≡' },
    { route: { name: 'saves' }, label: 'Saves', icon: '⇩' },
  ];
  const unemployed: Item[] = [
    { route: { name: 'home' }, label: 'Home', icon: '⌂' },
    { route: { name: 'inbox' }, label: 'Inbox', icon: '✉' },
    { route: { name: 'jobs' }, label: 'Jobs', icon: '✎' },
    { route: { name: 'saves' }, label: 'Saves', icon: '⇩' },
  ];
  const nav = $derived(o?.club ? employed : unemployed);
  /** On phones and portrait iPad: the first four, then "More". */
  const PRIMARY = 4;
  let more = $state(false);
  const current = $derived(game.router.route.name);
  const active = (item: Item) =>
    current === item.route.name || (item.route.name === 'squad' && current === 'player');
</script>

<div class="shell">
  <header>
    {#if o}
      <a class="club" href="#/home">
        {#if o.club}
          <Crest club={o.club} size={32} />
          <span class="name">{o.club.name}</span>
        {:else}
          <span class="name">{o.manager.name}</span>
          <span class="pill">Out of work</span>
        {/if}
      </a>
      <span class="date">
        <span class="dim">{o.weekday}</span>
        {longDate(o.date)}
      </span>
    {/if}
    <span class="spacer"></span>
    {#if game.busy}
      <span class="busy" role="status"><span class="spinner"></span>{game.busy}…</span>
    {/if}
    <button class="primary continue" disabled={game.busy !== null} onclick={() => game.continueOn()}>
      {o?.matchDay ? 'Match day ▸' : 'Continue ▸'}
    </button>
  </header>

  <nav aria-label="Main">
    {#each nav as item, i (item.label)}
      <a
        href={href(item.route)}
        class:active={active(item)}
        class:secondary={i >= PRIMARY}
        onclick={() => (more = false)}
      >
        <span class="icon" aria-hidden="true">{item.icon}</span>
        <span class="label">{item.label}</span>
        {#if item.route.name === 'inbox' && o && o.unread > 0}
          <span class="badge">{o.unread}</span>
        {/if}
      </a>
    {/each}
    {#if nav.length > PRIMARY}
      <button class="more" class:active={more || nav.slice(PRIMARY).some(active)} onclick={() => (more = !more)} aria-expanded={more}>
        <span class="icon" aria-hidden="true">⋯</span>
        <span class="label">More</span>
      </button>
    {/if}
  </nav>
  {#if more}
    <div class="sheet" role="menu">
      {#each nav.slice(PRIMARY) as item (item.label)}
        <a role="menuitem" href={href(item.route)} class:active={active(item)} onclick={() => (more = false)}>
          <span class="icon" aria-hidden="true">{item.icon}</span>
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
    grid-template-columns: 180px 1fr;
    grid-template-rows: auto 1fr;
    grid-template-areas: 'header header' 'nav main';
    height: 100%;
  }

  header {
    grid-area: header;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 10px 16px;
    padding-top: max(10px, env(safe-area-inset-top));
    border-bottom: 1px solid var(--line);
    background: var(--bg-raised);
  }

  .club {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text);
    font-weight: 700;
    font-size: 1.05rem;
  }

  .spacer {
    flex: 1;
  }

  .busy {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-dim);
  }

  .spinner {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid var(--line);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  nav {
    grid-area: nav;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 8px;
    border-right: 1px solid var(--line);
    background: var(--bg-raised);
  }

  nav a {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: var(--tap);
    padding: 0 12px;
    border-radius: var(--radius);
    color: var(--text-dim);
  }

  nav a.active {
    background: var(--bg-sunken);
    color: var(--text);
  }

  .icon {
    width: 1.4em;
    text-align: center;
  }

  .more,
  .sheet {
    display: none;
  }

  .badge {
    margin-left: auto;
    min-width: 20px;
    padding: 0 6px;
    border-radius: 10px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 0.75rem;
    font-weight: 700;
    text-align: center;
  }

  main {
    grid-area: main;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
  }

  .error {
    margin: 12px 16px 0;
    padding: 10px 14px;
    border-radius: var(--radius);
    border: 1px solid var(--bad);
    color: var(--bad);
    display: flex;
    gap: 12px;
    justify-content: space-between;
  }

  /* Portrait iPad and phones: tabs along the bottom. */
  @media (max-width: 900px) {
    .shell {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr auto;
      grid-template-areas: 'header' 'main' 'nav';
    }

    nav {
      flex-direction: row;
      justify-content: space-around;
      padding: 4px 4px max(4px, env(safe-area-inset-bottom));
      border-right: 0;
      border-top: 1px solid var(--line);
    }

    nav a {
      flex-direction: column;
      gap: 0;
      padding: 4px 6px;
      font-size: 0.72rem;
      position: relative;
    }

    .badge {
      position: absolute;
      top: 0;
      right: 0;
    }

    nav a.secondary {
      display: none;
    }

    .more {
      display: flex;
      flex-direction: column;
      align-items: center;
      min-height: var(--tap);
      padding: 4px 6px;
      border: 0;
      background: none;
      color: var(--text-dim);
      font-size: 0.72rem;
    }

    .more.active {
      color: var(--text);
    }

    .sheet {
      display: grid;
      position: fixed;
      right: 8px;
      bottom: calc(64px + env(safe-area-inset-bottom));
      min-width: 200px;
      padding: 6px;
      gap: 2px;
      background: var(--bg-raised);
      border: 1px solid var(--line);
      border-radius: var(--radius);
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
      z-index: 10;
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
      color: var(--text);
      background: var(--bg-sunken);
    }

    .name,
    .date .dim {
      display: none;
    }
  }
</style>
