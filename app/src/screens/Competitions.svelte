<!-- Every league and cup in the world, and continental coefficients. -->
<script lang="ts">
  import { useGame } from '../state/game.svelte';
  import { href } from '../state/router.svelte';
  import { load } from '../state/load.svelte';

  const game = useGame();
  const view = load(() => game.client.competitions());
  let filter = $state('');
  /** Column headings for past seasons: "-4" ... "Last". */
  const seasonLabel = (i: number, n: number) => (i === n - 1 ? 'Last' : `${i - n + 1}`);
  const nations = $derived(
    (view.data?.nations ?? []).filter((n) => {
      const f = filter.trim().toLowerCase();
      return f === '' || n.nation.toLowerCase().includes(f) || n.code.toLowerCase().includes(f);
    }),
  );
</script>

<div class="screen grid">
  <h1>Competitions</h1>
  {#if view.data}
    {#each view.data.continents as c (c.name)}
      <section class="card">
        <h2>{c.name}</h2>
        <div class="row cups">
          {#each c.cups as cup (cup.id)}
            <a class="pill link" href={href({ name: 'cup', cup: cup.id })}>{cup.name}</a>
          {/each}
        </div>
        <details>
          <summary>Nation coefficients</summary>
          <div class="scroll-x">
            <table class="data compact">
              <thead>
                <tr>
                  <th class="num">#</th>
                  <th>Nation</th>
                  <th class="num">Points</th>
                  {#each c.ranking[0]?.seasons ?? [] as _, i (i)}
                    <th class="num">{seasonLabel(i, c.ranking[0].seasons.length)}</th>
                  {/each}
                </tr>
              </thead>
              <tbody>
                {#each c.ranking as r (r.code + r.rank)}
                  <tr>
                    <td class="num">{r.rank}</td>
                    <td>{r.nation} <span class="dim">{r.code}</span></td>
                    <td class="num"><strong>{r.points.toFixed(1)}</strong></td>
                    {#each r.seasons as p, i (i)}
                      <td class="num dim">{p.toFixed(1)}</td>
                    {/each}
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          <p class="dim small">
            Points per club each season: two for a win and one for a draw in continental matches, plus one for
            each later stage reached. The better a nation's coefficient, the more of its clubs qualify.
          </p>
        </details>
      </section>
    {/each}

    <section class="card">
      <div class="row">
        <h2>Nations</h2>
        <span class="spacer"></span>
        {#if view.data.nations.length > 6}
          <input type="search" placeholder="Find a nation" bind:value={filter} aria-label="Find a nation" />
        {/if}
      </div>
      <div class="nations">
        {#each nations as n (n.code + n.nation)}
          <div class="nation">
            <h3>{n.nation} <span class="dim">{n.code}</span></h3>
            <ul>
              {#each n.leagues as l (l.id)}
                <li><a href={href({ name: 'table', league: l.id })}>{l.name}</a></li>
              {/each}
              {#each n.cups as cup (cup.id)}
                <li><a href={href({ name: 'cup', cup: cup.id })}>{cup.name}</a></li>
              {/each}
            </ul>
          </div>
        {/each}
      </div>
    </section>
  {/if}
</div>

<style>
  .cups {
    flex-wrap: wrap;
    margin-bottom: 8px;
  }

  .link {
    text-decoration: none;
  }

  .spacer {
    flex: 1;
  }

  .nations {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--gap);
  }

  .nation h3 {
    margin: 8px 0 4px;
  }

  .nation ul {
    margin: 0;
    padding-left: 18px;
  }

  .small {
    font-size: 0.85rem;
  }
</style>
