<!-- The app: title screen, or the game with its screens. -->
<script lang="ts">
  import { onMount } from 'svelte';
  import Shell from './components/Shell.svelte';
  import Fixtures from './screens/Fixtures.svelte';
  import Home from './screens/Home.svelte';
  import Inbox from './screens/Inbox.svelte';
  import Match from './screens/Match.svelte';
  import Player from './screens/Player.svelte';
  import Saves from './screens/Saves.svelte';
  import Squad from './screens/Squad.svelte';
  import Table from './screens/Table.svelte';
  import Tactics from './screens/Tactics.svelte';
  import Title from './screens/Title.svelte';
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
    {:else if route.name === 'saves'}
      <Saves />
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
