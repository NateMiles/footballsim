<!-- A club's crest: its colours and initials. -->
<script lang="ts">
  import type { ClubRef } from '../api';

  let { club, size = 28 }: { club: ClubRef; size?: number } = $props();
  const initials = $derived(
    club.shortName
      .split(/\s+/)
      .map((w) => w[0])
      .join('')
      .slice(0, 3)
      .toUpperCase(),
  );
</script>

<span
  class="crest"
  title={club.name}
  style:width="{size}px"
  style:height="{size}px"
  style:font-size="{size * 0.36}px"
  style:background="linear-gradient(135deg, {club.colors.primary} 0 55%, {club.colors.secondary} 55% 100%)"
  style:color={club.colors.secondary}
>
  <span>{initials}</span>
</span>

<style>
  .crest {
    display: inline-grid;
    place-items: center;
    flex: none;
    border-radius: 30% 30% 50% 50%;
    border: 1px solid rgba(255, 255, 255, 0.25);
    font-weight: 700;
    vertical-align: middle;
  }

  .crest span {
    mix-blend-mode: difference;
    color: white;
  }
</style>
