<!-- The manager's desk: league position, next match, latest news. -->
<script lang="ts">
  import FixtureLine from '../components/FixtureLine.svelte';
  import Form from '../components/Form.svelte';
  import Meter from '../components/Meter.svelte';
  import { money, ordinal } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';
  import { href } from '../state/router.svelte';

  const game = useGame();
  const o = $derived(game.overview);
  const inbox = load(() => game.client.inbox());

  async function toggleWatch() {
    if (!o) return;
    const next = await game.run('Saving', () => game.client.setWatchMatches(!o.watchMatches));
    if (next) game.changed(next);
  }

  let resigning = $state(false);

  async function resign() {
    const next = await game.run('Resigning', () => game.client.resign());
    resigning = false;
    if (next) game.changed(next);
  }

  async function toggleDelegate() {
    if (!o) return;
    const next = await game.run('Saving', () => game.client.setDelegation(!o.delegateSquad));
    if (next) game.changed(next);
  }
</script>

{#if o && !o.club}
  <div class="screen grid home">
    <section class="card">
      <h1>Out of work</h1>
      <p class="dim">{o.manager.name} · {o.season}</p>
      <p>Clubs that sack their managers advertise the job. Apply for one, or press Continue to wait for more to come up.</p>
      <div class="row">
        <button class="primary" onclick={() => game.router.go({ name: 'jobs' })}>Look for a job</button>
      </div>
    </section>
  </div>
{:else if o && o.club}
  {@const club = o.club}
  <div class="screen grid home">
    <section class="card hero">
      <div>
        <h1>{club.name}</h1>
        <p class="dim">
          {o.manager.name} · {o.season}
          {#if o.league}· {o.league.name}{/if}
        </p>
      </div>
      {#if o.position}
        <div class="pos">
          <span class="big">{ordinal(o.position)}</span>
          <Form form={o.form} />
        </div>
      {/if}
    </section>

    {#if o.matchDay}
      <section class="card matchday">
        <h3>Match day</h3>
        <FixtureLine fixture={o.matchDay.fixture} mine={club.id} competition />
        <p class="dim">{o.matchDay.fixture.competition.name}{o.matchDay.fixture.round ? ` · ${o.matchDay.fixture.round}` : ''}</p>
        <div class="row">
          <button class="primary" onclick={() => game.router.go({ name: 'match' })}>Go to the match</button>
        </div>
      </section>
    {/if}

    <div class="cols">
      <section class="card">
        <h3>Next match</h3>
        {#if o.nextFixture}
          <FixtureLine fixture={o.nextFixture} mine={club.id} competition />
          <div class="row"><button onclick={() => game.router.go({ name: 'prepare' })}>Prepare ▸</button></div>
        {:else}
          <p class="dim">No fixtures scheduled.</p>
        {/if}
        <h3 class="gap">Last match</h3>
        {#if o.lastFixture}
          <FixtureLine fixture={o.lastFixture} mine={club.id} competition />
        {:else}
          <p class="dim">The season has not started.</p>
        {/if}
      </section>

      <section class="card">
        <h3>Inbox</h3>
        {#if inbox.data}
          {#each inbox.data.messages.slice(0, 5) as m (m.id)}
            <a class="msg" class:unread={!m.read} href="#/inbox">
              <span>{m.title}</span>
            </a>
          {:else}
            <p class="dim">No messages.</p>
          {/each}
        {/if}
      </section>
    </div>

    {#if o.cups.length > 0}
      <section class="card">
        <h3>Cups</h3>
        {#each o.cups as c (c.cup.id)}
          <a class="cup" href={href({ name: 'cup', cup: c.cup.id })}>
            <span>{c.cup.name}</span>
            <span class:out={!c.alive} class="status">{c.status}</span>
          </a>
        {/each}
      </section>
    {/if}

    <div class="cols">
      {#if o.board}
        <section class="card">
          <h3>The board</h3>
          <p><strong>{o.board.objective}</strong></p>
          <div class="row">
            <span class="dim">Confidence</span>
            <Meter value={o.board.confidence} label="Board confidence" good={55} ok={35} />
            <span>{o.board.confidence}%</span>
          </div>
          {#if o.board.objectives.length > 0}
            <ul class="objectives" data-testid="objectives">
              {#each o.board.objectives as ob, i (i)}
                <li>
                  <span class="pill {ob.status === 'met' || ob.status === 'onTrack' ? 'good' : ob.status === 'failed' ? 'bad' : 'warn'}">
                    {ob.status === 'met' ? 'Met' : ob.status === 'failed' ? 'Missed' : ob.status === 'onTrack' ? 'On track' : 'Behind'}
                  </span>
                  <span>{ob.text}{ob.important ? ' (important)' : ''}</span>
                  <span class="dim">{ob.detail}</span>
                </li>
              {/each}
            </ul>
          {/if}
          <p class="dim small">The club {o.board.ethos.toLowerCase()}.</p>
        </section>
      {/if}
      <section class="card">
        <h3>Money</h3>
        <p>
          <span class="dim">Balance</span> <strong>{money(o.balance ?? 0)}</strong> ·
          <span class="dim">Transfer budget</span> <strong>{money(o.transferBudget ?? 0)}</strong>
        </p>
        <p class="dim">The transfer window is {o.windowOpen ? 'open' : 'closed'}.</p>
        <div class="row">
          <button onclick={() => game.router.go({ name: 'transfers' })}>Transfers</button>
          <button onclick={() => game.router.go({ name: 'finances' })}>Finances</button>
        </div>
      </section>
    </div>

    <section class="card grid">
      <h3>Settings</h3>
      <label class="toggle">
        <input type="checkbox" checked={o.watchMatches} onchange={toggleWatch} disabled={o.matchDay !== null} />
        <span>Stop on match days to watch and manage matches live</span>
      </label>
      <label class="toggle">
        <input type="checkbox" checked={o.delegateSquad} onchange={toggleDelegate} />
        <span>Let the assistant handle contracts and signings</span>
      </label>
      <div class="row">
        {#if resigning}
          <span>Leave {club.name}? You will have to find a new job.</span>
          <button class="danger" onclick={resign}>Resign</button>
          <button onclick={() => (resigning = false)}>Stay</button>
        {:else}
          <button class="danger" onclick={() => (resigning = true)}>Resign…</button>
        {/if}
      </div>
    </section>
  </div>
{/if}

<style>
  .hero {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }

  .pos {
    display: grid;
    justify-items: end;
    gap: 6px;
  }

  .big {
    font-size: 2rem;
    font-weight: 800;
  }

  .matchday {
    border-color: var(--accent);
  }

  .cols {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: var(--gap);
  }

  .gap {
    margin-top: 16px;
  }

  .cup {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 0;
    border-bottom: 1px solid var(--line);
    color: inherit;
  }

  .status {
    font-weight: 600;
    color: var(--accent);
  }

  .status.out {
    color: var(--text-dim);
    font-weight: 400;
  }

  .msg {
    display: block;
    padding: 8px 0;
    border-bottom: 1px solid var(--line);
    color: var(--text-dim);
  }

  .msg.unread {
    color: var(--text);
    font-weight: 600;
  }

  .toggle {
    display: flex;
    gap: 12px;
    align-items: center;
    min-height: var(--tap);
  }

  .toggle input {
    width: 22px;
    height: 22px;
    accent-color: var(--accent);
  }

  .objectives {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }

  .objectives li {
    display: grid;
    grid-template-columns: 6.5em 1fr;
    gap: 2px 8px;
    align-items: baseline;
  }

  .objectives li .dim {
    grid-column: 2;
    font-size: 0.85rem;
  }
</style>
