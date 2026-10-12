<!-- A club's crest: a shield in its colours, with a pattern chosen from
     the club's id and its initials across it. -->
<script lang="ts">
  import type { ClubRef } from '../api';

  let { club, size = 28 }: { club: ClubRef; size?: number } = $props();
  const uid = $props.id();

  const initials = $derived(
    club.shortName
      .split(/\s+/)
      .map((w) => w[0])
      .join('')
      .slice(0, 3)
      .toUpperCase(),
  );
  const fontSize = $derived(initials.length >= 3 ? 11 : initials.length === 2 ? 14 : 18);
  const pattern = $derived(club.id % 6);
  const SHIELD = 'M20 1.5 37.5 6.5V22c0 11.5-7.6 18.8-17.5 22.5C10.1 40.8 2.5 33.5 2.5 22V6.5z';
</script>

<svg
  class="crest"
  width={size}
  height={size}
  viewBox="0 0 40 46"
  role="img"
  aria-label={club.name}
>
  <title>{club.name}</title>
  <defs>
    <clipPath id="{uid}-clip"><path d={SHIELD} /></clipPath>
    <linearGradient id="{uid}-gloss" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#fff" stop-opacity="0.28" />
      <stop offset="0.5" stop-color="#fff" stop-opacity="0" />
      <stop offset="1" stop-color="#000" stop-opacity="0.25" />
    </linearGradient>
  </defs>
  <g clip-path="url(#{uid}-clip)">
    <rect width="40" height="46" fill={club.colors.primary} />
    <g fill={club.colors.secondary}>
      {#if pattern === 0}
        <rect x="-10" y="18" width="70" height="10" transform="rotate(-48 20 23)" />
      {:else if pattern === 1}
        <rect x="20" width="20" height="46" />
      {:else if pattern === 2}
        <rect x="7" width="6" height="46" />
        <rect x="17" width="6" height="46" />
        <rect x="27" width="6" height="46" />
      {:else if pattern === 3}
        <rect y="16" width="40" height="13" />
      {:else if pattern === 4}
        <path d="M0 12 20 25 40 12v11L20 36 0 23z" />
      {:else}
        <rect x="20" width="20" height="23" />
        <rect y="23" width="20" height="23" />
      {/if}
    </g>
    <rect width="40" height="46" fill="url(#{uid}-gloss)" />
  </g>
  <path d={SHIELD} fill="none" stroke="rgba(255,255,255,0.35)" stroke-width="1.4" />
  {#if size >= 26}<text
    x="20"
    y="23.5"
    font-size={fontSize}
    text-anchor="middle"
    dominant-baseline="central">{initials}</text
    >{/if}
</svg>

<style>
  .crest {
    display: inline-block;
    flex: none;
    vertical-align: middle;
    overflow: visible;
    filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.35));
  }

  text {
    font-family: var(--display);
    font-weight: 900;
    font-stretch: 90%;
    letter-spacing: -0.02em;
    fill: #fff;
    stroke: rgba(0, 0, 0, 0.55);
    stroke-width: 2.4px;
    paint-order: stroke;
    stroke-linejoin: round;
  }
</style>
