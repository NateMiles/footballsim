<!-- A private word with a player, in the manager's office: their face
     reacts as they answer, in their own words. -->
<script lang="ts">
  import type { PlayerView, Reaction, Talk, TalkInfo } from '../api';
  import Portrait, { type Expression } from '../components/Portrait.svelte';
  import { longDate } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  let { id }: { id: number } = $props();
  const game = useGame();
  const loaded = load(() => game.client.player(id));
  let fresh = $state<PlayerView | null>(null);
  const p = $derived(fresh ?? loaded.data);

  type Line = { from: 'you' | 'them'; text: string; reaction?: Reaction };
  let lines = $state<Line[]>([]);
  let face = $state<Expression | null>(null);
  let talking = $state(false);

  const GROUPS: { key: string; title: string }[] = [
    { key: 'listen', title: 'Listen' },
    { key: 'encourage', title: 'Encourage' },
    { key: 'challenge', title: 'Challenge' },
    { key: 'promise', title: 'Promise' },
  ];
  const REACTION: Record<Reaction, { label: string; tone: string }> = {
    delighted: { label: 'Delighted', tone: 'good' },
    pleased: { label: 'Pleased', tone: 'good' },
    determined: { label: 'Determined', tone: 'good' },
    indifferent: { label: 'Indifferent', tone: '' },
    sceptical: { label: 'Sceptical', tone: 'warn' },
    annoyed: { label: 'Annoyed', tone: 'bad' },
    angry: { label: 'Angry', tone: 'bad' },
  };

  /** Their face before anything is said: how they feel. */
  const resting = $derived.by((): Expression => {
    const h = p?.people?.mood.happiness ?? 0;
    return h >= 30 ? 'happy' : h <= -30 ? 'angry' : h <= -12 ? 'annoyed' : 'neutral';
  });
  const shown = $derived(talking ? 'talking' : (face ?? resting));
  const happiness = $derived(Math.max(-100, Math.min(100, p?.people?.mood.happiness ?? 0)));

  async function say(t: TalkInfo) {
    if (!p) return;
    lines.push({ from: 'you', text: t.line });
    talking = true;
    try {
      const r = await game.client.talkToPlayer(p.id, t.talk as Talk);
      await new Promise((ok) => setTimeout(ok, 450));
      lines.push({ from: 'them', text: r.reply, reaction: r.reaction });
      face = r.expression as Expression;
      fresh = r.player;
      game.changed();
    } catch (e) {
      lines.pop();
      game.fail(e);
    } finally {
      talking = false;
    }
    requestAnimationFrame(() => {
      document.querySelector('.chat-end')?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    });
  }
</script>

