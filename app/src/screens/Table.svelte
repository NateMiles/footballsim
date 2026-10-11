<!-- A league table. -->
<script lang="ts">
  import ClubName from '../components/ClubName.svelte';
  import Form from '../components/Form.svelte';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  let { league = undefined }: { league?: number } = $props();
  const game = useGame();
  const view = load(() => game.client.table(league ?? null));
  const mine = $derived(game.overview?.club?.id);
  let showOdds = $state(false);
  const odds = load(async () => (showOdds ? await game.client.projections(league ?? null) : null));
  const pct = (x: number) => (x >= 0.995 ? '>99%' : x > 0 && x < 0.005 ? '<1%' : `${Math.round(x * 100)}%`);
</script>

<div class="screen">
  {#if view.data}
    {@const t = view.data}
    <div class="row head">
      <h1>{t.league.name}</h1>
      <span class="dim">{t.season}</span>
      <span class="spacer"></span>
      <div class="seg" role="group" aria-label="Show">
        <button class:active={!showOdds} onclick={() => (showOdds = false)}>Table</button>
        <button class:active={showOdds} onclick={() => (showOdds = true)}>Season odds</button>
      </div>
      {#if t.leagues.length > 1}
        <select
          aria-label="League"
          value={t.league.id}
          onchange={(e) => game.router.go({ name: 'table', league: Number(e.currentTarget.value) })}
        >
          {#each t.leagues as l (l.id)}
            <option value={l.id}>{l.nation} – {l.name}</option>
          {/each}
        </select>
      {/if}
    </div>
    {#if showOdds}
      {#if odds.data}
        {@const o = odds.data}
        <div class="card scroll-x">
          <table class="data">
            <thead>
              <tr>
                <th class="num">#</th>
                <th>Club</th>
                <th class="num">Pts</th>
                <th class="num" title="Expected final points">xPts</th>
                <th class="num" title="Expected final position">xPos</th>
                {#each o.zones as z (z.name)}<th class="num" title="Positions {z.from}-{z.to}">{z.name}</th>{/each}
              </tr>
            </thead>
            <tbody>
              {#each o.rows as r (r.club.id)}
                <tr class:mine={r.club.id === mine}>
                  <td class="num">{r.position}</td>
                  <td><ClubName club={r.club} /></td>
                  <td class="num">{r.points}</td>
                  <td class="num">{r.expectedPoints.toFixed(0)}</td>
                  <td class="num">{r.expectedPosition.toFixed(1)}</td>
                  {#each r.zones as z, i (i)}<td class="num" class:faint={z < 0.005}>{pct(z)}</td>{/each}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <p class="dim">The rest of the season simulated {o.runs.toLocaleString()} times from today's squads and form.</p>
      {:else}
        <p class="dim" role="status">Simulating the rest of the season…</p>
      {/if}
    {:else}
    <div class="card scroll-x">
      <table class="data">
        <thead>
          <tr>
            <th class="num">#</th>
            <th>Club</th>
            <th class="num">P</th>
            <th class="num">W</th>
            <th class="num">D</th>
            <th class="num">L</th>
            <th class="num">GF</th>
            <th class="num">GA</th>
            <th class="num">GD</th>
            <th class="num">Pts</th>
            <th>Form</th>
          </tr>
        </thead>
        <tbody>
          {#each t.rows as r (r.club.id)}
            <tr class:mine={r.club.id === mine} class="zone-{r.zone ?? 'none'}">
              <td class="num">{r.position}</td>
              <td><ClubName club={r.club} /></td>
              <td class="num">{r.played}</td>
              <td class="num">{r.won}</td>
              <td class="num">{r.drawn}</td>
              <td class="num">{r.lost}</td>
              <td class="num">{r.scored}</td>
              <td class="num">{r.conceded}</td>
              <td class="num">{r.goalDifference > 0 ? '+' : ''}{r.goalDifference}</td>
              <td class="num"><strong>{r.points}</strong></td>
              <td><Form form={r.form} /></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    {/if}
    <p class="dim legend">
      <span class="key champion"></span> Champions
      {#if t.rows.some((r) => r.zone === 'promotion')}<span class="key promotion"></span> Promotion{/if}
      {#if t.rows.some((r) => r.zone === 'relegation')}<span class="key relegation"></span> Relegation{/if}
    </p>
  {/if}
</div>

<style>
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

  .faint {
    color: var(--text-dim);
    opacity: 0.6;
  }

  .head {
    margin-bottom: 12px;
  }

  .spacer {
    flex: 1;
  }

  tr.zone-champion td:first-child,
  tr.zone-promotion td:first-child {
    box-shadow: inset 3px 0 var(--good);
  }

  tr.zone-relegation td:first-child {
    box-shadow: inset 3px 0 var(--bad);
  }

  .legend {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .key {
    width: 10px;
    height: 10px;
    border-radius: 2px;
    display: inline-block;
    margin-left: 10px;
  }

  .champion,
  .promotion {
    background: var(--good);
  }

  .relegation {
    background: var(--bad);
  }
</style>
