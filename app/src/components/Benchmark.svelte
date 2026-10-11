<!-- A performance test of this device: generate a world, play part of a
     season, and report the speed and memory. The test world is separate
     from any game. -->
<script lang="ts">
  import type { WorldSize } from '../api';
  import { useGame } from '../state/game.svelte';

  /** Days of the season to play: August to mid-October, matches included. */
  const DAYS = 90;
  /** Days per request, so progress shows. */
  const STEP = 15;

  type Result = {
    size: WorldSize;
    players: number;
    generateMs: number;
    msPerDay: number;
    seasonSeconds: number;
    matches: number;
    memoryMb: number | null;
    threads: number;
  };

  const game = useGame();
  let running = $state<string | null>(null);
  let results = $state<Result[]>([]);
  let error = $state<string | null>(null);

  async function run(size: WorldSize) {
    error = null;
    try {
      running = `Generating a ${size} world`;
      let t = performance.now();
      let status = await game.client.benchmarkStart(size);
      const generateMs = performance.now() - t;
      // Skip the quiet weeks of July: start timing from August.
      status = await game.client.benchmarkRun(30);
      let elapsed = 0;
      for (let done = 0; done < DAYS; done += STEP) {
        running = `Playing ${size} world: day ${done} of ${DAYS}`;
        t = performance.now();
        status = await game.client.benchmarkRun(STEP);
        elapsed += performance.now() - t;
      }
      const msPerDay = elapsed / DAYS;
      results = [
        ...results,
        {
          size,
          players: status.players,
          generateMs,
          msPerDay,
          seasonSeconds: (msPerDay * 365) / 1000,
          matches: status.matches,
          memoryMb: status.memoryBytes === null ? null : status.memoryBytes / 1e6,
          threads: status.threads,
        },
      ];
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      running = null;
      await game.client.benchmarkEnd().catch(() => undefined);
    }
  }
</script>

<section class="card grid bench">
  <h3>Performance test</h3>
  <p class="dim">
    Generates a test world and plays {DAYS} days of its season on this device, apart from any game. The season figure
    is a projection: the summer and the 1 July rollover are slower than the autumn.
  </p>
  <div class="row">
    <button disabled={running !== null} onclick={() => run('small')}>Test small world</button>
    <button disabled={running !== null} onclick={() => run('medium')}>Test medium world</button>
  </div>
  {#if running}<p role="status">{running}…</p>{/if}
  {#if error}<p class="error">{error}</p>{/if}
  {#if results.length > 0}
    <table class="data">
      <thead>
        <tr>
          <th>World</th>
          <th class="num">Players</th>
          <th class="num">Generate</th>
          <th class="num">Per day</th>
          <th class="num">Season</th>
          <th class="num">Memory</th>
          <th class="num">Threads</th>
        </tr>
      </thead>
      <tbody>
        {#each results as r, i (i)}
          <tr>
            <td>{r.size}</td>
            <td class="num">{r.players.toLocaleString()}</td>
            <td class="num">{(r.generateMs / 1000).toFixed(1)} s</td>
            <td class="num">{r.msPerDay.toFixed(0)} ms</td>
            <td class="num"><strong>{r.seasonSeconds.toFixed(0)} s</strong></td>
            <td class="num">{r.memoryMb === null ? '–' : `${r.memoryMb.toFixed(0)} MB`}</td>
            <td class="num">{r.threads}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    <p class="dim small">{navigator.userAgent}</p>
  {/if}
</section>

<style>
  .error {
    color: var(--bad);
  }

  .small {
    font-size: 0.75rem;
    word-break: break-word;
  }
</style>