{#if p}
  <div class="screen office" style:--club={p.club?.colors.primary ?? '#2a3a52'}>
    <aside class="card who">
      <div class="face" class:pulse={talking}>
        <Portrait
          look={p.look}
          size={220}
          ring
          expression={shown}
          shirt={p.club?.colors.primary}
          trim={p.club?.colors.secondary}
          title={p.name}
        />
      </div>
      <h1>{p.name}</h1>
      <p class="dim">{p.position} · {p.age} · {p.nation}</p>
      {#if p.people}
        <div class="mood">
          <span class="eyebrow">Happiness</span>
          <span class="bar"><span class="dot" style:left="{(happiness + 100) / 2}%"></span></span>
          <strong data-testid="mood">{p.people.mood.label}</strong>
        </div>
      {/if}
      {#if p.personality}
        <p class="character"><strong>{p.personality.name}.</strong> <span class="dim">{p.personality.description}</span></p>
      {/if}
      <a class="back" href="#/player/{p.id}">Player profile</a>
    </aside>

    <section class="card chat">
      <header>
        <h2>A private word</h2>
        <span class="dim small">
          {#if p.people?.lastTalk}Last spoke {longDate(p.people.lastTalk)}{:else}Your office, door closed{/if}
        </span>
      </header>

      <ol class="lines" aria-live="polite">
        {#if lines.length === 0}
          <li class="hint dim">Choose what to say. Questions can be asked any time; anything else once a week.</li>
        {/if}
        {#each lines as l, i (i)}
          <li class={l.from}>
            <div class="bubble">
              <p>{l.text}</p>
              {#if l.reaction}
                <span class="pill {REACTION[l.reaction].tone}" data-testid="reaction">{REACTION[l.reaction].label}</span>
              {/if}
            </div>
          </li>
        {/each}
        {#if talking}
          <li class="them"><div class="bubble typing"><i></i><i></i><i></i></div></li>
        {/if}
        <li class="chat-end" aria-hidden="true"></li>
      </ol>

      {#if p.people}
        <div class="topics">
          {#each GROUPS as g (g.key)}
            {@const ts = p.people.talks.filter((t) => t.group === g.key)}
            {#if ts.length > 0}
              <div class="group">
                <span class="eyebrow">{g.title}</span>
                <div class="choices">
                  {#each ts as t (t.talk)}
                    <button
                      class="topic {g.key}"
                      title={t.description}
                      disabled={!t.available || talking}
                      onclick={() => say(t)}
                    >
                      <strong>{t.name}</strong>
                      <span>“{t.line}”</span>
                    </button>
                  {/each}
                </div>
              </div>
            {/if}
          {/each}
        </div>
        {#if !p.people.canTalk}
          <p class="dim small">You have had a proper talk this week. Questions are still fine.</p>
        {/if}
      {:else}
        <p class="dim">Only your own players can be called into your office.</p>
      {/if}
    </section>
  </div>
{/if}

<style>
  .office {
    display: grid;
    grid-template-columns: minmax(250px, 320px) minmax(0, 1fr);
    gap: var(--gap);
    align-items: start;
  }

  @media (max-width: 860px) {
    .office {
      grid-template-columns: 1fr;
    }
  }

  .who {
    display: grid;
    justify-items: center;
    text-align: center;
    gap: 6px;
    background:
      radial-gradient(90% 60% at 50% 0%, color-mix(in srgb, var(--club) 40%, transparent), transparent 70%),
      var(--bg-raised);
  }

  .who h1 {
    margin: 8px 0 0;
    font-size: 1.6rem;
  }

  .face {
    transition: transform 0.3s var(--ease);
  }

  .face.pulse {
    animation: nod 0.9s ease-in-out infinite;
  }

  @keyframes nod {
    50% {
      transform: translateY(-3px) rotate(-1deg);
    }
  }

  .mood {
    display: grid;
    gap: 6px;
    width: 100%;
    margin-top: 6px;
  }

  .bar {
    position: relative;
    height: 8px;
    border-radius: 999px;
    background: linear-gradient(90deg, var(--bad), var(--warn) 45%, var(--good));
  }

  .dot {
    position: absolute;
    top: 50%;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text);
    border: 3px solid var(--bg);
    transform: translate(-50%, -50%);
    transition: left 0.6s var(--ease);
  }

  .character {
    font-size: 0.9rem;
    margin-top: 6px;
  }

  .back {
    margin-top: 6px;
    font-size: 0.9rem;
  }

  .chat {
    display: grid;
    gap: 16px;
  }

  .chat header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 10px;
    flex-wrap: wrap;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--line);
  }

  .chat h2 {
    margin: 0;
  }

  .lines {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 10px;
    min-height: 120px;
    max-height: 46vh;
    overflow-y: auto;
  }

  .lines li {
    display: flex;
    animation: rise 0.25s var(--ease) both;
  }

  .lines li.you {
    justify-content: flex-end;
  }

  .hint {
    justify-content: center;
    padding: 20px 0;
    text-align: center;
  }

  .bubble {
    max-width: min(520px, 85%);
    padding: 10px 14px;
    border-radius: 18px;
    background: var(--bg-elevated);
    border: 1px solid var(--line);
  }

  .bubble p {
    margin: 0 0 4px;
  }

  .you .bubble {
    border-bottom-right-radius: 6px;
    background: color-mix(in srgb, var(--accent) 12%, var(--bg-elevated));
    border-color: color-mix(in srgb, var(--accent) 30%, transparent);
  }

  .them .bubble {
    border-bottom-left-radius: 6px;
  }

  .typing {
    display: flex;
    gap: 4px;
    padding: 14px 16px;
  }

  .typing i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-dim);
    animation: blink 1s infinite;
  }

  .typing i:nth-child(2) {
    animation-delay: 0.15s;
  }

  .typing i:nth-child(3) {
    animation-delay: 0.3s;
  }

  @keyframes blink {
    50% {
      opacity: 0.25;
    }
  }

  .topics {
    display: grid;
    gap: 12px;
  }

  .group {
    display: grid;
    gap: 6px;
  }

  .choices {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 8px;
  }

  .topic {
    display: grid;
    align-content: start;
    gap: 2px;
    padding: 10px 12px;
    min-height: 0;
    text-align: left;
    border-radius: var(--radius);
    background: var(--bg-sunken);
    border-color: var(--line);
    font-weight: 400;
  }

  .topic strong {
    font-size: 0.9rem;
  }

  .topic span {
    font-size: 0.82rem;
    color: var(--text-dim);
  }

  .topic.listen strong {
    color: var(--info);
  }

  .topic.encourage strong {
    color: var(--good);
  }

  .topic.challenge strong {
    color: var(--bad);
  }

  .topic.promise strong {
    color: var(--warn);
  }

  .topic:hover:not(:disabled) {
    border-color: var(--line-strong);
    background: var(--bg-hover);
  }
</style>
