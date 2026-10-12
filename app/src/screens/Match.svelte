<!-- Match day: kick off, watch and manage live, then the report. -->
<script lang="ts">
  import { analysisHref } from '../state/router.svelte';
  import type { Formation, Instruction, LivePlayerInfo, LiveStart, MatchReport } from '../api';
  import Crest from '../components/Crest.svelte';
  import FixtureLine from '../components/FixtureLine.svelte';
  import { kits } from '../match/canvas2d';
  import PitchView from '../match/PitchView.svelte';
  import { Playback } from '../match/playback.svelte';
  import type { MatchScene } from '../match/renderer';
  import StatsPanel from '../match/StatsPanel.svelte';
  import TeamPanel from '../match/TeamPanel.svelte';
  import { useGame } from '../state/game.svelte';

  const game = useGame();
  const o = $derived(game.overview);
  let playback = $state<Playback | null>(null);
  let report = $state<MatchReport | null>(null);
  let tab = $state<'commentary' | 'stats' | 'team'>('commentary');
  let selected = $state<number | null>(null);
  let finishing = $state(false);

  const FORMATIONS: Formation[] = ['4-4-2', '4-3-3', '4-2-3-1', '3-5-2', '5-3-2', '4-1-4-1'];
  const SPEEDS = [1, 2, 4, 8, 16, 32];

  function sceneOf(start: LiveStart): MatchScene {
    const k = kits(
      [start.fixture.home.colors.primary, start.fixture.home.colors.secondary],
      [start.fixture.away.colors.primary, start.fixture.away.colors.secondary],
    );
    return {
      pitch: start.pitch,
      kits: { home: k.home, away: k.away },
      labels: new Map(start.players.map((p) => [p.player.id, p.player.number?.toString() ?? ''])),
    };
  }

  const scene = $derived(playback ? sceneOf(playback.start) : null);
  const players = $derived(new Map<number, LivePlayerInfo>((playback?.start.players ?? []).map((p) => [p.player.id, p])));
  const mySide = $derived(playback?.start.side === 'home' ? 0 : 1);

  async function kickOff() {
    const start = await game.run('Preparing the match', () => game.client.startMatch());
    if (start) {
      playback = new Playback(game.client, start);
      report = null;
    }
  }

  async function skip() {
    const next = await game.run('Playing the match', () => game.client.skipMatch());
    if (next) {
      game.changed(next);
      await game.autosave();
      game.router.go({ name: 'home' });
    }
  }

  async function finish() {
    if (finishing) return;
    finishing = true;
    const r = await game.run('Playing out the match', () => game.client.finishMatch());
    finishing = false;
    if (r) {
      report = r;
      playback = null;
      await game.refresh();
      await game.autosave();
    }
  }

  async function instruct(i: Instruction): Promise<string | null> {
    if (!playback) return 'No match';
    return playback.instruct(i);
  }

  function openTeam() {
    tab = 'team';
    if (playback) playback.playing = false;
  }

  const name = (id: number) => players.get(id)?.player.shortName ?? '';

  /** A name from the report's ratings. */
  function reportName(r: MatchReport, id: number): string {
    for (const side of r.ratings) {
      const p = side.find((x) => x.player.id === id);
      if (p) return p.player.shortName;
    }
    return '';
  }
</script>

