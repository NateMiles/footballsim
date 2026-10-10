<!-- One fixture: date, teams and score. -->
<script lang="ts">
  import type { Fixture } from '../api';
  import { shortDate } from '../format';
  import ClubName from './ClubName.svelte';

  let { fixture, mine = null }: { fixture: Fixture; mine?: number | null } = $props();
  const result = $derived(fixture.result);
  const outcome = $derived.by(() => {
    if (!result || mine === null) return '';
    const us = fixture.home.id === mine ? result.goals[0] : result.goals[1];
    const them = fixture.home.id === mine ? result.goals[1] : result.goals[0];
    return us > them ? 'W' : us === them ? 'D' : 'L';
  });
</script>

<div class="fixture" class:mine={mine !== null && (fixture.home.id === mine || fixture.away.id === mine)}>
  <span class="date dim">{shortDate(fixture.date)}</span>
  <span class="home"><ClubName club={fixture.home} short /></span>
  <span class="score {outcome}">
    {#if result}{result.goals[0]} – {result.goals[1]}{:else}v{/if}
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

  .score.W {
    color: var(--good);
  }

  .score.L {
    color: var(--bad);
  }
</style>
