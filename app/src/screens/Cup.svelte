<!-- A cup this season: the latest round first, groups, and past winners. -->
<script lang="ts">
  import type { CupStage, Fixture, Tie } from '../api';
  import ClubName from '../components/ClubName.svelte';
  import FixtureLine from '../components/FixtureLine.svelte';
  import { useGame } from '../state/game.svelte';
  import { href } from '../state/router.svelte';
  import { load } from '../state/load.svelte';

  let { cup = undefined }: { cup?: number } = $props();
  const game = useGame();
  const view = load(() => game.client.cup(cup ?? null));
  const mine = $derived(game.overview?.club?.id ?? null);
  /** Newest stage first; the newest is open. */
  const stages = $derived([...(view.data?.stages ?? [])].reverse());

  function score(f: Fixture): string {
    const r = f.result;
    if (!r) return 'v';
    return `${r.goals[0]}–${r.goals[1]}`;
  }

  /** The tie's outcome in words, from the first-named club's side. */
  function tieNote(t: Tie): string {
    const last = t.legs.at(-1)?.result;
    if (!last) return t.legs.length > 1 ? 'Two legs' : '';
    const parts: string[] = [];
    if (t.aggregate) parts.push(`Agg ${t.aggregate[0]}–${t.aggregate[1]}`);
    if (last.penalties) {
      // Penalties as the first-named club scored them.
      const lastLeg = t.legs.at(-1)!;
      const p = lastLeg.home.id === t.clubs[0].id ? last.penalties : [last.penalties[1], last.penalties[0]];
      parts.push(`${p[0]}–${p[1]} on penalties`);
    } else if (last.extraTime) {
      parts.push('after extra time');
    }
    return parts.join(', ');
  }

  const involves = (s: CupStage) =>
    mine !== null &&
    (s.ties.some((t) => t.clubs.some((c) => c.id === mine)) ||
      s.groups.some((g) => g.rows.some((r) => r.club.id === mine)) ||
      s.byes.some((b) => b.id === mine));
</script>

