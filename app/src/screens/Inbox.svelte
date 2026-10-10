<!-- Messages: news about the club, results, injuries, contracts. -->
<script lang="ts">
  import type { Link, Message } from '../api';
  import { longDate } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';
  import type { Route } from '../state/router.svelte';

  const game = useGame();
  const inbox = load(() => game.client.inbox());
  let openId = $state<number | null>(null);
  const open = $derived(inbox.data?.messages.find((m) => m.id === openId) ?? null);

  async function select(m: Message) {
    openId = m.id;
    if (!m.read) {
      await game.client.markRead(m.id);
      await game.refresh();
    }
  }

  async function readAll() {
    await game.client.markRead(null);
    await game.refresh();
  }

  function route(link: Link): Route {
    switch (link.type) {
      case 'player':
        return { name: 'player', id: link.player };
      case 'fixture':
        return { name: 'fixtures', league: link.league };
      case 'table':
        return { name: 'table', league: link.league };
      case 'squad':
        return { name: 'squad' };
      case 'tactics':
        return { name: 'tactics' };
    }
  }

  const label: Record<Link['type'], string> = {
    player: 'View player',
    fixture: 'View fixtures',
    table: 'View table',
    squad: 'View squad',
    tactics: 'View tactics',
  };
</script>

<div class="screen">
  <div class="row head">
    <h1>Inbox</h1>
    <span class="spacer"></span>
    {#if inbox.data && inbox.data.unread > 0}
      <button onclick={readAll}>Mark all read</button>
    {/if}
  </div>
  <div class="split" class:reading={open !== null}>
    <ul class="list card">
      {#each inbox.data?.messages ?? [] as m (m.id)}
        <li>
          <button class="item" class:unread={!m.read} class:active={m.id === openId} onclick={() => select(m)}>
            <span class="pill">{m.category}</span>
            <span class="t">{m.title}</span>
            <span class="dim d">{longDate(m.date)}</span>
          </button>
        </li>
      {:else}
        <li class="dim">No messages yet.</li>
      {/each}
    </ul>
    <article class="card body">
      {#if open}
        <button class="link back" onclick={() => (openId = null)}>‹ Messages</button>
        <h2>{open.title}</h2>
        <p class="dim">{longDate(open.date)}</p>
        <p>{open.body}</p>
        {#if open.link}
          {@const link = open.link}
          <button onclick={() => game.router.go(route(link))}>{label[link.type]}</button>
        {/if}
      {:else}
        <p class="dim">Choose a message.</p>
      {/if}
    </article>
  </div>
</div>

<style>
  .head {
    margin-bottom: 12px;
  }

  .spacer {
    flex: 1;
  }

  .split {
    display: grid;
    grid-template-columns: minmax(280px, 2fr) 3fr;
    gap: var(--gap);
    align-items: start;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 6px;
    max-height: calc(100vh - 200px);
    overflow-y: auto;
  }

  .item {
    width: 100%;
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-areas: 'p d' 't t';
    gap: 4px 8px;
    text-align: left;
    border: 0;
    padding: 10px;
    background: none;
    color: var(--text-dim);
  }

  .item.unread {
    color: var(--text);
    font-weight: 600;
  }

  .item.active {
    background: var(--bg-sunken);
  }

  .item .pill {
    grid-area: p;
    justify-self: start;
  }

  .t {
    grid-area: t;
  }

  .d {
    grid-area: d;
    justify-self: end;
    font-weight: 400;
    font-size: 0.85rem;
  }

  .back {
    display: none;
    margin-bottom: 12px;
  }

  @media (max-width: 700px) {
    .split {
      grid-template-columns: 1fr;
    }

    .split.reading .list,
    .split:not(.reading) .body {
      display: none;
    }

    .back {
      display: inline;
    }
  }
</style>
