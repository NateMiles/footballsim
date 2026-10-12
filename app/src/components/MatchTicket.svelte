<!-- A fixture as a match ticket: both crests large, the competition and
     date above, the score (or "vs") between them. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Fixture } from '../api';
  import { shortDate } from '../format';
  import Crest from './Crest.svelte';

  let {
    fixture,
    mine = null,
    today = null,
    live = false,
    children,
  }: {
    fixture: Fixture;
    mine?: number | null;
    /** The game date, to say how far off the match is. */
    today?: string | null;
    /** Today's match: lit up. */
    live?: boolean;
    children?: Snippet;
  } = $props();

  const result = $derived(fixture.result);
  const days = $derived.by(() => {
    if (!today || result) return null;
    const ms = Date.parse(`${fixture.date}T12:00:00Z`) - Date.parse(`${today}T12:00:00Z`);
    return Math.round(ms / 86_400_000);
  });
  const when = $derived(
    days === null ? null : days <= 0 ? 'Today' : days === 1 ? 'Tomorrow' : `In ${days} days`,
  );
  const outcome = $derived.by(() => {
    if (!result || mine === null) return null;
    const home = fixture.home.id === mine;
    let us = home ? result.goals[0] : result.goals[1];
    let them = home ? result.goals[1] : result.goals[0];
    if (us === them && result.penalties) {
      us = home ? result.penalties[0] : result.penalties[1];
      them = home ? result.penalties[1] : result.penalties[0];
    }
    return us > them ? 'W' : us === them ? 'D' : 'L';
  });
</script>

<div class="ticket" class:live class:cup={fixture.competition.kind === 'cup'}>
  <div class="meta">
    <span class="comp">{fixture.competition.name}{fixture.round ? ` · ${fixture.round}` : ''}</span>
    <span class="date">
      {shortDate(fixture.date)}
      {#if when}<span class="when">{when}</span>{/if}
    </span>
  </div>
  <div class="teams">
    <div class="team" class:mine={fixture.home.id === mine}>
      <Crest club={fixture.home} size={56} />
      <span class="name">{fixture.home.shortName}</span>
      <span class="venue">{fixture.neutral ? 'Neutral' : 'Home'}</span>
    </div>
    <div class="middle">
      {#if result}
        <span class="score {outcome ?? ''}">{result.goals[0]}<i>–</i>{result.goals[1]}</span>
        {#if result.penalties}
          <span class="after">{result.penalties[0]}–{result.penalties[1]} pens</span>
        {:else if result.extraTime}
          <span class="after">aet</span>
        {/if}
      {:else}
        <span class="vs">vs</span>
      {/if}
    </div>
    <div class="team" class:mine={fixture.away.id === mine}>
      <Crest club={fixture.away} size={56} />
      <span class="name">{fixture.away.shortName}</span>
      <span class="venue">Away</span>
    </div>
  </div>
  {#if children}
    <div class="actions">{@render children()}</div>
  {/if}
</div>

<style>
  .ticket {
    position: relative;
    display: grid;
    gap: 14px;
    padding: 16px 18px;
    border-radius: var(--radius-lg);
    background:
      radial-gradient(120% 140% at 50% 0%, rgba(120, 170, 255, 0.07), transparent 60%),
      var(--bg-sunken);
    border: 1px solid var(--line);
  }

  /* The perforation of a ticket stub. */
  .ticket::before,
  .ticket::after {
    content: '';
    position: absolute;
    top: 46px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--bg-raised);
    border: 1px solid var(--line);
  }

  .ticket::before {
    left: -8px;
    clip-path: inset(0 0 0 50%);
  }

  .ticket::after {
    right: -8px;
    clip-path: inset(0 50% 0 0);
  }

  .live {
    border-color: var(--accent);
    box-shadow:
      0 0 0 1px var(--accent) inset,
      0 0 40px -12px var(--accent-glow);
  }

  .meta {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding-bottom: 12px;
    border-bottom: 1px dashed var(--line-strong);
    font-size: 0.8rem;
  }

  .comp {
    font-family: var(--display);
    font-stretch: 85%;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-dim);
  }

  .cup .comp {
    color: var(--warn);
  }

  .date {
    white-space: nowrap;
    color: var(--text-dim);
    display: inline-flex;
    gap: 8px;
    align-items: center;
  }

  .when {
    color: var(--accent);
    font-weight: 650;
  }

  .teams {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 10px;
  }

  .team {
    display: grid;
    justify-items: center;
    gap: 6px;
    text-align: center;
    min-width: 0;
  }

  .name {
    font-family: var(--display);
    font-weight: 750;
    font-stretch: 100%;
    font-size: 1.02rem;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mine .name {
    color: var(--accent);
  }

  .venue {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--text-faint);
  }

  .middle {
    display: grid;
    justify-items: center;
    gap: 2px;
  }

  .vs {
    font-family: var(--display);
    font-weight: 800;
    font-stretch: 125%;
    font-size: 1.1rem;
    text-transform: uppercase;
    color: var(--text-faint);
    padding: 8px 14px;
    border: 1px solid var(--line-strong);
    border-radius: 999px;
  }

  .score {
    font-family: var(--display);
    font-weight: 800;
    font-stretch: 112%;
    font-size: 2.1rem;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.02em;
  }

  .score i {
    font-style: normal;
    color: var(--text-faint);
    margin: 0 4px;
  }

  .score.W {
    color: var(--good);
  }

  .score.L {
    color: var(--bad);
  }

  .after {
    font-size: 0.75rem;
    color: var(--text-dim);
  }

  .actions {
    display: flex;
    gap: 10px;
    justify-content: center;
    flex-wrap: wrap;
  }
</style>
