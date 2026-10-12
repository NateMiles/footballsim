<!-- One match analysed: expected goals over time, possession by spell, the
     shot map, pass networks, where each side had the ball, and the
     players' numbers. Home is blue and attacks left to right; away is
     orange. -->
<script lang="ts">
  import type { MatchAnalysisView, PlayerLine, SideAnalysisView } from '../api';
  import ClubName from '../components/ClubName.svelte';
  import { longDate } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  let { kind, id, index }: { kind: 'league' | 'cup'; id: number; index: number } = $props();
  const game = useGame();
  const view = load(() => game.client.matchAnalysis({ kind, id, index }));

  // Pitch in metres; home attacks to the right.
  const W = 105;
  const H = 68;
  const SIDE = ['home', 'away'] as const;

  let tip = $state<{ x: number; y: number; text: string } | null>(null);
  function show(e: MouseEvent | FocusEvent, text: string) {
    const t = e.currentTarget as Element;
    const r = t.getBoundingClientRect();
    tip = { x: r.left + r.width / 2, y: r.top, text };
  }

  /** A shot's spot: a zone centre, spread a little so shots do not stack. */
  function spot(side: 'home' | 'away', band: number, lane: number, k: number): [number, number] {
    // Shots from the box spread over it; elsewhere over the zone.
    const box = band === 5;
    const jx = (((k * 37) % 11) - 5) * (box ? 1.3 : 0.9);
    const jy = (((k * 53) % 13) - 6) * (box ? 1.6 : 1.1);
    const x = ((band + 0.5) / 6) * W + jx;
    const y = ((lane + 0.5) / 3) * H + jy;
    return side === 'home' ? [x, y] : [W - x, H - y];
  }

  // xG timeline geometry.
  const CW = 600;
  const CH = 180;
  const PAD = { l: 34, r: 56, t: 10, b: 24 };
  let cross = $state<number | null>(null);

  function xgChart(a: MatchAnalysisView) {
    const end = Math.max(90, ...a.shots.map((s) => s.minute + 1));
    const top = Math.max(1, ...a.xg.map((p) => Math.max(p.home, p.away)));
    const ymax = Math.ceil(top * 2) / 2;
    const x = (m: number) => PAD.l + (m / end) * (CW - PAD.l - PAD.r);
    const y = (v: number) => CH - PAD.b - (v / ymax) * (CH - PAD.t - PAD.b);
    const path = (key: 'home' | 'away') => {
      let d = `M${x(0)},${y(0)}`;
      for (const p of a.xg.slice(1)) d += ` H${x(p.minute)} V${y(p[key])}`;
      return d + ` H${x(end)}`;
    };
    const ticks = [];
    for (let m = 0; m <= end; m += 15) ticks.push(m);
    const yticks = [];
    for (let v = 0; v <= ymax + 1e-9; v += 0.5) yticks.push(v);
    const last = a.xg[a.xg.length - 1];
    // End labels, kept apart when the totals are close.
    let [lh, la] = [y(last.home), y(last.away)];
    if (Math.abs(lh - la) < 13) {
      const mid = (lh + la) / 2;
      const up = last.home >= last.away;
      lh = mid + (up ? -7 : 7);
      la = mid + (up ? 7 : -7);
    }
    return { end, x, y, path, ticks, yticks, last, lh, la };
  }

  /** Cumulative xG at a minute. */
  function xgAt(a: MatchAnalysisView, minute: number) {
    let p = a.xg[0];
    for (const q of a.xg) if (q.minute <= minute) p = q;
    return p;
  }

  function onMove(e: PointerEvent, end: number) {
    const svg = e.currentTarget as SVGSVGElement;
    const r = svg.getBoundingClientRect();
    const px = ((e.clientX - r.left) / r.width) * CW;
    const m = ((px - PAD.l) / (CW - PAD.l - PAD.r)) * end;
    cross = m >= 0 && m <= end ? Math.round(m) : null;
  }

  function net(side: SideAnalysisView) {
    const at = new Map(side.nodes.map((n) => [n.player.id, n]));
    const maxCount = Math.max(1, ...side.links.map((l) => l.count));
    const maxInv = Math.max(1, ...side.nodes.map((n) => n.involvements));
    // The eleven most involved, so substitutes with a few touches do not
    // crowd the picture.
    const nodes = [...side.nodes].sort((a, b) => b.involvements - a.involvements).slice(0, 11);
    const shown = new Set(nodes.map((n) => n.player.id));
    const pos = (n: { band: number; lane: number }): [number, number] => [
      ((n.band + 0.5) / 6) * W,
      ((n.lane + 0.5) / 3) * H,
    ];
    const links = side.links
      .filter((l) => shown.has(l.from) && shown.has(l.to))
      .map((l) => ({ ...l, a: pos(at.get(l.from)!), b: pos(at.get(l.to)!), w: 0.4 + (2.6 * l.count) / maxCount }));
    return { nodes: nodes.map((n) => ({ n, p: pos(n), r: 1.4 + (2.6 * n.involvements) / maxInv })), links };
  }

  const pct = (x: number) => `${Math.round(x * 100)}%`;
  const total = (side: PlayerLine[]) => side.reduce((s, p) => s + p.minutes, 0);
