<!-- One fixture: date, teams and score, with the competition when asked. -->
<script lang="ts">
  import type { Fixture } from '../api';
  import { shortDate } from '../format';
  import ClubName from './ClubName.svelte';

  let {
    fixture,
    mine = null,
    competition = false,
    links = true,
  }: {
    fixture: Fixture;
    mine?: number | null;
    competition?: boolean;
    /** Club names link to their squads; off inside another link. */
    links?: boolean;
  } = $props();
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
  <span class="home"><ClubName club={fixture.home} short link={links} /></span>
  <span class="score {outcome}">
    {#if result}{result.goals[0]}<i>–</i>{result.goals[1]}{:else}<span class="vs">vs</span>{/if}
    {#if after}<small>{after}</small>{/if}
  </span>
  <span class="away"><ClubName club={fixture.away} short link={links} /></span>
</div>

<style>
  .fixture {
    display: grid;
    grid-template-columns: 5em minmax(0, 1fr) 4.5em minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    padding: 7px 0;
    max-width: 760px;
  }

  .fixture.with-comp {
    grid-template-columns: 5em 4.2em minmax(0, 1fr) 4.5em minmax(0, 1fr);
  }

  .date {
    font-variant-numeric: tabular-nums;
  }

  .comp {
    font-family: var(--display);
    font-stretch: 85%;
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    text-align: center;
    border-radius: 999px;
    padding: 2px 6px;
    color: var(--text-dim);
    background: rgba(255, 255, 255, 0.04);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .comp.cup {
    color: var(--warn);
    background: var(--warn-soft);
  }

  .home,
  .away {
    display: flex;
    align-items: center;
    min-width: 0;
  }

  .home {
    justify-content: flex-end;
  }

  .home :global(.club),
  .away :global(.club) {
    max-width: 100%;
  }

  .home :global(.club > span),
  .away :global(.club > span) {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .home :global(.club) {
    flex-direction: row-reverse;
  }

  .score {
    text-align: center;
    font-family: var(--display);
    font-weight: 800;
    font-stretch: 105%;
    font-size: 1.02rem;
    font-variant-numeric: tabular-nums;
    border-radius: 8px;
    padding: 3px 0;
    background: var(--bg-sunken);
    border: 1px solid var(--line);
  }

  .score i {
    font-style: normal;
    color: var(--text-faint);
    margin: 0 3px;
  }

  .vs {
    font-size: 0.75rem;
    font-weight: 700;
    font-stretch: 115%;
    text-transform: uppercase;
    color: var(--text-faint);
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

  /* Phones: no competition column; the tooltip still names it. */
  @media (max-width: 520px) {
    .fixture,
    .fixture.with-comp {
      grid-template-columns: 3.6em minmax(0, 1fr) 3.8em minmax(0, 1fr);
      gap: 6px;
    }

    .comp {
      display: none;
    }
  }
</style>
