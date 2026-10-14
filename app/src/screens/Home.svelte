<!-- The manager's desk: the club in its colours, the next match, news,
     the board and the money. -->
<script lang="ts">
  import ArticleCard from '../components/ArticleCard.svelte';
  import Crest from '../components/Crest.svelte';
  import Form from '../components/Form.svelte';
  import Gauge from '../components/Gauge.svelte';
  import Icon from '../components/Icon.svelte';
  import MatchTicket from '../components/MatchTicket.svelte';
  import Portrait from '../components/Portrait.svelte';
  import { money, ordinal } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';
  import { href } from '../state/router.svelte';

  const game = useGame();
  const o = $derived(game.overview);
  const inbox = load(() => game.client.inbox());

  async function toggleWatch() {
    if (!o) return;
    const next = await game.run('Saving', () => game.client.setWatchMatches(!o.watchMatches));
    if (next) game.changed(next);
  }

  let resigning = $state(false);

  async function resign() {
    const next = await game.run('Resigning', () => game.client.resign());
    resigning = false;
    if (next) game.changed(next);
  }

  async function toggleDelegate() {
    if (!o) return;
    const next = await game.run('Saving', () => game.client.setDelegation(!o.delegateSquad));
    if (next) game.changed(next);
  }

  const STATUS: Record<string, string> = { met: 'Met', failed: 'Missed', onTrack: 'On track', behind: 'Behind' };
  const TONE: Record<string, string> = { met: 'good', failed: 'bad', onTrack: 'good', behind: 'warn' };
</script>

