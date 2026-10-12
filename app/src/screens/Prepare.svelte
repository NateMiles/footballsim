<!-- Before the next match: what the scouts know about the opponent, and
     the match played hundreds of times with your tactic or another. -->
<script lang="ts">
  import type { Formation, Marking, Prediction, Tactic } from '../api';
  import ClubName from '../components/ClubName.svelte';
  import MatchTicket from '../components/MatchTicket.svelte';
  import Form from '../components/Form.svelte';
  import { ordinal } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  /** Runs per prediction: enough to separate tactics, quick on one thread. */
  const RUNS = 400;
  const FORMATIONS: Formation[] = ['4-4-2', '4-3-3', '4-2-3-1', '3-5-2', '5-3-2', '4-1-4-1'];

  const game = useGame();
  const report = load(() => game.client.oppositionReport());
  const tactics = load(() => game.client.tactics());
  const mine = $derived(game.overview?.club?.id ?? null);

  let trial = $state<Tactic | null>(null);
  /** Opposition instructions being edited, by player. */
  let marks = $state<Record<number, Marking>>({});
  let marksDirty = $state(false);

  $effect(() => {
    const ins = tactics.data?.instructions;
    if (!ins) return;
    marks = Object.fromEntries(ins.marking.map((m) => [m.player.id, m.marking]));
    marksDirty = false;
  });

  function setMark(player: number, value: string) {
    const next = { ...marks };
    if (value === '') delete next[player];
    else next[player] = value as Marking;
    marks = next;
    marksDirty = true;
  }

  async function saveMarks() {
    const t = tactics.data;
    if (!t) return;
    const ins = t.instructions;
    const v = await game.run('Saving instructions', () =>
      game.client.setInstructions(
        ins.roles.map((r) => ({ role: r.role, duty: r.duty })),
        { penalty: ins.penaltyTaker, freeKick: ins.freeKickTaker, corner: ins.cornerTaker },
        Object.entries(marks).map(([player, marking]) => ({ player: Number(player), marking })),
      ),
    );
    if (v) game.changed();
  }

  const markCount = $derived(Object.keys(marks).length);
  let current = $state<Prediction | null>(null);
  let alternative = $state<Prediction | null>(null);

  $effect(() => {
    if (tactics.data && trial === null) trial = { ...tactics.data.tactic };
  });

  async function predictCurrent() {
    const p = await game.run('Playing it out', () => game.client.predict(null, RUNS));
    if (p) current = p;
  }

  async function predictTrial() {
    if (!trial) return;
    const t = trial;
    const p = await game.run('Playing it out', () => game.client.predict(t, RUNS));
    if (p) alternative = p;
  }

  async function adopt() {
    if (!trial) return;
    const t = trial;
    const v = await game.run('Saving tactic', () => game.client.setTactic(t));
    if (v) {
      current = alternative;
      alternative = null;
      await game.refresh();
    }
  }

  function setSlider(key: string, value: number) {
    if (trial) trial = { ...trial, [key]: value };
    alternative = null;
  }

  /** Win, draw and loss from the manager's side. */
  function mySide(p: Prediction): { win: number; draw: number; lose: number; for: number; against: number } {
    const home = p.fixture.home.id === mine;
    return {
      win: home ? p.homeWin : p.awayWin,
      draw: p.draw,
      lose: home ? p.awayWin : p.homeWin,
      for: home ? p.goals[0] : p.goals[1],
      against: home ? p.goals[1] : p.goals[0],
    };
  }

  const pct = (x: number) => `${Math.round(x * 100)}%`;
  const edgeClass = (e: number) => (e > 0 ? 'bad' : e < 0 ? 'good' : 'even');
</script>

