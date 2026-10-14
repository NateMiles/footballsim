<!-- A story as a card: picture, label, headline and standfirst. -->
<script lang="ts">
  import type { Article } from '../api';
  import { shortDate } from '../format';
  import NewsArt from './NewsArt.svelte';

  let {
    article,
    compact = false,
    onclick,
  }: { article: Article; compact?: boolean; onclick?: () => void } = $props();

  const LABEL: Record<Article['kind'], string> = {
    result: 'Result',
    transfer: 'Transfer',
    rumour: 'Rumour',
    manager: 'Manager',
    trophy: 'Honours',
    injury: 'Injury',
    unrest: 'Unrest',
    youth: 'Youth',
    retirement: 'Retirement',
    quote: 'Press',
  };
</script>

<button class="story" class:compact class:mine={article.mine} {onclick}>
  <NewsArt art={article.art} />
  <span class="meta">
    <span class="kind {article.kind}">{LABEL[article.kind]}</span>
    <span class="date">{shortDate(article.date)}</span>
  </span>
  <strong class="headline">{article.headline}</strong>
  {#if !compact}<span class="standfirst">{article.standfirst}</span>{/if}
</button>

<style>
  .story {
    display: grid;
    align-content: start;
    gap: 8px;
    padding: 10px 10px 14px;
    min-height: 0;
    text-align: left;
    border-radius: var(--radius-lg);
    border: 1px solid var(--line);
    background: var(--bg-raised);
    box-shadow: var(--shadow);
    font-weight: 400;
    transition:
      transform 0.18s var(--ease),
      border-color 0.18s;
  }

  .story:hover:not(:disabled) {
    transform: translateY(-2px);
    border-color: var(--line-strong);
    background: var(--bg-raised);
  }

  .story.mine {
    border-color: color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 4px;
  }

  .kind {
    font-family: var(--display);
    font-stretch: 85%;
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--info);
  }

  .kind.transfer,
  .kind.quote {
    color: var(--accent);
  }

  .kind.rumour,
  .kind.unrest {
    color: var(--warn);
  }

  .kind.manager,
  .kind.injury {
    color: var(--bad);
  }

  .kind.trophy {
    color: #ffd76a;
  }

  .kind.youth {
    color: var(--good);
  }

  .date {
    font-size: 0.75rem;
    color: var(--text-faint);
  }

  .headline {
    padding: 0 4px;
    font-family: var(--display);
    font-size: 1.05rem;
    font-weight: 750;
    line-height: 1.2;
    color: var(--text);
  }

  .compact .headline {
    font-size: 0.95rem;
  }

  .standfirst {
    padding: 0 4px;
    font-size: 0.85rem;
    color: var(--text-dim);
    line-height: 1.45;
  }
</style>
