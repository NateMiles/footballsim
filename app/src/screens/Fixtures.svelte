<!-- The season's fixtures: the manager's club, or a whole league. -->
<script lang="ts">
  import type { Fixture } from '../api';
  import FixtureLine from '../components/FixtureLine.svelte';
  import { monthOf } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  let { league = undefined }: { league?: number } = $props();
  const game = useGame();
  let whole = $state(false);
  const mine = $derived(game.overview?.club?.id ?? null);
  const view = load(() =>
    whole || league !== undefined ? game.client.fixtures(league ?? game.overview?.league?.id ?? null, null) : game.client.fixtures(),
  );

  const months = $derived.by(() => {
    const groups: { month: string; fixtures: Fixture[] }[] = [];
    for (const f of view.data?.fixtures ?? []) {
      const m = monthOf(f.date);
      if (groups.at(-1)?.month !== m) groups.push({ month: m, fixtures: [] });
      groups.at(-1)!.fixtures.push(f);
    }
    return groups;
  });
</script>

<div class="screen">
  <div class="row head">
    <h1>Fixtures</h1>
    <span class="dim">{view.data?.league.name ?? ''}</span>
    <span class="spacer"></span>
    {#if league === undefined}
      <div class="seg" role="group" aria-label="Show">
        <button class:active={!whole} onclick={() => (whole = false)}>My club</button>
        <button class:active={whole} onclick={() => (whole = true)}>Whole league</button>
      </div>
    {/if}
  </div>
  {#each months as g (g.month)}
    <section class="card month">
      <h3>{g.month}</h3>
      {#each g.fixtures as f (f.index)}
        <FixtureLine fixture={f} {mine} />
      {/each}
    </section>
  {:else}
    <p class="dim">No fixtures.</p>
  {/each}
</div>

<style>
  .head {
    margin-bottom: 12px;
  }

  .spacer {
    flex: 1;
  }

  .seg {
    display: inline-flex;
  }

  .seg button {
    border-radius: 0;
  }

  .seg button:first-child {
    border-radius: var(--radius) 0 0 var(--radius);
  }

  .seg button:last-child {
    border-radius: 0 var(--radius) var(--radius) 0;
  }

  .seg button.active {
    background: var(--bg-sunken);
    border-color: var(--accent);
  }

  .month {
    margin-bottom: var(--gap);
  }
</style>