{#snippet outcome(p: Prediction, title: string)}
  {@const s = mySide(p)}
  <div class="pred">
    <h4>{title}</h4>
    <div class="bar" role="img" aria-label="Win {pct(s.win)}, draw {pct(s.draw)}, lose {pct(s.lose)}">
      <span class="w" style="width: {s.win * 100}%">{pct(s.win)}</span>
      <span class="d" style="width: {s.draw * 100}%">{pct(s.draw)}</span>
      <span class="l" style="width: {s.lose * 100}%">{pct(s.lose)}</span>
    </div>
    <p class="dim">
      Win · draw · lose. Goals {s.for.toFixed(1)} – {s.against.toFixed(1)} on average over {p.runs} games.
    </p>
    <p class="scores">
      {#each p.scores.slice(0, 4) as sc, i (i)}
        <span class="pill">{sc.goals[0]}–{sc.goals[1]} <span class="dim">{pct(sc.chance)}</span></span>
      {/each}
    </p>
  </div>
{/snippet}

<div class="screen grid">
  <h1>Match preparation</h1>
  {#if report.data}
    {@const r = report.data}
    {#if r.fixture}
      <div class="ticket-wrap"><MatchTicket fixture={r.fixture} {mine} today={game.overview?.date ?? null} /></div>
    {/if}
    <div class="cols">
      <section class="card grid">
        <div class="row head">
          <h2><ClubName club={r.club} /></h2>
          <span class="spacer"></span>
          {#if r.position}<span class="dim">{ordinal(r.position)} in {r.league?.name}</span>{/if}
        </div>
        <div class="row">
          <Form form={r.form} />
          <span class="dim">
            {r.record.won}W {r.record.drawn}D {r.record.lost}L · {r.record.scored}–{r.record.conceded} this season
          </span>
        </div>
        <p>
          <strong>{r.tactic.formation}</strong>
          {#each r.style as s (s)}<span class="pill">{s}</span>{/each}
        </p>
        {#if r.expectedGoals}
          <p class="dim">
            Expected goals: {r.club.shortName} {r.expectedGoals[0].toFixed(2)}, you {r.expectedGoals[1].toFixed(2)}.
          </p>
        {/if}
        <h3>How you compare</h3>
        {#each r.comparison as c (c.area)}
          <div class="cmp {edgeClass(c.edge)}">
            <span class="area">{c.area}</span>
            <span class="meter" aria-hidden="true">
              {#each [-2, -1, 0, 1, 2] as k (k)}<span class:on={k === c.edge}></span>{/each}
            </span>
            <span>{c.verdict}</span>
          </div>
        {/each}
        <p class="dim small">
          Your scouts know their players {r.knowledge < 35 ? 'only a little' : r.knowledge < 60 ? 'fairly well' : 'well'}
          ({r.knowledge}%). {r.knowledge < 35 ? 'Treat the comparison as a rough guide; scouting them would sharpen it.' : ''}
        </p>
      </section>

      <section class="card">
        <h3>Likely eleven</h3>
        <table class="data compact">
          <tbody>
            {#each r.likelyXi as p (p.player.id)}
              <tr>
                <td class="dim">{p.position}</td>
                <td><a href="#/player/{p.player.id}">{p.player.name}</a></td>
                <td class="num">{p.ability}</td>
                <td class="num dim">{p.goals > 0 ? `${p.goals} gls` : ''}</td>
                {#if tactics.data}
                  <td>
                    <select
                      aria-label="Instruction for {p.player.name}"
                      value={marks[p.player.id] ?? ''}
                      disabled={!marks[p.player.id] && markCount >= tactics.data.instructions.maxMarking}
                      onchange={(e) => setMark(p.player.id, e.currentTarget.value)}
                    >
                      <option value="">–</option>
                      {#each tactics.data.instructions.markings as m (m.marking)}
                        <option value={m.marking}>{m.name}</option>
                      {/each}
                    </select>
                  </td>
                {/if}
              </tr>
            {/each}
          </tbody>
        </table>
        {#if tactics.data}
          <div class="row marks">
            <span class="dim small">
              Up to {tactics.data.instructions.maxMarking} instructions, for this match only.
              {#each tactics.data.instructions.markings as m (m.marking)}<br />{m.name}: {m.description}{/each}
            </span>
            <button class="primary" disabled={!marksDirty || game.busy !== null} onclick={saveMarks}>Save</button>
          </div>
        {/if}
        <h3>Players to watch</h3>
        {#each r.keyPlayers as k (k.player.id)}
          <p><a href="#/player/{k.player.id}">{k.player.name}</a> <span class="dim">– {k.reason}</span></p>
        {/each}
        {#if r.absent.length > 0}
          <h3>Missing</h3>
          {#each r.absent as k (k.player.id)}
            <p><a href="#/player/{k.player.id}">{k.player.name}</a> <span class="dim">– {k.reason}</span></p>
          {/each}
        {/if}
      </section>
    </div>

    <section class="card grid">
      <h2>Predictor</h2>
      <p class="dim">
        Plays the match {RUNS} times on the full match engine against their likely team. It never changes what will
        happen in the real match.
      </p>
      <div class="cols">
        <div>
          {#if current}
            {@render outcome(current, `Your tactic (${current.tactic.formation})`)}
          {:else}
            <button class="primary" disabled={game.busy !== null} onclick={predictCurrent}>Predict with your tactic</button>
          {/if}
        </div>
        <div>
          {#if alternative}
            {@render outcome(alternative, `Trial tactic (${alternative.tactic.formation})`)}
          {/if}
        </div>
      </div>
      {#if trial && tactics.data}
        <details>
          <summary>Try another tactic</summary>
          <div class="trial grid">
            <label class="field">
              <span>Formation</span>
              <select
                value={trial.formation}
                onchange={(e) => {
                  trial = { ...trial!, formation: e.currentTarget.value as Formation };
                  alternative = null;
                }}
              >
                {#each FORMATIONS as f (f)}<option value={f}>{f}</option>{/each}
              </select>
            </label>
            {#each tactics.data.sliders as s (s.key)}
              <label class="slider">
                <span class="sname">{s.name}</span>
                <span class="dim">{s.low}</span>
                <input
                  type="range"
                  min="-2"
                  max="2"
                  step="1"
                  value={(trial as unknown as Record<string, number>)[s.key]}
                  oninput={(e) => setSlider(s.key, Number(e.currentTarget.value))}
                />
                <span class="dim">{s.high}</span>
              </label>
            {/each}
            <div class="row">
              <button disabled={game.busy !== null} onclick={predictTrial}>Predict with this tactic</button>
              {#if alternative}<button class="primary" onclick={adopt}>Use this tactic</button>{/if}
            </div>
          </div>
        </details>
      {/if}
    </section>
  {:else}
    <p class="dim">No match to prepare for.</p>
  {/if}
</div>

<style>
  .ticket-wrap {
    max-width: 560px;
  }

  .cols {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: var(--gap);
  }

  .head h2 {
    margin: 0;
  }

  .spacer {
    flex: 1;
  }

  .cmp {
    display: grid;
    grid-template-columns: 7em 6em 1fr;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
  }

  .meter {
    display: inline-flex;
    gap: 2px;
  }

  .meter span {
    width: 14px;
    height: 8px;
    border-radius: 2px;
    background: var(--bg-sunken);
  }

  .cmp.bad .meter span.on {
    background: var(--bad);
  }

  .cmp.good .meter span.on {
    background: var(--good);
  }

  .cmp.even .meter span.on {
    background: var(--text-dim);
  }

  .bar {
    display: flex;
    height: 28px;
    border-radius: 6px;
    overflow: hidden;
    font-size: 0.8rem;
    font-weight: 700;
  }

  .bar span {
    display: grid;
    place-items: center;
    overflow: hidden;
    color: #0b0f0c;
  }

  .bar .w {
    background: var(--good);
  }

  .bar .d {
    background: var(--text-dim);
  }

  .bar .l {
    background: var(--bad);
  }

  .scores {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .slider {
    display: grid;
    grid-template-columns: 9em 6em 1fr 6em;
    align-items: center;
    gap: 8px;
  }

  .small {
    font-size: 0.85rem;
  }

  .marks {
    align-items: flex-end;
    justify-content: space-between;
    margin-top: 8px;
  }

  @media (max-width: 600px) {
    .slider {
      grid-template-columns: 1fr;
    }
  }
</style>
