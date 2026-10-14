<!-- A story's picture, drawn from what it is about: a scoreboard, a person
     in front of their club's colours, a player between two clubs, a trophy,
     or a manager at the press-room microphones. -->
<script lang="ts">
  import type { Art, ClubRef, Expression as Expr } from '../api';
  import Crest from './Crest.svelte';
  import Portrait, { type Expression } from './Portrait.svelte';

  let { art, lead = false }: { art: Art; lead?: boolean } = $props();
  const uid = $props.id();

  const NEUTRAL = { primary: '#2a3a52', secondary: '#dfe6ee' };
  function colours(c: ClubRef | null | undefined) {
    return c?.colors ?? NEUTRAL;
  }
  const expr = (e: Expr) => e as Expression;
  const main = $derived(
    art.type === 'score'
      ? colours(art.home)
      : art.type === 'trophy'
        ? colours(art.club)
        : art.type === 'move'
          ? colours(art.to)
          : colours(art.figure.club),
  );
</script>

<div
  class="art {art.type}"
  class:lead
  style:--c1={main.primary}
  style:--c2={main.secondary}
  aria-hidden="true"
>
  {#if art.type === 'score'}
    <div class="split" style:--h={art.home.colors.primary} style:--a={art.away.colors.primary}></div>
    <div class="beams"></div>
    <div class="board">
      <div class="side"><Crest club={art.home} size={64} /><span>{art.home.shortName}</span></div>
      <div class="digits">
        <b>{art.goals[0]}</b><i></i><b>{art.goals[1]}</b>
      </div>
      <div class="side"><Crest club={art.away} size={64} /><span>{art.away.shortName}</span></div>
    </div>
    <span class="tag">{art.competition}</span>
  {:else if art.type === 'portrait'}
    <div class="stripes"></div>
    <div class="person">
      <Portrait look={art.figure.look} size={200} expression={expr(art.figure.expression)} shirt={main.primary} trim={main.secondary} />
    </div>
    {#if art.figure.club}<div class="badge"><Crest club={art.figure.club} size={44} /></div>{/if}
    <span class="tag">{art.caption}</span>
  {:else if art.type === 'move'}
    <div class="stripes"></div>
    <div class="route">
      {#if art.from}<div class="crest from"><Crest club={art.from} size={56} /></div>{/if}
      <svg class="arrow" viewBox="0 0 60 20"><path d="M2 10h50M44 3l9 7-9 7" /></svg>
      <div class="crest to"><Crest club={art.to} size={56} /></div>
    </div>
    <div class="person">
      <Portrait look={art.figure.look} size={200} expression={expr(art.figure.expression)} shirt={main.primary} trim={main.secondary} />
    </div>
    <span class="fee">{art.fee}</span>
  {:else if art.type === 'trophy'}
    <div class="burst"></div>
    <div class="confetti"></div>
    <svg class="cup" viewBox="0 0 100 120">
      <defs>
        <linearGradient id="{uid}-gold" x1="0" x2="1">
          <stop offset="0" stop-color="#b8892b" />
          <stop offset="0.45" stop-color="#ffe38a" />
          <stop offset="1" stop-color="#a87a1f" />
        </linearGradient>
      </defs>
      <path d="M26 8h48v26c0 16-10 28-24 30-14-2-24-14-24-30z" fill="url(#{uid}-gold)" />
      <path d="M26 16H12c0 16 8 24 18 26M74 16h14c0 16-8 24-18 26" fill="none" stroke="url(#{uid}-gold)" stroke-width="5" />
      <path d="M44 64h12v18H44z" fill="url(#{uid}-gold)" />
      <path d="M30 82h40v10H30zM24 92h52v14H24z" fill="url(#{uid}-gold)" />
      <path d="M30 22h40" stroke="var(--c1)" stroke-width="5" />
      <path d="M30 29h40" stroke="var(--c2)" stroke-width="3" />
    </svg>
    <div class="badge"><Crest club={art.club} size={44} /></div>
    <span class="tag">{art.competition}</span>
  {:else if art.type === 'microphone'}
    <div class="wall"></div>
    {#if art.figure.club}
      <div class="wall-crests">
        {#each Array(8) as _, i (i)}<Crest club={art.figure.club} size={28} />{/each}
      </div>
    {/if}
    <div class="person">
      <Portrait look={art.figure.look} size={200} expression={expr(art.figure.expression)} shirt="#1b2433" trim={main.primary} />
    </div>
    <svg class="mics" viewBox="0 0 200 70">
      {#each [[40, '#e5484d'], [72, '#3987e5'], [100, '#d4ff3f'], [128, '#f5a524'], [160, '#9b8afb']] as [x, c], i (i)}
        <g transform="translate({x} {i % 2 ? 6 : 0}) rotate({(i - 2) * 6})">
          <rect x="-2" y="18" width="4" height="60" fill="#2b2f36" />
          <rect x="-7" y="12" width="14" height="10" rx="2" fill={String(c)} />
          <ellipse cx="0" cy="8" rx="7" ry="9" fill="#3a3f47" />
          <ellipse cx="0" cy="6" rx="5" ry="6" fill="#4a505a" />
        </g>
      {/each}
    </svg>
  {/if}
</div>

<style>
  .art {
    position: relative;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    border-radius: var(--radius);
    container-type: inline-size;
    background:
      radial-gradient(120% 90% at 50% 0%, color-mix(in srgb, var(--c1) 55%, #0b111a), #070b11 75%);
    isolation: isolate;
  }

  .tag,
  .fee {
    position: absolute;
    left: 4cqi;
    bottom: 4cqi;
    z-index: 3;
    padding: 0.6cqi 2cqi;
    border-radius: 999px;
    background: rgba(5, 8, 13, 0.75);
    backdrop-filter: blur(6px);
    color: var(--text);
    font-family: var(--display);
    font-stretch: 90%;
    font-weight: 700;
    font-size: clamp(0.62rem, 3cqi, 0.95rem);
    letter-spacing: 0.04em;
    max-width: 70%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .fee {
    left: auto;
    right: 4cqi;
    top: 4cqi;
    bottom: auto;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: clamp(0.8rem, 4.4cqi, 1.4rem);
    font-weight: 800;
  }

  /* Score: each club's colour from its side, under floodlights. */
  .split {
    position: absolute;
    inset: 0;
    background: linear-gradient(105deg, color-mix(in srgb, var(--h) 70%, #070b11) 0 49.6%, color-mix(in srgb, var(--a) 70%, #070b11) 50.4% 100%);
    opacity: 0.85;
  }

  .beams {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(40% 70% at 15% -10%, rgba(255, 255, 255, 0.28), transparent 70%),
      radial-gradient(40% 70% at 85% -10%, rgba(255, 255, 255, 0.28), transparent 70%),
      linear-gradient(180deg, transparent 55%, rgba(5, 8, 13, 0.85));
  }

  .board {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    padding: 0 4cqi;
    gap: 2cqi;
  }

  .side {
    display: grid;
    justify-items: center;
    gap: 1.5cqi;
    min-width: 0;
  }

  .side :global(.crest) {
    width: 20cqi;
    height: 20cqi;
  }

  .side span {
    font-family: var(--display);
    font-weight: 800;
    font-size: clamp(0.7rem, 3.6cqi, 1.3rem);
    text-shadow: 0 2px 10px rgba(0, 0, 0, 0.6);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .digits {
    display: flex;
    align-items: center;
    gap: 1.6cqi;
    padding: 1.4cqi 3cqi;
    border-radius: 2.4cqi;
    background: rgba(5, 8, 13, 0.82);
    border: 1px solid rgba(255, 255, 255, 0.12);
    box-shadow: 0 0 6cqi rgba(212, 255, 63, 0.18);
  }

  .digits b {
    font-family: var(--display);
    font-weight: 900;
    font-stretch: 120%;
    font-size: clamp(1.4rem, 13cqi, 4.4rem);
    line-height: 1;
    color: var(--accent);
    text-shadow: 0 0 3cqi rgba(212, 255, 63, 0.5);
    font-variant-numeric: tabular-nums;
  }

  .digits i {
    width: 2.6cqi;
    height: 0.9cqi;
    border-radius: 1cqi;
    background: rgba(255, 255, 255, 0.4);
  }

  /* Portrait: kit stripes and a big bust. */
  .stripes {
    position: absolute;
    inset: -20%;
    background: repeating-linear-gradient(
      115deg,
      color-mix(in srgb, var(--c1) 75%, #070b11) 0 9%,
      color-mix(in srgb, var(--c2) 22%, color-mix(in srgb, var(--c1) 60%, #070b11)) 9% 13%
    );
    opacity: 0.8;
  }

  .stripes::after {
    content: '';
    position: absolute;
    inset: 0;
    background: radial-gradient(60% 80% at 50% 30%, transparent, rgba(5, 8, 13, 0.75));
  }

  .person {
    position: absolute;
    left: 50%;
    bottom: -3cqi;
    width: 58cqi;
    height: 58cqi;
    transform: translateX(-50%);
    z-index: 2;
    filter: drop-shadow(0 2cqi 4cqi rgba(0, 0, 0, 0.55));
  }

  .person :global(svg) {
    width: 100%;
    height: 100%;
  }

  .badge {
    position: absolute;
    right: 4cqi;
    top: 4cqi;
    z-index: 3;
  }

  .badge :global(.crest) {
    width: 13cqi;
    height: 13cqi;
  }

  /* A move: from one crest to the other behind the player. */
  .move .person {
    left: 50%;
    width: 52cqi;
    height: 52cqi;
  }

  .route {
    position: absolute;
    inset: 8cqi 6cqi auto;
    display: flex;
    align-items: center;
    justify-content: space-between;
    z-index: 1;
  }

  .route .crest :global(.crest) {
    width: 16cqi;
    height: 16cqi;
  }

  .route .from {
    opacity: 0.55;
    filter: grayscale(0.4);
  }

  .arrow {
    flex: 1;
    height: 5cqi;
    margin: 0 2cqi;
    stroke: var(--accent);
    stroke-width: 3;
    fill: none;
    stroke-linecap: round;
    stroke-linejoin: round;
    opacity: 0.9;
  }

  /* A trophy: gold under a burst of the club's colours. */
  .burst {
    position: absolute;
    inset: -40%;
    background: repeating-conic-gradient(
      from 0deg at 50% 50%,
      color-mix(in srgb, var(--c1) 70%, #070b11) 0 6deg,
      color-mix(in srgb, var(--c1) 40%, #070b11) 6deg 12deg
    );
    opacity: 0.85;
  }

  .confetti {
    position: absolute;
    inset: 0;
    background-image:
      radial-gradient(circle, #ffe38a 1px, transparent 1.6px),
      radial-gradient(circle, var(--c2) 1.2px, transparent 1.8px),
      radial-gradient(circle, var(--accent) 1px, transparent 1.6px);
    background-size: 23px 31px, 37px 29px, 41px 43px;
    background-position: 0 0, 11px 7px, 5px 19px;
    opacity: 0.7;
    mask-image: linear-gradient(180deg, #000, transparent 85%);
  }

  .cup {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 38cqi;
    height: 46cqi;
    transform: translate(-50%, -52%);
    filter: drop-shadow(0 0 5cqi rgba(255, 220, 120, 0.45));
  }

  /* The press room: a sponsor wall, a face and a bank of microphones. */
  .wall {
    position: absolute;
    inset: 0;
    background:
      linear-gradient(180deg, rgba(5, 8, 13, 0.2), rgba(5, 8, 13, 0.7)),
      color-mix(in srgb, var(--c1) 35%, #0d1420);
  }

  .wall-crests {
    position: absolute;
    inset: 4cqi;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    place-items: center;
    opacity: 0.22;
  }

  .wall-crests :global(.crest) {
    width: 9cqi;
    height: 9cqi;
  }

  .microphone .person {
    width: 54cqi;
    height: 54cqi;
    bottom: 2cqi;
  }

  .mics {
    position: absolute;
    left: 50%;
    bottom: -6cqi;
    width: 70cqi;
    transform: translateX(-50%);
    z-index: 3;
  }
</style>
