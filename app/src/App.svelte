<!-- The app: title screen, or the game with its screens. -->
<script lang="ts">
  import { onMount } from 'svelte';
  import Shell from './components/Shell.svelte';
  import Competitions from './screens/Competitions.svelte';
  import Cup from './screens/Cup.svelte';
  import Finances from './screens/Finances.svelte';
  import Fixtures from './screens/Fixtures.svelte';
  import Home from './screens/Home.svelte';
  import Inbox from './screens/Inbox.svelte';
  import Jobs from './screens/Jobs.svelte';
  import Match from './screens/Match.svelte';
  import Player from './screens/Player.svelte';
  import Prepare from './screens/Prepare.svelte';
  import Saves from './screens/Saves.svelte';
  import Squad from './screens/Squad.svelte';
  import Staff from './screens/Staff.svelte';
  import Table from './screens/Table.svelte';
  import Tactics from './screens/Tactics.svelte';
  import Title from './screens/Title.svelte';
  import Transfers from './screens/Transfers.svelte';
  import { useGame } from './state/game.svelte';

  const game = useGame();
  const route = $derived(game.router.route);

  onMount(() => {
    void game.boot();
  });
</script>

{#if game.phase === 'loading'}
  <div class="loading" role="status">
    {#if game.error}
      <p class="error">{game.error}</p>
    {:else}
      <p>Loading the simulation…</p>
    {/if}
  </div>
{:else if game.phase === 'title'}
  <Title />
{:else if route.name === 'match'}
  <Match />
{:else}
  <Shell>
    {#if route.name === 'inbox'}
      <Inbox />
    {:else if route.name === 'squad'}
      {#key route.club}<Squad club={route.club} />{/key}
    {:else if route.name === 'player'}
      {#key route.id}<Player id={route.id} />{/key}
    {:else if route.name === 'tactics'}
      <Tactics />
    {:else if route.name === 'fixtures'}
      {#key route.league}<Fixtures league={route.league} />{/key}
    {:else if route.name === 'table'}
      {#key route.league}<Table league={route.league} />{/key}
    {:else if route.name === 'cup'}
      {#key route.cup}<Cup cup={route.cup} />{/key}
    {:else if route.name === 'competitions'}
      <Competitions />
    {:else if route.name === 'prepare'}
      <Prepare />
    {:else if route.name === 'saves'}
      <Saves />
    {:else if route.name === 'transfers'}
      {#key route.tab}<Transfers tab={route.tab} />{/key}
    {:else if route.name === 'staff'}
      <Staff />
    {:else if route.name === 'finances'}
      <Finances />
    {:else if route.name === 'jobs'}
      <Jobs />
    {:else}
      <Home />
    {/if}
  </Shell>
{/if}

<style>
  .loading {
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--text-dim);
  }

  .error {
    color: var(--bad);
    max-width: 40em;
  }
</style>
