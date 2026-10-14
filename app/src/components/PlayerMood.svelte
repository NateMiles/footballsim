<!-- A player as a person: character, and for your own players their mood
     with its reasons, their role, promises, and a private word. -->
<script lang="ts">
  import type { PlayerView, SquadStatus } from '../api';
  import { longDate } from '../format';
  import { useGame } from '../state/game.svelte';

  let { p }: { p: PlayerView } = $props();
  const game = useGame();

  const STATUSES: { status: SquadStatus; name: string }[] = [
    { status: 'key', name: 'Key player' },
    { status: 'regular', name: 'Regular starter' },
    { status: 'rotation', name: 'Squad rotation' },
    { status: 'backup', name: 'Backup' },
    { status: 'prospect', name: 'Prospect' },
  ];

  const moodClass = (h: number) => (h >= 15 ? 'good' : h <= -15 ? 'bad' : h < 0 ? 'warn' : '');
  const signed = (n: number) => (n > 0 ? `+${n}` : `${n}`);


  async function act(label: string, f: () => Promise<unknown>) {
    const r = await game.run(label, f);
    if (r) game.changed();
  }

</script>

{#if p.personality || p.people}
  <section class="card grid mood">
    <h3>Person</h3>
    {#if p.personality}
      <p><strong>{p.personality.name}</strong> <span class="dim">– {p.personality.description}</span></p>
    {/if}
    {#if p.people}
      {@const pp = p.people}
      <div class="row wrap">
        <span class="pill {moodClass(pp.mood.happiness)}" data-testid="mood">{pp.mood.label}</span>
        {#if pp.captain}<span class="pill">Captain</span>{/if}
        {#if pp.influential}<span class="pill">Influential</span>{/if}
        <label class="field inline">
          <span>Status</span>
          <select
            value={pp.status}
            disabled={game.busy !== null}
            onchange={(e) => act('Saving', () => game.client.setSquadStatus(p.id, e.currentTarget.value as SquadStatus))}
          >
            {#each STATUSES as s (s.status)}<option value={s.status}>{s.name}</option>{/each}
          </select>
        </label>
        <span class="dim">Playing time {pp.playingTime}% (expects {pp.expected}%)</span>
        {#if !pp.captain}
          <button disabled={game.busy !== null} onclick={() => act('Saving', () => game.client.setCaptain(p.id))}>
            Make captain
          </button>
        {/if}
      </div>
      {#if pp.mood.reasons.length > 0}
        <ul class="reasons">
          {#each pp.mood.reasons as r (r.text)}
            <li class={r.effect > 0 ? 'up' : 'down'}><span>{r.text}</span> <span class="dim">{signed(r.effect)}</span></li>
          {/each}
        </ul>
      {/if}
      {#if pp.request && !pp.request.accepted}
        <div class="row wrap">
          <strong>Asked to leave on {longDate(pp.request.date)}.</strong>
          <button disabled={game.busy !== null} onclick={() => act('Answering', () => game.client.answerTransferRequest(p.id, true))}>
            List for sale
          </button>
          <button disabled={game.busy !== null} onclick={() => act('Answering', () => game.client.answerTransferRequest(p.id, false))}>
            Refuse
          </button>
        </div>
      {:else if pp.request}
        <p class="dim">Listed for sale at their own request.</p>
      {/if}
      {#each pp.promises as pr (pr.kind)}
        <p>
          Promised: {pr.text}, by {longDate(pr.due)}.
          <span class="pill {pr.onTrack ? 'good' : 'warn'}">{pr.onTrack ? 'On track' : 'Behind'}</span>
        </p>
      {/each}
      <div class="row wrap word">
        <a class="talk-link" href="#/talk/{p.id}">Have a word with {p.firstName}</a>
        {#if !pp.canTalk && pp.lastTalk}
          <span class="dim small">Last proper talk {longDate(pp.lastTalk)}</span>
        {/if}
      </div>
    {/if}
  </section>
{/if}

<style>
  .mood h3 {
    margin: 0;
  }

  .wrap {
    flex-wrap: wrap;
  }

  .inline {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .reasons {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 2px;
  }

  .reasons li {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 2px 8px;
    border-left: 3px solid var(--line);
  }

  .reasons li.up {
    border-left-color: var(--good);
  }

  .reasons li.down {
    border-left-color: var(--bad);
  }

  .talk-link {
    display: inline-flex;
    align-items: center;
    min-height: var(--tap);
    padding: 0 18px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 700;
  }

  .talk-link:hover {
    color: var(--accent-ink);
    background: #e2ff70;
  }
</style>
