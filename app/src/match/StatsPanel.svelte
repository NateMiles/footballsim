<!-- Team statistics side by side. -->
<script lang="ts">
  import type { TeamStats } from '../api';

  let { stats }: { stats: [TeamStats, TeamStats] } = $props();
  const pct = (a: number, b: number) => (b === 0 ? 0 : Math.round((100 * a) / b));
  const rows = $derived<[string, string, string][]>([
    ['Possession', `${Math.round(stats[0].possession * 100)}%`, `${Math.round(stats[1].possession * 100)}%`],
    ['Shots', `${stats[0].shots}`, `${stats[1].shots}`],
    ['On target', `${stats[0].shotsOnTarget}`, `${stats[1].shotsOnTarget}`],
    ['Expected goals', stats[0].xg.toFixed(2), stats[1].xg.toFixed(2)],
    ['Passes', `${stats[0].passes}`, `${stats[1].passes}`],
    ['Pass accuracy', `${pct(stats[0].passesCompleted, stats[0].passes)}%`, `${pct(stats[1].passesCompleted, stats[1].passes)}%`],
    ['Tackles', `${stats[0].tackles}`, `${stats[1].tackles}`],
    ['Corners', `${stats[0].corners}`, `${stats[1].corners}`],
    ['Fouls', `${stats[0].fouls}`, `${stats[1].fouls}`],
    ['Offsides', `${stats[0].offsides}`, `${stats[1].offsides}`],
    ['Yellow cards', `${stats[0].yellows}`, `${stats[1].yellows}`],
    ['Red cards', `${stats[0].reds}`, `${stats[1].reds}`],
    ['Saves', `${stats[0].saves}`, `${stats[1].saves}`],
  ]);
</script>

<table class="stats">
  <tbody>
    {#each rows as [label, h, a] (label)}
      <tr>
        <td class="num">{h}</td>
        <td class="label dim">{label}</td>
        <td>{a}</td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  .stats {
    width: 100%;
    border-collapse: collapse;
    font-variant-numeric: tabular-nums;
  }

  td {
    padding: 6px 4px;
    border-bottom: 1px solid var(--line);
    width: 30%;
  }

  .label {
    text-align: center;
    width: 40%;
    font-size: 0.85rem;
  }
</style>
