<!-- The manager's desk: league position, next match, latest news. -->
<script lang="ts">
  import FixtureLine from '../components/FixtureLine.svelte';
  import Form from '../components/Form.svelte';
  import { ordinal } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  const game = useGame();
  const o = $derived(game.overview);
  const inbox = load(() => game.client.inbox());

  async function toggleWatch() {
    if (!o) return;
    const next = await game.run('Saving', () => game.client.setWatchMatches(!o.watchMatches));
    if (next) game.changed(next);
  }

  async function toggleDelegate() {
    if (!o) return;
    const next = await game.run('Saving', () => game.client.setDelegation(!o.delegateSquad));
    if (next) game.changed(next);
  }
</script>

{#if o}
  <div class="screen grid home">
    <section class="card hero">
      <div>
        <h1>{o.club.name}</h1>
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
        <FixtureLine fixture={o.matchDay.fixture} mine={o.club.id} />
        <div class="row">
          <button class="primary" onclick={() => game.router.go({ name: 'match' })}>Go to the match</button>
        </div>
      </section>
    {/if}

    <div class="cols">
      <section class="card">
        <h3>Next match</h3>
        {#if o.nextFixture}
          <FixtureLine fixture={o.nextFixture} mine={o.club.id} />
        {:else}
          <p class="dim">No fixtures scheduled.</p>
        {/if}
        <h3 class="gap">Last match</h3>
        {#if o.lastFixture}
          <FixtureLine fixture={o.lastFixture} mine={o.club.id} />
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
</style>
