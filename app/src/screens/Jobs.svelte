<!-- Vacant jobs, for a manager out of work (or looking to move). -->
<script lang="ts">
  import Crest from '../components/Crest.svelte';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  const game = useGame();
  const view = load(() => game.client.jobs());

  async function apply(club: number) {
    const o = await game.run('Applying', () => game.client.applyForJob(club));
    if (o) {
      game.changed(o);
      game.router.go({ name: 'home' });
    }
  }
</script>

{#if view.data}
  {@const v = view.data}
  <div class="screen grid">
    <h1>Jobs</h1>
    <p class="dim">Your reputation: {v.reputation}/100. Boards hire managers whose reputation is close to their club's.</p>
    {#if v.employed}
      <p>You already have a club. Resign from the home screen to take another job.</p>
    {/if}
    <section class="card">
      {#each v.jobs as j (j.club.id)}
        <div class="job">
          <Crest club={j.club} size={30} />
          <div class="name">
            <strong>{j.club.name}</strong>
            <span class="dim">{j.league.name}</span>
          </div>
          <span class="stars" title="{j.stars} of 5">{'★'.repeat(j.stars)}{'☆'.repeat(5 - j.stars)}</span>
          {#if j.wouldHire}
            <button class="primary" disabled={v.employed} onclick={() => apply(j.club.id)}>Take the job</button>
          {:else}
            <span class="pill">Out of reach</span>
          {/if}
        </div>
      {:else}
        <p class="dim">No clubs are looking for a manager. Press Continue and check again.</p>
      {/each}
    </section>
  </div>
{/if}

<style>
  .job {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 0;
    border-bottom: 1px solid var(--line);
  }

  .name {
    display: grid;
    flex: 1;
  }

  .stars {
    color: var(--warn);
  }
</style>
