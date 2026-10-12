<!-- No game loaded: start a new one or load a save. -->
<script lang="ts">
  import type { ClubChoice, WorldSize } from '../api';
  import Benchmark from '../components/Benchmark.svelte';
  import Crest from '../components/Crest.svelte';
  import Icon from '../components/Icon.svelte';
  import { useGame } from '../state/game.svelte';
  import type { SaveSlot } from '../storage/saves';
  import { longDate } from '../format';

  const game = useGame();
  let step = $state<'start' | 'world' | 'club'>('start');
  let seed = $state(Math.floor(Math.random() * 1_000_000));
  let size = $state<WorldSize>('tiny');
  let choices = $state<ClubChoice[]>([]);
  let picked = $state<ClubChoice | null>(null);
  let firstName = $state('');
  let lastName = $state('');
  let slots = $state<SaveSlot[]>([]);
  let leagueFilter = $state<number | 'all'>('all');
  let testing = $state(false);

  const SIZES: { value: WorldSize; title: string; detail: string }[] = [
    { value: 'tiny', title: 'One league', detail: '20 clubs. Quick to play.' },
    { value: 'small', title: 'A football world', detail: '24 nations, 120 leagues, continental cups.' },
  ];

  $effect(() => {
    game.saves.list().then((s) => (slots = s)).catch(game.fail);
  });

  const leagues = $derived([...new Map(choices.map((c) => [c.league.id, c.league])).values()]);
  const shown = $derived(leagueFilter === 'all' ? choices : choices.filter((c) => c.league.id === leagueFilter));

  async function generate() {
    const c = await game.run('Generating the world', () => game.client.newWorld(seed, size));
    if (c) {
      choices = c.clubs;
      picked = null;
      step = 'club';
    }
  }

  async function start() {
    if (!picked) return;
    const club = picked.club.id;
    const o = await game.run('Starting', () =>
      game.client.startCareer(club, firstName.trim() || 'Alex', lastName.trim() || 'Manager'),
    );
    if (o) {
      await game.enter(o);
      await game.autosave();
    }
  }

  async function load(slot: SaveSlot) {
    const save = await game.saves.get(slot.name);
    if (!save) return;
    const o = await game.run('Loading', () => game.client.load(save.data));
    if (o) await game.enter(o);
  }
</script>

