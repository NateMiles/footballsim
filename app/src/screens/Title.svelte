<!-- No game loaded: start a new one or load a save. -->
<script lang="ts">
  import type { ClubChoice, WorldSize } from '../api';
  import Crest from '../components/Crest.svelte';
  import { useGame } from '../state/game.svelte';
  import type { SaveSlot } from '../storage/saves';
  import { longDate } from '../format';

  const game = useGame();
  let step = $state<'start' | 'world' | 'club'>('start');
  let seed = $state(Math.floor(Math.random() * 1_000_000));
  let size = $state<WorldSize>('tiny');
  let choices = $state<ClubChoice[]>([]);
  let picked = $state<ClubChoice | null>(null);
  let firstName = $state('');
  let lastName = $state('');
  let slots = $state<SaveSlot[]>([]);
  let leagueFilter = $state<number | 'all'>('all');

  $effect(() => {
    game.saves.list().then((s) => (slots = s)).catch(game.fail);
  });

  const leagues = $derived([...new Map(choices.map((c) => [c.league.id, c.league])).values()]);
  const shown = $derived(leagueFilter === 'all' ? choices : choices.filter((c) => c.league.id === leagueFilter));

  async function generate() {
    const c = await game.run('Generating the world', () => game.client.newWorld(seed, size));
    if (c) {
      choices = c.clubs;
      picked = null;
      step = 'club';
    }
  }

  async function start() {
    if (!picked) return;
    const club = picked.club.id;
    const o = await game.run('Starting', () =>
      game.client.startCareer(club, firstName.trim() || 'Alex', lastName.trim() || 'Manager'),
    );
    if (o) {
      await game.enter(o);
      await game.autosave();
    }
  }

  async function load(slot: SaveSlot) {
    const save = await game.saves.get(slot.name);
    if (!save) return;
    const o = await game.run('Loading', () => game.client.load(save.data));
    if (o) await game.enter(o);
  }
</script>

<div class="title">
  <h1 class="logo">footballsim</h1>

  {#if game.error}
    <p class="error" role="alert">{game.error}</p>
  {/if}

  {#if step === 'start'}
    <div class="card grid">
      <button class="primary" onclick={() => (step = 'world')}>New game</button>
      {#if slots.length > 0}
        <h3>Continue a saved game</h3>
        {#each slots as slot (slot.name)}
          <button class="slot" onclick={() => load(slot)}>
            <strong>{slot.summary.club}</strong>
            <span class="dim">{slot.summary.manager} · {longDate(slot.summary.date)} · {slot.name}</span>
          </button>
        {/each}
      {/if}
    </div>
  {:else if step === 'world'}
    <div class="card grid">
      <h2>New world</h2>
      <label class="field">
        <span>World</span>
        <select bind:value={size}>
          <option value="tiny">One league (20 clubs)</option>
          <option value="small">24 nations, 120 leagues</option>
        </select>
      </label>
      <label class="field">
        <span>Seed</span>
        <input type="number" min="0" bind:value={seed} />
      </label>
      <div class="row">
        <button onclick={() => (step = 'start')}>Back</button>
        <button class="primary" disabled={game.busy !== null} onclick={generate}>
          {game.busy ?? 'Generate'}
        </button>
      </div>
    </div>
  {:else}
    <div class="card grid">
      <h2>Choose your club</h2>
      {#if leagues.length > 1}
        <select bind:value={leagueFilter} aria-label="League">
          <option value="all">All leagues</option>
          {#each leagues as l (l.id)}
            <option value={l.id}>{l.nation} – {l.name}</option>
          {/each}
        </select>
      {/if}
      <div class="clubs">
        {#each shown.slice(0, 400) as c (c.club.id)}
          <button class="club" class:picked={picked?.club.id === c.club.id} onclick={() => (picked = c)}>
            <Crest club={c.club} size={30} />
            <span class="cname">
              <strong>{c.club.name}</strong>
              <span class="dim">{c.league.name}</span>
            </span>
            <span class="stars" title="{c.stars} of 5">{'★'.repeat(c.stars)}{'☆'.repeat(5 - c.stars)}</span>
          </button>
        {/each}
      </div>
      <div class="row">
        <input type="text" placeholder="First name" bind:value={firstName} aria-label="First name" />
        <input type="text" placeholder="Last name" bind:value={lastName} aria-label="Last name" />
      </div>
      <div class="row">
        <button onclick={() => (step = 'world')}>Back</button>
        <button class="primary" disabled={!picked || game.busy !== null} onclick={start}>
          {picked ? `Take charge of ${picked.club.shortName}` : 'Pick a club'}
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .title {
    max-width: 640px;
    margin: 0 auto;
    padding: max(40px, env(safe-area-inset-top)) 16px 40px;
    display: grid;
    gap: 20px;
  }

  .logo {
    font-size: 2.4rem;
    letter-spacing: -0.03em;
    color: var(--accent);
    text-align: center;
  }

  .field {
    display: grid;
    gap: 6px;
  }

  .slot {
    display: grid;
    justify-items: start;
    padding: 10px 14px;
    text-align: left;
  }

  .clubs {
    display: grid;
    gap: 6px;
    max-height: 50vh;
    overflow-y: auto;
  }

  .club {
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
    padding: 8px 12px;
  }

  .club.picked {
    border-color: var(--accent);
    background: rgba(61, 220, 132, 0.08);
  }

  .cname {
    display: grid;
    flex: 1;
  }

  .stars {
    color: var(--warn);
    letter-spacing: 1px;
  }

  .error {
    color: var(--bad);
  }
</style>
