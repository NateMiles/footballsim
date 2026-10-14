<!-- The news: a lead story with its picture and full text, then the rest as
     cards. Pick a card to read it as the lead. -->
<script lang="ts">
  import type { Article, NewsScope } from '../api';
  import ArticleCard from '../components/ArticleCard.svelte';
  import NewsArt from '../components/NewsArt.svelte';
  import Portrait from '../components/Portrait.svelte';
  import { longDate } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  const game = useGame();
  let scope = $state<NewsScope>('nation');
  const news = load(() => game.client.news(scope));
  let chosen = $state<number | null>(null);
  const articles = $derived(news.data?.articles ?? []);
  const lead = $derived(articles.find((a) => a.id === chosen) ?? articles[0] ?? null);
  const rest = $derived(articles.filter((a) => a !== lead));
  const nation = $derived(game.overview?.league?.nation ?? 'National');

  const SCOPES: { value: NewsScope; label: string }[] = $derived([
    { value: 'club', label: 'Your club' },
    { value: 'nation', label: nation },
    { value: 'world', label: 'World' },
  ]);

  function read(a: Article) {
    chosen = a.id;
    document.querySelector('main')?.scrollTo({ top: 0, behavior: 'smooth' });
  }
</script>

<div class="screen grid news">
  <header class="masthead">
    <div>
      <span class="eyebrow">{game.overview ? longDate(game.overview.date) : ''}</span>
      <h1>The Football Daily</h1>
    </div>
    <div class="tabs" role="tablist" aria-label="Which news">
      {#each SCOPES as s (s.value)}
        <button
          role="tab"
          aria-selected={scope === s.value}
          onclick={() => {
            scope = s.value;
            chosen = null;
          }}>{s.label}</button
        >
      {/each}
    </div>
  </header>

  {#if news.data && !lead}
    <section class="card empty">
      <p>No news yet. Stories arrive as the season unfolds.</p>
    </section>
  {/if}

  {#if lead}
    <article class="card lead" data-testid="lead-story">
      <div class="lead-art"><NewsArt art={lead.art} lead /></div>
      <div class="lead-text">
        <span class="kind {lead.kind}">{lead.kind}</span>
        <h2>{lead.headline}</h2>
        <p class="standfirst">{lead.standfirst}</p>
        <div class="byline">
          {#if lead.byline.look}<Portrait look={lead.byline.look} size={34} ring />{/if}
          <span>
            <strong>{lead.byline.name}</strong>
            <span class="dim">{lead.byline.outlet} · {longDate(lead.date)}</span>
          </span>
        </div>
        {#each lead.body as para, i (i)}
          <p class="para">{para}</p>
        {/each}
        {#if lead.player !== null}
          <a class="more" href="#/player/{lead.player}">View player</a>
        {/if}
      </div>
    </article>
  {/if}

  {#if rest.length > 0}
    <section class="stories">
      {#each rest as a (a.id)}
        <ArticleCard article={a} onclick={() => read(a)} />
      {/each}
    </section>
  {/if}
</div>

<style>
  .masthead {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    padding-bottom: 14px;
    border-bottom: 3px double var(--line-strong);
  }

  .masthead h1 {
    margin: 2px 0 0;
    font-size: clamp(1.8rem, 4vw, 2.8rem);
    font-stretch: 125%;
    letter-spacing: -0.03em;
  }

  .lead {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
    gap: 24px;
    padding: 20px;
  }

  @container (max-width: 820px) {
    .lead {
      grid-template-columns: 1fr;
    }
  }

  .news {
    container-type: inline-size;
  }

  .lead-art :global(.art) {
    border-radius: var(--radius-lg);
  }

  .lead-text h2 {
    font-size: clamp(1.5rem, 3vw, 2.2rem);
    font-weight: 800;
    font-stretch: 110%;
    letter-spacing: -0.02em;
    line-height: 1.08;
    margin: 10px 0 10px;
  }

  .standfirst {
    font-size: 1.08rem;
    color: var(--text);
    opacity: 0.85;
  }

  .byline {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 16px 0;
    padding: 10px 0;
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
    font-size: 0.85rem;
  }

  .byline > span {
    display: grid;
  }

  .para {
    color: var(--text-dim);
    line-height: 1.6;
  }

  .more {
    display: inline-block;
    margin-top: 6px;
    font-weight: 600;
  }

  .stories {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: var(--gap);
  }

  .empty {
    text-align: center;
    color: var(--text-dim);
    padding: 40px 18px;
  }

  .kind {
    display: inline-block;
    padding: 2px 10px;
    border-radius: 999px;
    font-family: var(--display);
    font-stretch: 85%;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    background: var(--accent-soft);
    color: var(--accent);
  }
</style>