<div class="title">
  <!-- A pitch under lights, seen from the stand. -->
  <div class="stadium" aria-hidden="true">
    <div class="lights"></div>
    <svg class="pitch" viewBox="0 0 1050 680" preserveAspectRatio="xMidYMid slice">
      <g fill="none" stroke="currentColor" stroke-width="3">
        <rect x="20" y="20" width="1010" height="640" />
        <line x1="525" y1="20" x2="525" y2="660" />
        <circle cx="525" cy="340" r="92" />
        <rect x="20" y="138" width="165" height="404" />
        <rect x="865" y="138" width="165" height="404" />
        <rect x="20" y="248" width="55" height="184" />
        <rect x="975" y="248" width="55" height="184" />
        <path d="M185 266a92 92 0 0 1 0 148M865 266a92 92 0 0 0 0 148" />
      </g>
      <circle cx="525" cy="340" r="5" fill="currentColor" />
    </svg>
  </div>

  <header class="intro">
    <span class="mark"><Icon name="ball" size={30} stroke={1.5} /></span>
    <h1 class="logo">football<b>sim</b></h1>
    <p class="tagline">Run a club. Shape a career. A whole football world, alive on your device.</p>
    <ul class="features">
      <li><span>01</span>Matches played out action by action, watched live in 2D</li>
      <li><span>02</span>Leagues and cups across two dozen nations</li>
      <li><span>03</span>Players with moods, agents and ambitions</li>
    </ul>
  </header>

  <div class="panel">
    {#if game.error}
      <p class="error" role="alert">{game.error}</p>
    {/if}

    {#if step !== 'start'}
      <ol class="steps" aria-label="Steps">
        <li class:done={step === 'club'} class:now={step === 'world'}><span>1</span>World</li>
        <li class:now={step === 'club'}><span>2</span>Club and manager</li>
      </ol>
    {/if}

    {#if step === 'start'}
      <div class="card grid">
        <button class="primary big" onclick={() => (step = 'world')}>
          New game <Icon name="arrow" size={18} stroke={2.2} />
        </button>
        {#if slots.length > 0}
          <h3>Continue a saved game</h3>
          <div class="slots">
            {#each slots as slot (slot.name)}
              <button class="slot" onclick={() => load(slot)}>
                <Icon name="saves" size={18} />
                <span class="slot-text">
                  <strong>{slot.summary.club}</strong>
                  <span class="dim">{slot.summary.manager} · {longDate(slot.summary.date)} · {slot.name}</span>
                </span>
              </button>
            {/each}
          </div>
        {/if}
        <button class="link" onclick={() => (testing = !testing)}>Test this device's performance</button>
      </div>
      {#if testing}<Benchmark />{/if}
    {:else if step === 'world'}
      <div class="card grid">
        <h2>New world</h2>
        <div class="sizes" role="radiogroup" aria-label="World">
          {#each SIZES as o (o.value)}
            <label class="size" class:picked={size === o.value}>
              <input type="radio" name="size" value={o.value} bind:group={size} />
              <strong>{o.title}</strong>
              <span class="dim small">{o.detail}</span>
            </label>
          {/each}
        </div>
        <label class="field">
          <span>Seed</span>
          <input type="number" min="0" bind:value={seed} />
        </label>
        <div class="row">
          <button onclick={() => (step = 'start')}>Back</button>
          <span class="spacer"></span>
          <button class="primary" disabled={game.busy !== null} onclick={generate}>
            {game.busy ?? 'Generate'}
          </button>
        </div>
      </div>
    {:else}
      <div class="card grid">
        <div class="row">
          <h2>Choose your club</h2>
          <span class="spacer"></span>
          {#if leagues.length > 1}
            <select bind:value={leagueFilter} aria-label="League">
              <option value="all">All leagues</option>
              {#each leagues as l (l.id)}
                <option value={l.id}>{l.nation} – {l.name}</option>
              {/each}
            </select>
          {/if}
        </div>
        <div class="clubs">
          {#each shown.slice(0, 400) as c (c.club.id)}
            <button
              class="club"
              class:picked={picked?.club.id === c.club.id}
              style:--club={c.club.colors.primary}
              onclick={() => (picked = c)}
            >
              <Crest club={c.club} size={40} />
              <span class="cname">
                <strong>{c.club.name}</strong>
                <span class="dim">{c.league.name}</span>
              </span>
              <span class="stars" title="{c.stars} of 5">
                {#each [1, 2, 3, 4, 5] as n (n)}<i class:on={n <= c.stars}></i>{/each}
              </span>
            </button>
          {/each}
        </div>
        <div class="names">
          <label class="field">
            <span>First name</span>
            <input type="text" placeholder="Alex" bind:value={firstName} aria-label="First name" />
          </label>
          <label class="field">
            <span>Last name</span>
            <input type="text" placeholder="Manager" bind:value={lastName} aria-label="Last name" />
          </label>
        </div>
        <div class="row">
          <button onclick={() => (step = 'world')}>Back</button>
          <span class="spacer"></span>
          <button class="primary" disabled={!picked || game.busy !== null} onclick={start}>
            {picked ? `Take charge of ${picked.club.shortName}` : 'Pick a club'}
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .title {
    position: relative;
    min-height: 100%;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 600px);
    align-items: center;
    gap: 48px;
    max-width: 1240px;
    margin: 0 auto;
    padding: max(48px, env(safe-area-inset-top)) 32px 48px;
  }

  .stadium {
    position: fixed;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .lights {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(600px 380px at 12% -6%, rgba(200, 225, 255, 0.16), transparent 70%),
      radial-gradient(600px 380px at 88% -6%, rgba(200, 225, 255, 0.12), transparent 70%),
      radial-gradient(900px 500px at 50% 115%, rgba(40, 140, 80, 0.22), transparent 70%);
  }

  /* The pitch, laid back in perspective and fading into the dark. */
  .pitch {
    position: absolute;
    left: 50%;
    bottom: -8%;
    width: 130%;
    height: 80%;
    color: rgba(210, 255, 190, 0.2);
    transform: translateX(-50%) perspective(1100px) rotateX(58deg);
    transform-origin: 50% 100%;
    mask-image: linear-gradient(to top, #000 30%, transparent 95%);
    background: repeating-linear-gradient(
      90deg,
      rgba(60, 160, 90, 0.07) 0 70px,
      rgba(60, 160, 90, 0.03) 70px 140px
    );
  }

  .intro,
  .panel {
    position: relative;
    z-index: 1;
  }

  .mark {
    display: inline-grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: 16px;
    background: var(--accent);
    color: var(--accent-ink);
    box-shadow: 0 10px 40px -10px var(--accent-glow);
    margin-bottom: 22px;
  }

  .logo {
    margin: 0;
    font-size: clamp(2.6rem, 6vw, 4.8rem);
    font-weight: 900;
    font-stretch: 125%;
    letter-spacing: -0.04em;
    line-height: 0.95;
  }

  .logo b {
    color: var(--accent);
    font-weight: 900;
  }

  .tagline {
    margin: 18px 0 26px;
    max-width: 30ch;
    font-size: 1.15rem;
    color: var(--text-dim);
  }

  .features {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 12px;
    max-width: 44ch;
  }

  .features li {
    display: flex;
    gap: 14px;
    align-items: baseline;
    color: var(--text-dim);
  }

  .features span {
    font-family: var(--display);
    font-stretch: 85%;
    font-weight: 800;
    font-size: 0.8rem;
    color: var(--accent);
    letter-spacing: 0.08em;
  }

  .panel {
    display: grid;
    gap: 16px;
  }

  .panel > .card {
    background: var(--glass);
    backdrop-filter: blur(18px) saturate(1.3);
    -webkit-backdrop-filter: blur(18px) saturate(1.3);
    padding: 22px;
  }

  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    gap: 18px;
    font-size: 0.85rem;
    color: var(--text-faint);
  }

  .steps li {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .steps li span {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 1px solid var(--line-strong);
    font-weight: 700;
    font-size: 0.75rem;
  }

  .steps li.now {
    color: var(--text);
  }

  .steps li.now span {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }

  .steps li.done span {
    border-color: var(--accent);
    color: var(--accent);
  }

  .big {
    min-height: 54px;
    font-size: 1.05rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
  }

  .slots {
    display: grid;
    gap: 8px;
  }

  .slot {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 16px;
    text-align: left;
    border-radius: var(--radius);
    background: var(--bg-sunken);
    border-color: var(--line);
    color: var(--text-dim);
  }

  .slot-text {
    display: grid;
    color: var(--text);
  }

  .sizes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .size {
    display: grid;
    align-content: start;
    gap: 4px;
    padding: 14px 16px;
    border-radius: var(--radius);
    background: var(--bg-sunken);
    border: 1px solid var(--line);
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }

  .size:hover {
    border-color: var(--line-strong);
  }

  .size input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .size.picked {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .size:has(input:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .clubs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 8px;
    max-height: 46vh;
    overflow-y: auto;
    padding: 2px;
    margin: 0 -2px;
  }

  .club {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
    padding: 10px 14px 10px 12px;
    min-height: 64px;
    border-radius: var(--radius);
    background:
      linear-gradient(100deg, color-mix(in srgb, var(--club) 18%, transparent), transparent 55%),
      var(--bg-sunken);
    border-color: var(--line);
    overflow: hidden;
  }

  .club.picked {
    border-color: var(--accent);
    box-shadow:
      0 0 0 1px var(--accent) inset,
      0 0 26px -10px var(--accent-glow);
  }

  .cname {
    display: grid;
    flex: 1;
    min-width: 0;
    line-height: 1.3;
  }

  .cname strong,
  .cname span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cname span {
    font-size: 0.82rem;
    font-weight: 400;
  }

  .stars {
    display: flex;
    gap: 3px;
    align-self: flex-start;
    margin-top: 4px;
  }

  .stars i {
    width: 6px;
    height: 14px;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.1);
  }

  .stars i.on {
    background: var(--accent);
  }

  .names {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .error {
    color: var(--bad);
  }

  @media (max-width: 1000px) {
    .title {
      grid-template-columns: minmax(0, 640px);
      justify-content: center;
      gap: 28px;
      padding: max(32px, env(safe-area-inset-top)) 16px 32px;
    }

    .features {
      display: none;
    }

    .tagline {
      margin-bottom: 0;
    }
  }

  @media (max-width: 520px) {
    .sizes,
    .names {
      grid-template-columns: 1fr;
    }
  }
</style>
