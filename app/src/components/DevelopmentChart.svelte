<!-- Ability by season, as a small line chart (0-100 scale). -->
<script lang="ts">
  let { points }: { points: { label: string; value: number }[] } = $props();
  const W = 320;
  const H = 120;
  const PAD = 22;
  const lo = $derived(Math.max(0, Math.min(...points.map((p) => p.value)) - 5));
  const hi = $derived(Math.min(100, Math.max(...points.map((p) => p.value)) + 5));
  const x = (i: number) => PAD + (i * (W - 2 * PAD)) / Math.max(1, points.length - 1);
  const y = (v: number) => H - PAD - ((v - lo) * (H - 2 * PAD)) / Math.max(1, hi - lo);
  const path = $derived(points.map((p, i) => `${i ? 'L' : 'M'}${x(i).toFixed(1)},${y(p.value).toFixed(1)}`).join(' '));
</script>

{#if points.length >= 2}
  <svg viewBox="0 0 {W} {H}" role="img" aria-label="Ability by season">
    <line x1={PAD} y1={H - PAD} x2={W - PAD} y2={H - PAD} class="axis" />
    <path d={path} class="line" />
    {#each points as p, i (i)}
      <circle cx={x(i)} cy={y(p.value)} r="3" class="dot" />
      <text x={x(i)} y={y(p.value) - 7} class="value">{p.value}</text>
      {#if points.length <= 8 || i % 2 === 0 || i === points.length - 1}
        <text x={x(i)} y={H - 6} class="label">{p.label}</text>
      {/if}
    {/each}
  </svg>
{/if}

<style>
  svg {
    width: 100%;
    max-width: 480px;
    height: auto;
  }

  .axis {
    stroke: var(--line);
  }

  .line {
    fill: none;
    stroke: var(--accent);
    stroke-width: 2;
  }

  .dot {
    fill: var(--accent);
  }

  text {
    fill: var(--text-dim);
    font-size: 9px;
    text-anchor: middle;
  }

  .value {
    fill: var(--text);
  }
</style>
