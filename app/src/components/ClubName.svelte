<!-- A crest and name; links to the club's squad. -->
<script lang="ts">
  import type { ClubRef } from '../api';
  import { href } from '../state/router.svelte';
  import Crest from './Crest.svelte';

  let { club, short = false, link = true }: { club: ClubRef; short?: boolean; link?: boolean } = $props();
</script>

{#if link}
  <a class="club" href={href({ name: 'squad', club: club.id })}>
    <Crest {club} size={22} />
    <span>{short ? club.shortName : club.name}</span>
  </a>
{:else}
  <span class="club">
    <Crest {club} size={22} />
    <span>{short ? club.shortName : club.name}</span>
  </span>
{/if}

<style>
  .club {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: inherit;
  }
</style>