{#if report}
  {@const f = report.fixture}
  <div class="report screen grid">
    <section class="card board">
      <div class="team"><Crest club={f.home} size={48} /><strong>{f.home.name}</strong></div>
      <div class="big">
        {f.result?.goals[0]} – {f.result?.goals[1]}
        {#if f.result?.penalties}
          <div class="after">{f.result.penalties[0]}–{f.result.penalties[1]} on penalties</div>
        {:else if f.result?.extraTime}
          <div class="after">after extra time</div>
        {/if}
      </div>
      <div class="team"><Crest club={f.away} size={48} /><strong>{f.away.name}</strong></div>
    </section>
    <p class="dim comp">{f.competition.name}{f.round ? ` · ${f.round}` : ''}</p>
    {#if report.shootout}
      <section class="card">
        <h3>Penalty shootout</h3>
        <div class="kicks">
          {#each ['home', 'away'] as side (side)}
            <ol class:away={side === 'away'}>
              {#each report.shootout.kicks.filter((k) => k.side === side) as k, i (i)}
                <li>
                  <span class:scored={k.scored} class:missed={!k.scored}>{k.scored ? '●' : '○'}</span>
                  {reportName(report, k.taker)}
                </li>
              {/each}
            </ol>
          {/each}
        </div>
      </section>
    {/if}
    <div class="cols">
      <section class="card">
        <h3>Goals and cards</h3>
        {#each report.events as e, i (i)}
          {#if e.type === 'goal'}
            <p>⚽ {e.minute}' {report.ratings[e.side === 'home' ? 0 : 1].find((r) => r.player.id === e.scorer)?.player.shortName ?? ''}</p>
          {:else if e.type === 'card'}
            <p><span class="card-mark" class:red={e.red}></span> {e.minute}' {report.ratings[e.side === 'home' ? 0 : 1].find((r) => r.player.id === e.player)?.player.shortName ?? ''}</p>
          {/if}
        {/each}
        <h3>Statistics</h3>
        <StatsPanel stats={report.stats} />
      </section>
      {#each [0, 1] as s (s)}
        <section class="card">
          <h3>{s === 0 ? f.home.shortName : f.away.shortName} ratings</h3>
          <table class="data">
            <tbody>
              {#each report.ratings[s] as r (r.player.id)}
                <tr>
                  <td>{r.player.shortName}</td>
                  <td class="dim">{r.minutes}'</td>
                  <td>{'⚽'.repeat(r.goals)}{'🅰'.repeat(r.assists)}</td>
                  <td class="num"><strong>{r.rating.toFixed(1)}</strong></td>
                </tr>
              {/each}
            </tbody>
          </table>
        </section>
      {/each}
    </div>
    <div class="row">
      <button class="primary" onclick={() => game.router.go({ name: 'home' })}>Continue</button>
      <button onclick={() => (location.hash = analysisHref(f))}>Full analysis</button>
    </div>
  </div>
{:else if playback && scene}
  {@const p = playback}
  {@const start = p.start}
  <div class="live">
    <header class="scoreboard">
      <button class="link" onclick={() => game.router.go({ name: 'home' })} title="Leave (the match restarts when you return)">‹</button>
      <span class="team"><Crest club={start.fixture.home} size={26} /> {start.fixture.home.shortName}</span>
      <span class="score" data-testid="score">
        {p.score[0]} – {p.score[1]}
        {#if p.penalties}<small>({p.penalties[0]}–{p.penalties[1]} pens)</small>{/if}
      </span>
      <span class="team">{start.fixture.away.shortName} <Crest club={start.fixture.away} size={26} /></span>
      <span class="clock" data-testid="clock">{p.over ? 'FT' : `${p.clock}'`}</span>
      <span class="spacer"></span>
      <button onclick={() => (p.playing = !p.playing)} aria-label={p.playing ? 'Pause' : 'Play'}>{p.playing ? '❚❚' : '▶'}</button>
      <select bind:value={p.speed} aria-label="Speed">
        {#each SPEEDS as s (s)}<option value={s}>{s}×</option>{/each}
      </select>
      {#if p.over}
        <button class="primary" onclick={finish} disabled={finishing}>Full time ▸</button>
      {:else}
        <button onclick={finish} disabled={finishing} title="Play the rest without watching">Skip to end</button>
      {/if}
    </header>
    <div class="pitch-area">
      <PitchView playback={p} {scene} {selected} />
      {#if p.error}<p class="overlay error">{p.error}</p>{/if}
    </div>
    <aside class="side">
      <div class="tabs" role="tablist">
        <button role="tab" aria-selected={tab === 'commentary'} class:active={tab === 'commentary'} onclick={() => (tab = 'commentary')}>Commentary</button>
        <button role="tab" aria-selected={tab === 'stats'} class:active={tab === 'stats'} onclick={() => (tab = 'stats')}>Stats</button>
        <button role="tab" aria-selected={tab === 'team'} class:active={tab === 'team'} onclick={openTeam}>Team</button>
      </div>
      <div class="tab-body">
        {#if tab === 'commentary'}
          <ol class="commentary" data-testid="commentary">
            {#each p.commentary as line, i (p.commentary.length - i)}
              <li class="k-{line.kind}" class:mine={line.side === start.side}>
                <span class="min">{line.clock}'</span>
                <span>{line.text}</span>
              </li>
            {/each}
          </ol>
        {:else if tab === 'stats'}
          {#if p.latest}<StatsPanel stats={p.latest.stats} />{/if}
        {:else if p.latest}
          <TeamPanel
            lineup={p.latest.lineups[mySide]}
            {players}
            shouts={start.shouts}
            formations={FORMATIONS}
            oninstruct={instruct}
            onselect={(id) => (selected = id)}
          />
          {#if !p.playing}<button class="primary resume" onclick={() => (p.playing = true)}>Resume ▶</button>{/if}
        {/if}
      </div>
      {#if selected !== null}<p class="dim sel">Selected: {name(selected)}</p>{/if}
    </aside>
  </div>
{:else}
  <div class="screen grid">
    <h1>Match day</h1>
    {#if o?.matchDay}
      <section class="card grid">
        <FixtureLine fixture={o.matchDay.fixture} mine={o.club?.id ?? null} competition />
        <p class="dim">
          {o.matchDay.fixture.competition.name}{o.matchDay.fixture.round ? ` · ${o.matchDay.fixture.round}` : ''}
          {#if o.matchDay.fixture.neutral}· neutral ground{/if}
        </p>
        <p class="dim">Check your team before kick-off. Once the match starts, the team can only change through substitutions and touchline instructions.</p>
        <div class="row">
          <button class="primary" disabled={game.busy !== null} onclick={kickOff}>Kick off ▸</button>
          <button onclick={() => game.router.go({ name: 'prepare' })}>Opposition and predictor</button>
          <button onclick={() => game.router.go({ name: 'tactics' })}>Tactics and team</button>
          <button disabled={game.busy !== null} onclick={skip}>Let the assistant manage</button>
        </div>
      </section>
    {:else}
      <p class="dim">There is no match to play today.</p>
      <div class="row"><button onclick={() => game.router.go({ name: 'home' })}>Home</button></div>
    {/if}
  </div>
{/if}

<style>
  .live {
    display: grid;
    grid-template-columns: 1fr 340px;
    grid-template-rows: auto 1fr;
    grid-template-areas: 'board board' 'pitch side';
    height: 100%;
  }

  .scoreboard {
    grid-area: board;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    padding-top: max(8px, env(safe-area-inset-top));
    background: var(--bg-raised);
    border-bottom: 1px solid var(--line);
    flex-wrap: wrap;
  }

  .scoreboard .link {
    font-size: 1.6rem;
    padding: 0 6px;
  }

  .team {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
  }

  .score {
    font-family: var(--display);
    font-size: 1.6rem;
    font-weight: 800;
    font-stretch: 112%;
    font-variant-numeric: tabular-nums;
    padding: 2px 14px;
    border-radius: 10px;
    background: var(--bg-sunken);
    border: 1px solid var(--line-strong);
    box-shadow: 0 0 24px -12px var(--accent-glow);
  }

  .clock {
    font-variant-numeric: tabular-nums;
    color: var(--accent);
    font-weight: 700;
    min-width: 4ch;
  }

  .spacer {
    flex: 1;
  }

  .pitch-area {
    grid-area: pitch;
    position: relative;
    min-height: 0;
    padding: 8px;
    background: #123d22;
  }

  .overlay {
    position: absolute;
    left: 16px;
    bottom: 16px;
    padding: 8px 12px;
    border-radius: var(--radius);
    background: rgba(0, 0, 0, 0.7);
  }

  .error {
    color: var(--bad);
  }

  .side {
    grid-area: side;
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-left: 1px solid var(--line);
    background: var(--bg-raised);
  }

  .tabs {
    display: flex;
  }

  .tabs button {
    flex: 1;
    border-radius: 0;
    border-width: 0 0 2px;
    border-color: transparent;
    background: none;
  }

  .tabs button.active {
    border-color: var(--accent);
    color: var(--accent);
  }

  .tab-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px 12px;
  }

  .commentary {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .commentary li {
    display: grid;
    grid-template-columns: 3.4em 1fr;
    gap: 6px;
    padding: 6px 0;
    border-bottom: 1px solid var(--line);
    color: var(--text-dim);
  }

  .commentary li.mine {
    color: var(--text);
  }

  .commentary .k-goal {
    color: var(--accent);
    font-weight: 700;
  }

  .commentary .k-card {
    color: var(--warn);
  }

  .commentary .k-interval,
  .commentary .k-shootout,
  .commentary .k-whistle {
    color: var(--text);
    font-weight: 600;
  }

  .min {
    font-variant-numeric: tabular-nums;
    color: var(--text-dim);
  }

  .resume {
    margin-top: 12px;
    width: 100%;
  }

  .sel {
    padding: 0 12px;
  }

  .board {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 24px;
  }

  .board .team {
    flex-direction: column;
  }

  .big {
    font-size: 2.6rem;
    font-weight: 800;
    text-align: center;
  }

  .after {
    font-size: 0.9rem;
    font-weight: 500;
    color: var(--text-dim);
  }

  .comp {
    text-align: center;
  }

  .score small {
    font-size: 0.8rem;
    font-weight: 500;
  }

  .kicks {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .kicks ol {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .kicks ol.away {
    text-align: right;
  }

  .scored {
    color: var(--good);
  }

  .missed {
    color: var(--bad);
  }

  .cols {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: var(--gap);
  }

  .card-mark {
    display: inline-block;
    width: 9px;
    height: 12px;
    background: #f5d742;
    border-radius: 1px;
  }

  .card-mark.red {
    background: var(--bad);
  }

  @media (max-width: 900px) {
    .live {
      grid-template-columns: 1fr;
      grid-template-rows: auto minmax(220px, 45vh) 1fr;
      grid-template-areas: 'board' 'pitch' 'side';
    }

    .side {
      border-left: 0;
      border-top: 1px solid var(--line);
    }
  }
</style>