</script>

{#snippet pitch()}
  <rect x="0" y="0" width={W} height={H} class="turf" />
  <line x1={W / 2} y1="0" x2={W / 2} y2={H} class="mark" />
  <circle cx={W / 2} cy={H / 2} r="9.15" class="mark" />
  <rect x="0" y={H / 2 - 20.15} width="16.5" height="40.3" class="mark" />
  <rect x={W - 16.5} y={H / 2 - 20.15} width="16.5" height="40.3" class="mark" />
{/snippet}

{#if view.data}
  {@const a = view.data}
  {@const f = a.fixture}
  {@const c = xgChart(a)}
  <div class="screen grid">
    <section class="card head">
      <span class="dim">{f.competition.name}{f.round ? ` · ${f.round}` : ''} · {longDate(f.date)}</span>
      <div class="score">
        <span class="team home"><ClubName club={f.home} /></span>
        <strong>{a.goals[0]} – {a.goals[1]}</strong>
        <span class="team away"><ClubName club={f.away} /></span>
      </div>
      <span class="dim">xG {a.stats[0].xg.toFixed(2)} – {a.stats[1].xg.toFixed(2)} · possession {pct(a.stats[0].possession)} – {pct(a.stats[1].possession)}</span>
    </section>

    <div class="cols">
      <section class="card">
        <h3>Expected goals</h3>
        <div class="legend">
          <span><i class="sw home"></i>{f.home.shortName}</span><span><i class="sw away"></i>{f.away.shortName}</span>
        </div>
        <svg
          viewBox="0 0 {CW} {CH}"
          class="chart"
          role="img"
          aria-label="Cumulative expected goals: {f.home.shortName} {c.last.home.toFixed(2)}, {f.away.shortName} {c.last.away.toFixed(2)}"
          onpointermove={(e) => onMove(e, c.end)}
          onpointerleave={() => (cross = null)}
        >
          {#each c.yticks as v (v)}
            <line x1={PAD.l} x2={CW - PAD.r} y1={c.y(v)} y2={c.y(v)} class="grid-line" />
            <text x={PAD.l - 6} y={c.y(v) + 4} class="axis" text-anchor="end">{v.toFixed(1)}</text>
          {/each}
          {#each c.ticks as m (m)}
            <text x={c.x(m)} y={CH - 6} class="axis" text-anchor="middle">{m}'</text>
          {/each}
          <path d={c.path('home')} class="line home" />
          <path d={c.path('away')} class="line away" />
          {#each a.shots.filter((s) => s.outcome === 'goal') as s, k (k)}
            {@const p = xgAt(a, s.minute)}
            <circle cx={c.x(s.minute)} cy={c.y(s.side === 'home' ? p.home : p.away)} r="5" class="goal {s.side}" />
          {/each}
          <text x={CW - PAD.r + 6} y={c.lh + 4} class="label">{c.last.home.toFixed(2)}</text>
          <text x={CW - PAD.r + 6} y={c.la + 4} class="label">{c.last.away.toFixed(2)}</text>
          {#if cross !== null}
            {@const p = xgAt(a, cross)}
            <line x1={c.x(cross)} x2={c.x(cross)} y1={PAD.t} y2={CH - PAD.b} class="crosshair" />
            <g transform="translate({Math.min(c.x(cross) + 8, CW - 150)},{PAD.t + 4})">
              <rect width="130" height="44" rx="4" class="tipbox" />
              <text x="8" y="16" class="tiptext">{cross}'</text>
              <text x="8" y="34" class="tiptext">{f.home.shortName} {p.home.toFixed(2)} · {f.away.shortName} {p.away.toFixed(2)}</text>
            </g>
          {/if}
        </svg>
        <h3 class="gap">Possession by spell</h3>
        <div class="spells" role="img" aria-label="Home possession by 15-minute spell">
          {#each a.possession as share, k (k)}
            <div class="spell" title="{k * 15}–{(k + 1) * 15}': {f.home.shortName} {pct(share)}, {f.away.shortName} {pct(1 - share)}">
              <span class="home" style="flex: {share}"></span><span class="away" style="flex: {1 - share}"></span>
              <small class="dim">{k * 15}'</small>
            </div>
          {/each}
        </div>
      </section>

      <section class="card">
        <h3>Shots</h3>
        <p class="dim small">Filled: goal. Size: chance quality (xG). {f.home.shortName} attack to the right.</p>
        <svg viewBox="-2 -2 {W + 4} {H + 4}" class="pitch" role="img" aria-label="Shot map">
          {@render pitch()}
          {#each a.shots as s, k (k)}
            {@const [x, y] = spot(s.side, s.band, s.lane, k)}
            <circle
              cx={x}
              cy={y}
              r={1.2 + Math.sqrt(s.xg) * 4}
              class="shot {s.side}"
              class:scored={s.outcome === 'goal'}
              tabindex="0"
              role="button"
              aria-label="{s.minute}' {s.player.name}, {s.kind}, xG {s.xg.toFixed(2)}, {s.outcome}"
              onpointerenter={(e) => show(e, `${s.minute}' ${s.player.shortName} · ${s.kind} · xG ${s.xg.toFixed(2)} · ${s.outcome}`)}
              onpointerleave={() => (tip = null)}
              onfocus={(e) => show(e, `${s.minute}' ${s.player.shortName} · ${s.kind} · xG ${s.xg.toFixed(2)} · ${s.outcome}`)}
              onblur={() => (tip = null)}
            />
          {/each}
        </svg>
      </section>
    </div>

    <div class="cols">
      {#each a.sides as side, s (s)}
        {@const n = net(side)}
        {@const maxTouch = Math.max(1, ...side.touches.flat())}
        <section class="card">
          <h3><ClubName club={side.club} /></h3>
          <p class="dim small">Pass network: average positions, attacking right; thicker lines for more passes.</p>
          <svg viewBox="-2 -2 {W + 4} {H + 4}" class="pitch" role="img" aria-label="Pass network for {side.club.name}">
            {@render pitch()}
            {#each n.links as l, k (k)}
              <line x1={l.a[0]} y1={l.a[1]} x2={l.b[0]} y2={l.b[1]} class="link {SIDE[s]}" stroke-width={l.w}>
                <title>{l.count} passes</title>
              </line>
            {/each}
            {#each n.nodes as { n: node, p, r } (node.player.id)}
              <circle cx={p[0]} cy={p[1]} {r} class="node {SIDE[s]}"><title>{node.player.name}: {node.involvements} involvements</title></circle>
              <text x={p[0]} y={p[1] - r - 1} class="name" text-anchor="middle">{node.player.shortName.split(' ').pop()}</text>
            {/each}
          </svg>
          <p class="dim small gap">Where they had the ball (actions by zone).</p>
          <svg viewBox="0 0 {W} {H}" class="pitch zones" role="img" aria-label="Actions by zone for {side.club.name}">
            {#each side.touches as bandRow, b (b)}
              {#each bandRow as count, l (l)}
                <rect
                  x={(b / 6) * W + 0.5}
                  y={(l / 3) * H + 0.5}
                  width={W / 6 - 1}
                  height={H / 3 - 1}
                  rx="1"
                  class="zone {SIDE[s]}"
                  style="opacity: {0.12 + (0.88 * count) / maxTouch}"
                ><title>{count} actions</title></rect>
                <text x={((b + 0.5) / 6) * W} y={((l + 0.5) / 3) * H + 2} class="zonelabel" text-anchor="middle">{count}</text>
              {/each}
            {/each}
          </svg>
        </section>
      {/each}
    </div>

    <div class="cols">
      {#each a.players as side, s (s)}
        <section class="card scroll-x">
          <h3>{s === 0 ? f.home.name : f.away.name} <span class="dim small">{total(side)} minutes</span></h3>
          <table class="data compact">
            <thead>
              <tr><th>Player</th><th class="num">Min</th><th class="num">G</th><th class="num">A</th><th class="num">Sh</th><th class="num">xG</th><th class="num">Pass</th><th class="num">Key</th><th class="num">Tkl</th><th class="num">Rtg</th></tr>
            </thead>
            <tbody>
              {#each side as p (p.player.id)}
                <tr>
                  <td><a href="#/player/{p.player.id}">{p.player.shortName}</a></td>
                  <td class="num">{p.minutes}</td>
                  <td class="num">{p.goals || ''}</td>
                  <td class="num">{p.assists || ''}</td>
                  <td class="num">{p.shots || ''}</td>
                  <td class="num">{p.xg > 0 ? p.xg.toFixed(2) : ''}</td>
                  <td class="num">{p.passesCompleted}/{p.passes}</td>
                  <td class="num">{p.keyPasses || ''}</td>
                  <td class="num">{p.tackles || ''}</td>
                  <td class="num">{p.rating.toFixed(1)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </section>
      {/each}
    </div>
  </div>
  {#if tip}
    <div class="tooltip" style="left: {tip.x}px; top: {tip.y}px">{tip.text}</div>
  {/if}
{:else}
  <div class="screen"><p class="dim">Loading the analysis…</p></div>
{/if}

<style>
  .cols {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: var(--gap);
  }

  .head {
    display: grid;
    justify-items: center;
    gap: 4px;
  }

  .score {
    display: flex;
    align-items: center;
    gap: 16px;
    font-size: 1.4rem;
  }

  .small {
    font-size: 0.85rem;
  }

  .gap {
    margin-top: 12px;
  }

  h3 {
    margin: 0 0 6px;
  }

  .legend {
    display: flex;
    gap: 14px;
    font-size: 0.85rem;
    color: var(--text-dim);
  }

  .sw {
    display: inline-block;
    width: 12px;
    height: 3px;
    border-radius: 2px;
    margin-right: 6px;
    vertical-align: middle;
  }

  .sw.home,
  .spell .home {
    background: var(--series-home);
  }

  .sw.away,
  .spell .away {
    background: var(--series-away);
  }

  .chart {
    width: 100%;
    height: auto;
    touch-action: pan-y;
  }

  .grid-line {
    stroke: var(--line);
    stroke-width: 1;
  }

  .axis,
  .label {
    fill: var(--text-dim);
    font-size: 11px;
  }

  .label {
    fill: var(--text);
  }

  .line {
    fill: none;
    stroke-width: 2;
  }

  .line.home {
    stroke: var(--series-home);
  }

  .line.away {
    stroke: var(--series-away);
  }

  .goal {
    stroke: var(--bg-raised);
    stroke-width: 2;
  }

  .goal.home {
    fill: var(--series-home);
  }

  .goal.away {
    fill: var(--series-away);
  }

  .crosshair {
    stroke: var(--text-dim);
    stroke-width: 1;
    stroke-dasharray: 3 3;
  }

  .tipbox {
    fill: var(--bg-sunken);
    stroke: var(--line);
  }

  .tiptext {
    fill: var(--text);
    font-size: 11px;
  }

  .spells {
    display: flex;
    gap: 2px;
  }

  .spell {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    height: 34px;
    align-content: flex-start;
  }

  .spell span {
    height: 18px;
    border-radius: 4px;
  }

  .spell small {
    flex-basis: 100%;
    font-size: 0.7rem;
  }

  .pitch {
    width: 100%;
    height: auto;
    display: block;
  }

  .turf {
    fill: var(--bg-sunken);
    stroke: var(--line);
    stroke-width: 0.5;
  }

  .mark {
    fill: none;
    stroke: var(--line);
    stroke-width: 0.5;
  }

  .shot {
    fill: none;
    stroke-width: 0.8;
    cursor: pointer;
  }

  .shot.home {
    stroke: var(--series-home);
  }

  .shot.away {
    stroke: var(--series-away);
  }

  .shot.scored.home {
    fill: var(--series-home);
    stroke: var(--bg-raised);
  }

  .shot.scored.away {
    fill: var(--series-away);
    stroke: var(--bg-raised);
  }

  .shot:focus-visible {
    outline: none;
    stroke: var(--text);
  }

  .link {
    stroke-linecap: round;
    opacity: 0.55;
  }

  .link.home,
  .node.home,
  .zone.home {
    stroke: var(--series-home);
    fill: var(--series-home);
  }

  .link.away,
  .node.away,
  .zone.away {
    stroke: var(--series-away);
    fill: var(--series-away);
  }

  .node {
    stroke: var(--bg-raised) !important;
    stroke-width: 0.5;
  }

  .zone {
    stroke: none !important;
  }

  .name,
  .zonelabel {
    fill: var(--text);
    font-size: 3px;
  }

  .zonelabel {
    font-size: 4px;
  }

  .tooltip {
    position: fixed;
    transform: translate(-50%, calc(-100% - 8px));
    background: var(--bg-sunken);
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 0.8rem;
    pointer-events: none;
    white-space: nowrap;
    z-index: 10;
  }
</style>
