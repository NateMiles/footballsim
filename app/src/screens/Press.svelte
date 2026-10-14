<!-- The press room: conferences before and after matches, one question at
     a time, with what each answer did. -->
<script lang="ts">
  import type { ConferenceView, PressView, Tone } from '../api';
  import ArticleCard from '../components/ArticleCard.svelte';
  import Crest from '../components/Crest.svelte';
  import Portrait from '../components/Portrait.svelte';
  import { longDate } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  const game = useGame();
  const loaded = load(() => game.client.press());
  let fresh = $state<PressView | null>(null);
  const press = $derived(fresh ?? loaded.data);
  const me = $derived(game.overview?.manager);
  let sending = $state(false);

  async function answer(c: ConferenceView, question: number, tone: Tone) {
    sending = true;
    try {
      fresh = await game.client.answerQuestion(c.id, question, tone);
      await game.refresh();
    } catch (e) {
      game.fail(e);
    } finally {
      sending = false;
    }
  }

  async function leave(c: ConferenceView) {
    fresh = (await game.run('Leaving', () => game.client.leaveConference(c.id))) ?? fresh;
    await game.refresh();
  }

  async function delegate(on: boolean) {
    fresh = (await game.run('Saving', () => game.client.setPressDelegation(on))) ?? fresh;
    await game.refresh();
  }

  // Rapport from -100..100 as a 0..100 position on the meter.
  const rapportAt = $derived(press ? (press.rapport + 100) / 2 : 50);
</script>