<div class="screen grid">
  {#if view.data}
    {@const c = view.data}
    <div class="row head">
      <h1>{c.cup.name}</h1>
      <span class="dim">{c.cup.region} · {c.season}</span>
      <span class="spacer"></span>
      {#if c.cups.length > 1}
        <select
          aria-label="Cup"
          value={c.cup.id}
          onchange={(e) => game.router.go({ name: 'cup', cup: Number(e.currentTarget.value) })}
        >
          {#each c.cups as x (x.id)}
            <option value={x.id}>{x.continental ? x.name : `${x.name}`}</option>
          {/each}
        </select>
      {/if}
      <a href={href({ name: 'competitions' })}>All competitions</a>
    </div>

    {#if c.winner}
      <section class="card winner">
        <span class="trophy" aria-hidden="true">♕</span>
        <div>
          <div class="dim">Winners {c.season}</div>
          <ClubName club={c.winner} />
        </div>
      </section>
    {/if}

    {#each stages as s, i (s.name + i)}
      <details class="card stage" open={i === 0 || involves(s)}>
        <summary>
          <h3>{s.name}</h3>
          {#if involves(s)}<span class="pill">You</span>{/if}
        </summary>
        {#if s.groups.length > 0}
          <div class="groups">
            {#each s.groups as g (g.name)}
              <div class="group">
                <h4>{g.name}</h4>
                <table class="data compact">
                  <thead>
                    <tr><th>Club</th><th class="num">P</th><th class="num">GD</th><th class="num">Pts</th></tr>
                  </thead>
                  <tbody>
                    {#each g.rows as r (r.club.id)}
                      <tr class:mine={r.club.id === mine} class:through={r.zone === 'qualify'}>
                        <td><ClubName club={r.club} short /></td>
                        <td class="num">{r.played}</td>
                        <td class="num">{r.goalDifference > 0 ? '+' : ''}{r.goalDifference}</td>
                        <td class="num"><strong>{r.points}</strong></td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
                {#if g.fixtures.some((f) => f.home.id === mine || f.away.id === mine)}
                  <div class="legs">
                    {#each g.fixtures.filter((f) => f.home.id === mine || f.away.id === mine) as f (f.index)}
                      <FixtureLine fixture={f} {mine} />
                    {/each}
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
        {#each s.ties as t (t.clubs[0].id)}
          <div class="tie" class:mine={t.clubs.some((x) => x.id === mine)}>
            <span class="side a" class:won={t.winner === t.clubs[0].id} class:lost={t.winner !== null && t.winner !== t.clubs[0].id}>
              <ClubName club={t.clubs[0]} short />
            </span>
            <span class="scores">
              {#if t.legs.some((f) => f.result)}
                {#each t.legs as f (f.index)}
                  <span title={f.round ?? ''}>{f.home.id === t.clubs[0].id ? score(f) : score(f).split('–').reverse().join('–')}</span>
                {/each}
              {:else}
                <span class="vs">vs</span>
              {/if}
            </span>
            <span class="side b" class:won={t.winner === t.clubs[1].id} class:lost={t.winner !== null && t.winner !== t.clubs[1].id}>
              <ClubName club={t.clubs[1]} short />
            </span>
            {#if tieNote(t)}<span class="note dim">{tieNote(t)}</span>{/if}
          </div>
        {/each}
        {#if s.byes.length > 0}
          <p class="dim byes">
            Byes: {s.byes.map((b) => b.shortName).join(', ')}
          </p>
        {/if}
      </details>
    {/each}

    {#if c.upcoming.length > 0}
      <p class="dim">Still to come: {c.upcoming.join(', ')}.</p>
    {/if}

    {#if c.history.length > 0}
      <section class="card">
        <h3>Past winners</h3>
        <table class="data">
          <thead><tr><th>Season</th><th>Winners</th><th>Runners-up</th></tr></thead>
          <tbody>
            {#each c.history as h (h.season)}
              <tr>
                <td>{h.season}</td>
                <td>{#if h.winner}<ClubName club={h.winner} />{:else}<span class="dim">Not completed</span>{/if}</td>
                <td>{#if h.runnerUp}<ClubName club={h.runnerUp} />{/if}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>
    {/if}
  {/if}
</div>

<style>
  .head {
    flex-wrap: wrap;
  }

  .spacer {
    flex: 1;
  }

  .winner {
    display: flex;
    align-items: center;
    gap: 16px;
    border-color: var(--accent);
  }

  .trophy {
    font-size: 2.2rem;
    color: var(--accent);
  }

  .stage summary {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    list-style: none;
  }

  .stage summary h3 {
    margin: 0;
  }

  .groups {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: var(--gap);
    margin-top: 8px;
  }

  .group h4 {
    margin: 4px 0;
  }

  tr.through td:first-child {
    box-shadow: inset 3px 0 var(--good);
  }

  .tie {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 8px;
    padding: 6px 0;
    border-bottom: 1px solid var(--line);
  }

  .tie.mine {
    background: var(--accent-soft);
    border-radius: 10px;
    border-bottom-color: transparent;
  }

  .side.a {
    justify-self: end;
  }

  .side.won {
    font-weight: 700;
  }

  .side.lost {
    opacity: 0.6;
  }

  .scores {
    display: inline-flex;
    gap: 6px;
    font-family: var(--display);
    font-variant-numeric: tabular-nums;
    font-weight: 800;
  }

  .scores span {
    min-width: 3.4em;
    text-align: center;
    background: var(--bg-sunken);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 3px 8px;
  }

  .scores .vs {
    font-size: 0.75rem;
    font-weight: 700;
    font-stretch: 115%;
    text-transform: uppercase;
    color: var(--text-faint);
  }

  .note {
    grid-column: 1 / -1;
    text-align: center;
    font-size: 0.8rem;
  }

  .byes {
    font-size: 0.85rem;
  }

  .legs {
    margin-top: 6px;
  }
</style>
