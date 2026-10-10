<!-- A club's players, sortable. -->
<script lang="ts">
  import type { SquadPlayer } from '../api';
  import ClubName from '../components/ClubName.svelte';
  import Meter from '../components/Meter.svelte';
  import { shortDate } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  let { club = undefined }: { club?: number } = $props();
  const game = useGame();
  const squad = load(() => game.client.squad(club ?? null));
  const own = $derived(club === undefined || club === game.overview?.club.id);

  type Key = keyof Pick<SquadPlayer, 'number' | 'name' | 'position' | 'age' | 'ability' | 'potential' | 'fitness' | 'morale'> | 'apps' | 'goals' | 'rating';
  const columns: { key: Key; label: string; num?: boolean }[] = [
    { key: 'number', label: '#', num: true },
    { key: 'name', label: 'Name' },
    { key: 'position', label: 'Pos' },
    { key: 'age', label: 'Age', num: true },
    { key: 'ability', label: 'Abil', num: true },
    { key: 'potential', label: 'Pot', num: true },
    { key: 'fitness', label: 'Fitness' },
    { key: 'morale', label: 'Morale' },
    { key: 'apps', label: 'Apps', num: true },
    { key: 'goals', label: 'Gls', num: true },
    { key: 'rating', label: 'Rating', num: true },
  ];
  const POS_ORDER = ['GK', 'DR', 'DC', 'DL', 'WBR', 'WBL', 'DM', 'MR', 'MC', 'ML', 'AMR', 'AMC', 'AML', 'ST'];
  let sort = $state<Key>('position');
  let desc = $state(false);
  let filter = $state<'all' | 'first' | 'academy'>('all');

  function value(p: SquadPlayer, k: Key): number | string {
    switch (k) {
      case 'apps':
        return p.season.appearances;
      case 'goals':
        return p.season.goals;
      case 'rating':
        return p.season.rating ?? 0;
      case 'position':
        return POS_ORDER.indexOf(p.position);
      case 'number':
        return p.number ?? 100;
      default:
        return p[k];
    }
  }

  const rows = $derived.by(() => {
    const list = (squad.data?.players ?? []).filter((p) =>
      filter === 'all' ? true : filter === 'academy' ? p.academy : !p.academy,
    );
    return [...list].sort((a, b) => {
      const x = value(a, sort);
      const y = value(b, sort);
      const c = typeof x === 'string' ? x.localeCompare(String(y)) : x - (y as number);
      return desc ? -c : c;
    });
  });

  function sortBy(k: Key) {
    if (sort === k) desc = !desc;
    else {
      sort = k;
      desc = ['ability', 'potential', 'fitness', 'morale', 'apps', 'goals', 'rating'].includes(k);
    }
  }
</script>

<div class="screen">
  <div class="row head">
    {#if squad.data}
      {#if own}
        <h1>Squad</h1>
      {:else}
        <h1><ClubName club={squad.data.club} link={false} /></h1>
      {/if}
    {/if}
    <span class="spacer"></span>
    <select bind:value={filter} aria-label="Filter">
      <option value="all">All players</option>
      <option value="first">First team</option>
      <option value="academy">Academy</option>
    </select>
  </div>
  <div class="card scroll-x">
    <table class="data">
      <thead>
        <tr>
          {#each columns as c (c.key)}
            <th class="sortable" class:num={c.num} onclick={() => sortBy(c.key)}>
              {c.label}{sort === c.key ? (desc ? ' ▾' : ' ▴') : ''}
            </th>
          {/each}
          <th>Status</th>
          {#if own}<th>Contract</th>{/if}
        </tr>
      </thead>
      <tbody>
        {#each rows as p (p.id)}
          <tr class="clickable" onclick={() => game.router.go({ name: 'player', id: p.id })}>
            <td class="num dim">{p.number ?? ''}</td>
            <td><a href="#/player/{p.id}">{p.name}</a></td>
            <td>{p.position}</td>
            <td class="num">{p.age}</td>
            <td class="num">{p.ability}</td>
            <td class="num dim">{p.potential}</td>
            <td><Meter value={p.fitness} label="Fitness" /></td>
            <td><Meter value={p.morale} label="Morale" good={60} ok={40} /></td>
            <td class="num">{p.season.appearances}</td>
            <td class="num">{p.season.goals}</td>
            <td class="num">{p.season.rating?.toFixed(2) ?? '–'}</td>
            <td>
              {#if p.injury}<span class="pill bad" title="Out until {p.injury.until}">Injured</span>{/if}
              {#if p.suspension > 0}<span class="pill warn">Banned {p.suspension}</span>{/if}
              {#if p.selection === 'starter'}<span class="pill good">XI</span>{:else if p.selection === 'bench'}<span class="pill">Sub</span>{/if}
              {#if p.academy}<span class="pill">U18</span>{/if}
            </td>
            {#if own}
              <td class="dim">{p.contract ? shortDate(p.contract.expires) + ' ' + p.contract.expires.slice(0, 4) : ''}</td>
            {/if}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .head {
    margin-bottom: 12px;
  }

  .spacer {
    flex: 1;
  }

  td .pill + .pill {
    margin-left: 4px;
  }
</style>