<div class="screen grid">
  <div class="row head">
    <h1>Press room</h1>
    <span class="spacer"></span>
    {#if press}
      <div class="rapport" title="How the press takes to you">
        <span class="eyebrow">Press relations</span>
        <span class="meter"><span class="dot" style:left="{rapportAt}%"></span></span>
        <strong>{press.rapportLabel}</strong>
      </div>
    {/if}
  </div>

  {#if press}
    {#each press.conferences as c (c.id)}
      {@const current = c.questions.findIndex((q) => !q.answer)}
      <section class="card conference" style:--c1={c.club.colors.primary} data-testid="conference">
        <header class="stage">
          <div class="wall">
            {#each Array(10) as _, i (i)}<Crest club={c.club} size={30} />{/each}
          </div>
          <div class="stage-text">
            <span class="eyebrow">{c.preMatch ? 'Pre-match' : 'Post-match'} press conference</span>
            <h2>{c.title}</h2>
            <span class="dim small">Open until {longDate(c.closes)}</span>
          </div>
          <div class="versus">
            <Crest club={c.club} size={48} />
            <span>v</span>
            <Crest club={c.opponent} size={48} />
          </div>
        </header>

        <ol class="transcript">
          {#each c.questions as q (q.index)}
            {#if q.answer || q.index === current}
              <li class="ask">
                <Portrait look={q.journalist.look} size={44} ring />
                <div class="bubble them">
                  <span class="who">{q.journalist.name} <span class="outlet {q.journalist.kind}">{q.journalist.outlet}</span></span>
                  <p>{q.text}</p>
                </div>
              </li>
            {/if}
            {#if q.answer && me}
              <li class="reply">
                <div class="bubble you">
                  <span class="who">You · {q.answer.label}</span>
                  <p>“{q.answer.text}”</p>
                  {#if q.effects.length > 0}
                    <span class="effects">
                      {#each q.effects as e, i (i)}
                        <span class="pill" class:good={e.good === true} class:bad={e.good === false}>{e.text}</span>
                      {/each}
                    </span>
                  {/if}
                </div>
                <Portrait look={me.look} size={44} ring shirt="#1b2433" trim={c.club.colors.primary} expression="talking" />
              </li>
            {/if}
          {/each}
        </ol>

        {#if current >= 0}
          {@const q = c.questions[current]}
          <div class="answers" role="group" aria-label="Your answer">
            {#each q.options as o (o.tone)}
              <button class="answer" disabled={sending} onclick={() => answer(c, q.index, o.tone)}>
                <span class="tone">{o.label}</span>
                <span class="line">“{o.text}”</span>
              </button>
            {/each}
          </div>
          <div class="row">
            <span class="dim small">Question {current + 1} of {c.questions.length}</span>
            <span class="spacer"></span>
            <button class="link" onclick={() => leave(c)}>Leave the rest to your assistant</button>
          </div>
        {:else}
          <div class="row done">
            <span>That's all, thank you.</span>
            <span class="spacer"></span>
            <button onclick={() => leave(c)}>Leave the room</button>
          </div>
        {/if}
      </section>
    {:else}
      <section class="card empty">
        <h3>No press conference</h3>
        <p class="dim">
          {press.delegated
            ? 'Your assistant is facing the press for you.'
            : 'The press will want to hear from you before and after each match.'}
        </p>
      </section>
    {/each}

    <section class="card">
      <label class="toggle">
        <input type="checkbox" role="switch" checked={press.delegated} onchange={() => delegate(!press.delegated)} />
        <span>Send your assistant to press conferences</span>
      </label>
    </section>

    {#if press.quotes.length > 0}
      <section class="grid">
        <h3>Your words in the papers</h3>
        <div class="quotes">
          {#each press.quotes as a (a.id)}
            <ArticleCard article={a} compact onclick={() => game.router.go({ name: 'news' })} />
          {/each}
        </div>
      </section>
    {/if}
  {/if}
</div>

<style>
  .head h1 {
    margin: 0;
  }

  .rapport {
    display: grid;
    grid-template-columns: auto 140px auto;
    align-items: center;
    gap: 10px;
  }

  .meter {
    position: relative;
    height: 8px;
    border-radius: 999px;
    background: linear-gradient(90deg, var(--bad), var(--warn) 45%, var(--good));
    opacity: 0.85;
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
  }

  .conference {
    display: grid;
    gap: 18px;
    padding: 0 0 18px;
    overflow: hidden;
  }

  .conference > :not(.stage) {
    margin: 0 18px;
  }

  .stage {
    position: relative;
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 22px 22px;
    overflow: hidden;
    background:
      linear-gradient(90deg, rgba(5, 8, 13, 0.85), rgba(5, 8, 13, 0.45)),
      color-mix(in srgb, var(--c1) 45%, #0d1420);
    border-bottom: 1px solid var(--line);
  }

  .wall {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    place-items: center;
    opacity: 0.12;
    transform: rotate(-6deg) scale(1.2);
    pointer-events: none;
  }

  .stage-text {
    position: relative;
    flex: 1;
  }

  .stage-text h2 {
    margin: 4px 0;
    font-size: 1.5rem;
    font-stretch: 110%;
  }

  .versus {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    font-family: var(--display);
    font-weight: 800;
    color: var(--text-dim);
  }

  .transcript {
    list-style: none;
    padding: 0;
    display: grid;
    gap: 12px;
  }

  .ask,
  .reply {
    display: flex;
    align-items: flex-end;
    gap: 10px;
    animation: rise 0.3s var(--ease) both;
  }

  .reply {
    justify-content: flex-end;
  }

  .bubble {
    max-width: min(560px, 80%);
    padding: 10px 14px 12px;
    border-radius: 18px;
    background: var(--bg-elevated);
    border: 1px solid var(--line);
  }

  .bubble p {
    margin: 2px 0 0;
  }

  .them {
    border-bottom-left-radius: 6px;
  }

  .you {
    border-bottom-right-radius: 6px;
    background: color-mix(in srgb, var(--accent) 12%, var(--bg-elevated));
    border-color: color-mix(in srgb, var(--accent) 30%, transparent);
  }

  .who {
    font-size: 0.78rem;
    font-weight: 650;
    color: var(--text-dim);
  }

  .outlet {
    font-weight: 500;
    margin-left: 4px;
    color: var(--text-faint);
  }

  .outlet.tabloid {
    color: var(--bad);
  }

  .outlet.broadsheet {
    color: var(--info);
  }

  .outlet.fanSite {
    color: var(--good);
  }

  .effects {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 8px;
  }

  .answers {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 10px;
  }

  .answer {
    display: grid;
    align-content: start;
    gap: 4px;
    padding: 12px 14px;
    min-height: 0;
    text-align: left;
    border-radius: var(--radius);
    background: var(--bg-sunken);
    border-color: var(--line);
    font-weight: 400;
  }

  .answer:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .tone {
    font-family: var(--display);
    font-stretch: 85%;
    font-weight: 700;
    font-size: 0.75rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--accent);
  }

  .line {
    color: var(--text);
    line-height: 1.4;
  }

  .done {
    color: var(--text-dim);
  }

  .empty {
    text-align: center;
    padding: 34px 18px;
  }

  .empty h3 {
    justify-content: center;
  }

  .quotes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--gap);
  }

  .toggle {
    display: flex;
    gap: 14px;
    align-items: center;
    min-height: var(--tap);
    cursor: pointer;
  }


  @media (max-width: 700px) {
    .rapport {
      grid-template-columns: auto 1fr auto;
      width: 100%;
    }

    .versus {
      display: none;
    }
  }
</style>
