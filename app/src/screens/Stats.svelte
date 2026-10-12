<!-- The club's season by competition: results, goals and expected goals,
     the players' numbers, and every match with its analysis. -->
<script lang="ts">
  import FixtureLine from '../components/FixtureLine.svelte';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';
  import { analysisHref } from '../state/router.svelte';

  const game = useGame();
  const view = load(() => game.client.seasonStats());
  const mine = $derived(game.overview?.club?.id ?? null);
  let tab = $state(0);

  const pct = (x: number) => `${Math.round(x * 100)}%`;
</script>

<div class="screen grid">
  <h1>Season statistics</h1>
  {#if view.data}
    {@const v = view.data}
    <div class="tabs" role="tablist">
      {#each v.competitions as c, i (i)}
        <button role="tab" aria-selected={tab === i} class:active={tab === i} onclick={() => (tab = i)}>
          {c.competition?.name ?? 'All competitions'}
        </button>
      {/each}
    </div>
    {@const c = v.competitions[Math.min(tab, v.competitions.length - 1)]}
    {#if c.played === 0}
      <p class="dim">No matches yet this season.</p>
    {:else}
      <section class="card tiles" data-testid="season-summary">
        <div><span class="dim">Played</span><strong>{c.played}</strong></div>
        <div><span class="dim">W–D–L</span><strong>{c.won}–{c.drawn}–{c.lost}</strong></div>
        <div><span class="dim">Goals</span><strong>{c.goalsFor}–{c.goalsAgainst}</strong></div>
        <div><span class="dim">xG</span><strong>{c.xgFor.toFixed(1)}–{c.xgAgainst.toFixed(1)}</strong></div>
        <div><span class="dim">Shots</span><strong>{c.shotsFor}–{c.shotsAgainst}</strong></div>
        <div><span class="dim">Possession</span><strong>{pct(c.possession)}</strong></div>
      </section>

      <section class="card scroll-x">
        <h3>Players</h3>
        <table class="data compact">
          <thead>
            <tr><th>Player</th><th class="num">Min</th><th class="num">G</th><th class="num">A</th><th class="num">xG</th><th class="num">Sh</th><th class="num">Key</th><th class="num">Pass %</th><th class="num">Tkl</th><th class="num">Int</th><th class="num">Rtg</th></tr>
          </thead>
          <tbody>
            {#each c.players as p (p.player.id)}
              <tr>
                <td><span class="dim pos">{p.player.position}</span><a href="#/player/{p.player.id}">{p.player.name}</a></td>
                <td class="num">{p.minutes}</td>
                <td class="num">{p.goals}</td>
                <td class="num">{p.assists}</td>
                <td class="num">{p.xg.toFixed(1)}</td>
                <td class="num">{p.shots}</td>
                <td class="num">{p.keyPasses}</td>
                <td class="num">{p.passes > 0 ? pct(p.passesCompleted / p.passes) : '–'}</td>
                <td class="num">{p.tackles}</td>
                <td class="num">{p.interceptions}</td>
                <td class="num">{p.rating.toFixed(2)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>

      <section class="card">
        <h3>Matches</h3>
        {#each c.matches as f (f.competition.kind + f.competition.id + '-' + f.index)}
          <a class="match" href={analysisHref(f)} data-testid="analysis-link">
            <FixtureLine fixture={f} {mine} competition={c.competition === null} />
            <span class="go">Analysis ▸</span>
          </a>
        {/each}
      </section>
    {/if}
  {/if}
</div>

<style>
  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .tabs button.active {
    border-color: var(--accent);
    color: var(--accent);
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
    gap: 12px;
  }

  .tiles div {
    display: grid;
    gap: 2px;
  }

  .tiles strong {
    font-size: 1.3rem;
  }

  .pos {
    display: inline-block;
    width: 3em;
  }

  .match {
    display: flex;
    align-items: center;
    gap: 8px;
    color: inherit;
    text-decoration: none;
    padding: 2px 0;
  }

  .match :global(.fixture) {
    flex: 1;
  }

  .go {
    color: var(--accent);
    font-size: 0.85rem;
    white-space: nowrap;
  }
</style>