{#if o && !o.club}
  <div class="screen home">
    <section class="hero free">
      <div class="hero-text">
        <span class="eyebrow">{o.season}</span>
        <h1>Out of work</h1>
        <p class="sub">{o.manager.name}</p>
      </div>
    </section>
    <section class="card">
      <h3>Next job</h3>
      <p>Clubs that sack their managers advertise the job. Apply for one, or press Continue to wait for more to come up.</p>
      <div class="row">
        <button class="primary" onclick={() => game.router.go({ name: 'jobs' })}>Look for a job</button>
      </div>
    </section>
  </div>
{:else if o && o.club}
  {@const club = o.club}
  <div class="screen home">
    <section
      class="hero"
      style:--club={club.colors.primary}
      style:--club-2={club.colors.secondary}
    >
      <Crest {club} size={76} />
      <div class="hero-text">
        <span class="eyebrow">{o.league?.name ?? 'No league'} · {o.season}</span>
        <h1>{club.name}</h1>
        <p class="sub manager">
          <Portrait look={o.manager.look} size={26} ring shirt="#1f2a3a" trim={club.colors.primary} />
          Managed by {o.manager.name}
        </p>
      </div>
      {#if o.position}
        <div class="standing">
          <span class="label">League position</span>
          <span class="pos">{o.position}<small>{ordinal(o.position).replace(/\d+/, '')}</small></span>
          {#if o.form.length > 0}<Form form={o.form} />{/if}
        </div>
      {/if}
    </section>

    <div class="board-grid">
      {#if o.matchDay}
        <section class="card wide">
          <h3>Match day</h3>
          <MatchTicket fixture={o.matchDay.fixture} mine={club.id} today={o.date} live>
            <button class="primary" onclick={() => game.router.go({ name: 'match' })}>Go to the match</button>
          </MatchTicket>
        </section>
      {/if}

      {#if o.pressWaiting > 0}
        <section class="card wide press-call">
          <span class="mic"><Icon name="mic" size={22} /></span>
          <div>
            <strong>The press are waiting</strong>
            <span class="dim small">
              {o.pressWaiting === 1 ? 'A press conference wants' : `${o.pressWaiting} press conferences want`} to hear from you.
            </span>
          </div>
          <span class="spacer"></span>
          <button class="primary" onclick={() => game.router.go({ name: 'press' })}>Face the press</button>
        </section>
      {/if}

      <section class="card next">
        <h3>Next match</h3>
        {#if o.nextFixture}
          <MatchTicket fixture={o.nextFixture} mine={club.id} today={o.date}>
            <button onclick={() => game.router.go({ name: 'prepare' })}>Prepare</button>
          </MatchTicket>
        {:else}
          <p class="dim">No fixtures scheduled.</p>
        {/if}
        <h3 class="gap">Last match</h3>
        {#if o.lastFixture}
          <MatchTicket fixture={o.lastFixture} mine={club.id} />
        {:else}
          <p class="dim">The season has not started.</p>
        {/if}
      </section>

      <div class="side">
        <section class="card">
          <div class="head">
            <h3>Inbox</h3>
            <a class="more" href="#/inbox">All <Icon name="arrow" size={14} /></a>
          </div>
          {#if inbox.data}
            <ul class="msgs">
              {#each inbox.data.messages.slice(0, 5) as m (m.id)}
                <li>
                  <a class="msg" class:unread={!m.read} href="#/inbox">
                    <span class="dot"></span>
                    <span class="title">{m.title}</span>
                  </a>
                </li>
              {:else}
                <li class="dim">No messages.</li>
              {/each}
            </ul>
          {/if}
        </section>

        {#if o.cups.length > 0}
          <section class="card">
            <h3>Cups</h3>
            <ul class="cups">
              {#each o.cups as c (c.cup.id)}
                <li>
                  <a href={href({ name: 'cup', cup: c.cup.id })}>
                    <span class="trophy" class:out={!c.alive}><Icon name="cups" size={18} /></span>
                    <span class="title">{c.cup.name}</span>
                    <span class="pill" class:good={c.alive}>{c.status}</span>
                  </a>
                </li>
              {/each}
            </ul>
          </section>
        {/if}
      </div>

      {#if o.headlines.length > 0}
        <section class="wide headlines" data-testid="headlines">
          <div class="head">
            <h3>Headlines</h3>
            <a class="more" href="#/news">All news <Icon name="arrow" size={14} /></a>
          </div>
          <div class="stories">
            {#each o.headlines as a (a.id)}
              <ArticleCard article={a} compact onclick={() => game.router.go({ name: 'news' })} />
            {/each}
          </div>
        </section>
      {/if}

      {#if o.board}
        <section class="card board">
          <h3>The board</h3>
          <div class="confidence">
            <Gauge value={o.board.confidence} label="Board confidence" good={55} ok={35} />
            <div>
              <span class="label">Main objective</span>
              <p class="objective">{o.board.objective}</p>
              <p class="dim small">The club {o.board.ethos.toLowerCase()}.</p>
            </div>
          </div>
          {#if o.board.objectives.length > 0}
            <ul class="objectives" data-testid="objectives">
              {#each o.board.objectives as ob, i (i)}
                <li class={TONE[ob.status]}>
                  <span class="text">
                    {ob.text}
                    {#if ob.important}<span class="pill warn">Important</span>{/if}
                  </span>
                  <span class="pill {TONE[ob.status]}">{STATUS[ob.status]}</span>
                  <span class="detail">{ob.detail}</span>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      {/if}

      <section class="card money">
        <h3>Money</h3>
        <div class="figures">
          <div>
            <span class="label">Balance</span>
            <span class="figure" class:neg={(o.balance ?? 0) < 0}>{money(o.balance ?? 0)}</span>
          </div>
          <div>
            <span class="label">Transfer budget</span>
            <span class="figure">{money(o.transferBudget ?? 0)}</span>
          </div>
        </div>
        <p class="window">
          <span class="pill" class:good={o.windowOpen}>Transfer window {o.windowOpen ? 'open' : 'closed'}</span>
        </p>
        <div class="row">
          <button onclick={() => game.router.go({ name: 'transfers' })}>Transfers</button>
          <button onclick={() => game.router.go({ name: 'finances' })}>Finances</button>
        </div>
      </section>

      <section class="card wide settings">
        <h3>Settings</h3>
        <label class="toggle">
          <input
            type="checkbox"
            role="switch"
            checked={o.watchMatches}
            onchange={toggleWatch}
            disabled={o.matchDay !== null}
          />
          <span>Stop on match days to watch and manage matches live</span>
        </label>
        <label class="toggle">
          <input type="checkbox" role="switch" checked={o.delegateSquad} onchange={toggleDelegate} />
          <span>Let the assistant handle contracts and signings</span>
        </label>
        <div class="row">
          {#if resigning}
            <span>Leave {club.name}? You will have to find a new job.</span>
            <button class="danger" onclick={resign}>Resign</button>
            <button onclick={() => (resigning = false)}>Stay</button>
          {:else}
            <button class="danger" onclick={() => (resigning = true)}>Resign…</button>
          {/if}
        </div>
      </section>
    </div>
  </div>
{/if}

<style>
  .home {
    display: grid;
    gap: var(--gap);
    container-type: inline-size;
  }

  /* The club in its colours: a wash of the primary from the left, a
     sash of the secondary, and the crest as a watermark. */
  .hero {
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    gap: 22px;
    flex-wrap: wrap;
    padding: 30px 30px 28px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--line);
    box-shadow: var(--shadow);
    background:
      linear-gradient(180deg, transparent 40%, rgba(5, 8, 13, 0.45)),
      linear-gradient(
        112deg,
        color-mix(in srgb, var(--club) 42%, #070b11) 0%,
        color-mix(in srgb, var(--club) 16%, #0b111a) 46%,
        var(--bg-raised) 78%
      );
  }

  .hero::before {
    content: '';
    position: absolute;
    inset: -40% auto -40% 58%;
    width: 46px;
    transform: rotate(18deg);
    background: var(--club-2);
    opacity: 0.12;
    box-shadow: 70px 0 0 color-mix(in srgb, var(--club) 60%, transparent);
  }

  .hero.free {
    background:
      radial-gradient(80% 140% at 0% 0%, rgba(120, 170, 255, 0.14), transparent 60%),
      var(--bg-raised);
  }

  .hero > :global(.crest) {
    position: relative;
    box-shadow: 0 12px 30px -10px rgba(0, 0, 0, 0.7);
    border-width: 2px;
  }

  .hero-text {
    position: relative;
    flex: 1;
    min-width: 220px;
  }

  .eyebrow,
  .label {
    display: block;
    font-family: var(--display);
    font-stretch: 85%;
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.14em;
    color: var(--text-dim);
  }

  .hero .eyebrow {
    color: rgba(238, 242, 247, 0.72);
  }

  .hero h1 {
    margin: 4px 0 4px;
    font-size: clamp(1.8rem, 4.2cqi, 2.9rem);
    font-stretch: 118%;
    letter-spacing: -0.02em;
    line-height: 1;
    text-shadow: 0 2px 20px rgba(0, 0, 0, 0.35);
  }

  .sub.manager {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
  }

  .sub {
    margin: 0;
    color: rgba(238, 242, 247, 0.75);
  }

  .standing {
    position: relative;
    display: grid;
    justify-items: end;
    gap: 6px;
  }

  .pos {
    font-family: var(--display);
    font-weight: 900;
    font-stretch: 120%;
    font-size: 4.2rem;
    line-height: 0.9;
    letter-spacing: -0.04em;
    font-variant-numeric: tabular-nums;
  }

  .pos small {
    font-size: 1.4rem;
    letter-spacing: 0;
    margin-left: 2px;
    color: var(--text-dim);
    vertical-align: top;
  }

  /* Twelve columns once there is room; one column below. */
  .board-grid {
    display: grid;
    gap: var(--gap);
    grid-template-columns: 1fr;
  }

  @container (min-width: 760px) {
    .board-grid {
      grid-template-columns: repeat(12, 1fr);
    }

    .wide {
      grid-column: 1 / -1;
    }

    .next,
    .board {
      grid-column: span 7;
    }

    .side,
    .money {
      grid-column: span 5;
    }
  }

  .press-call {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
    background:
      linear-gradient(100deg, var(--accent-soft), transparent 60%),
      var(--bg-raised);
  }

  .press-call > div {
    display: grid;
  }

  .mic {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 14px;
    background: var(--accent);
    color: var(--accent-ink);
    box-shadow: 0 0 24px -6px var(--accent-glow);
  }

  .headlines .stories {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: var(--gap);
  }

  .side {
    display: grid;
    gap: var(--gap);
    align-content: start;
  }

  .gap {
    margin-top: 20px;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .more {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 0.85rem;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .msg,
  .cups a {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 8px;
    margin: 0 -8px;
    border-radius: 10px;
    color: var(--text-dim);
    transition: background 0.15s;
  }

  .msg:hover,
  .cups a:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex: none;
    background: var(--line-strong);
  }

  .msg.unread {
    color: var(--text);
    font-weight: 600;
  }

  .msg.unread .dot {
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent-glow);
  }

  .cups a {
    color: var(--text);
  }

  .trophy {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 10px;
    background: var(--warn-soft);
    color: var(--warn);
  }

  .trophy.out {
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-faint);
  }

  .confidence {
    display: flex;
    align-items: center;
    gap: 20px;
    margin-bottom: 8px;
  }

  .objective {
    margin: 2px 0 4px;
    font-family: var(--display);
    font-size: 1.25rem;
    font-weight: 750;
    font-stretch: 105%;
  }

  .objectives {
    display: grid;
    gap: 8px;
    margin-top: 12px;
  }

  .objectives li {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 4px 12px;
    align-items: center;
    padding: 12px 14px;
    border-radius: var(--radius);
    background: var(--bg-sunken);
    border: 1px solid var(--line);
    border-left: 3px solid var(--line-strong);
  }

  .objectives li.good {
    border-left-color: var(--good);
  }

  .objectives li.warn {
    border-left-color: var(--warn);
  }

  .objectives li.bad {
    border-left-color: var(--bad);
  }

  .objectives .text {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .detail {
    grid-column: 1 / -1;
    font-size: 0.85rem;
    color: var(--text-dim);
  }

  .figures {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    margin-bottom: 14px;
  }

  .figures > div {
    display: grid;
    gap: 4px;
    padding: 14px;
    border-radius: var(--radius);
    background: var(--bg-sunken);
    border: 1px solid var(--line);
  }

  .figure {
    font-family: var(--display);
    font-weight: 800;
    font-stretch: 110%;
    font-size: 1.7rem;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.01em;
  }

  .figure.neg {
    color: var(--bad);
  }

  .window {
    margin-bottom: 14px;
  }

  .settings {
    display: grid;
    gap: 6px;
  }

  .toggle {
    display: flex;
    gap: 14px;
    align-items: center;
    min-height: var(--tap);
    cursor: pointer;
  }


  @media (max-width: 700px) {
    .hero {
      padding: 22px 18px;
      gap: 16px;
    }

    .standing {
      justify-items: start;
    }

    .pos {
      font-size: 3rem;
    }
  }
</style>
