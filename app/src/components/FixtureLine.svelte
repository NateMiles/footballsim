<!-- One fixture: date, teams and score, with the competition when asked. -->
<script lang="ts">
  import type { Fixture } from '../api';
  import { shortDate } from '../format';
  import ClubName from './ClubName.svelte';

  let {
    fixture,
    mine = null,
    competition = false,
  }: { fixture: Fixture; mine?: number | null; competition?: boolean } = $props();
  const result = $derived(fixture.result);
  const outcome = $derived.by(() => {
    if (!result || mine === null) return '';
    const home = fixture.home.id === mine;
    const us = home ? result.goals[0] : result.goals[1];
    const them = home ? result.goals[1] : result.goals[0];
    if (us === them && result.penalties) {
      const [h, a] = result.penalties;
      return (home ? h > a : a > h) ? 'W' : 'L';
    }
    return us > them ? 'W' : us === them ? 'D' : 'L';
  });
  const after = $derived(
    result?.penalties ? `${result.penalties[0]}–${result.penalties[1]} pens` : result?.extraTime ? 'aet' : '',
  );
  const title = $derived(
    [fixture.competition.name, fixture.round, fixture.neutral ? 'neutral ground' : null].filter(Boolean).join(' · '),
  );
</script>

<div
  class="fixture"
  class:with-comp={competition}
  class:mine={mine !== null && (fixture.home.id === mine || fixture.away.id === mine)}
  {title}
>
  <span class="date dim">{shortDate(fixture.date)}</span>
  {#if competition}
    <span class="comp" class:cup={fixture.competition.kind === 'cup'}>{fixture.competition.shortName}</span>
  {/if}
  <span class="home"><ClubName club={fixture.home} short /></span>
  <span class="score {outcome}">
    {#if result}{result.goals[0]} – {result.goals[1]}{:else}v{/if}
    {#if after}<small>{after}</small>{/if}
  </span>
  <span class="away"><ClubName club={fixture.away} short /></span>
</div>

<style>
  .fixture {
    display: grid;
    grid-template-columns: 5.5em 1fr 4.5em 1fr;
    align-items: center;
    gap: 8px;
    padding: 6px 0;
  }

  .fixture.with-comp {
    grid-template-columns: 5.5em 3.5em 1fr 4.5em 1fr;
  }

  .comp {
    font-size: 0.75rem;
    text-align: center;
    border-radius: 6px;
    padding: 1px 4px;
    color: var(--text-dim);
    border: 1px solid var(--line);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .comp.cup {
    color: var(--accent);
    border-color: var(--accent);
  }

  .home {
    justify-self: end;
  }

  .score {
    text-align: center;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    border-radius: 6px;
    padding: 2px 0;
    background: var(--bg-sunken);
  }

  .score small {
    display: block;
    font-size: 0.65rem;
    font-weight: 500;
    color: var(--text-dim);
  }

  .score.W {
    color: var(--good);
  }

  .score.L {
    color: var(--bad);
  }

  @media (max-width: 520px) {
    .fixture.with-comp {
      grid-template-columns: 4.2em 2.8em 1fr 4em 1fr;
      gap: 4px;
    }
  }
</style>
