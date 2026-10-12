<!-- A 0-100 dial, coloured by level: green from `good`, amber from `ok`. -->
<script lang="ts">
  let {
    value,
    label,
    size = 112,
    good = 75,
    ok = 50,
  }: { value: number; label: string; size?: number; good?: number; ok?: number } = $props();

  const v = $derived(Math.max(0, Math.min(100, value)));
  const color = $derived(v >= good ? 'var(--good)' : v >= ok ? 'var(--warn)' : 'var(--bad)');
  // A 270° arc open at the bottom, on a 100-unit box.
  const R = 42;
  const SWEEP = 0.75 * 2 * Math.PI * R;
</script>

<span
  class="gauge"
  style:width="{size}px"
  style:height="{size}px"
  role="meter"
  aria-label={label}
  aria-valuenow={v}
  aria-valuemin={0}
  aria-valuemax={100}
>
  <svg viewBox="0 0 100 100">
    <g transform="rotate(135 50 50)">
      <circle class="track" cx="50" cy="50" r={R} stroke-dasharray="{SWEEP} 999" />
      <circle
        class="fill"
        cx="50"
        cy="50"
        r={R}
        stroke={color}
        stroke-dasharray="{(SWEEP * v) / 100} 999"
        style:filter="drop-shadow(0 0 6px {color})"
      />
    </g>
  </svg>
  <span class="value"><b>{v}</b><small>%</small></span>
</span>

<style>
  .gauge {
    position: relative;
    display: inline-grid;
    place-items: center;
    flex: none;
  }

  svg {
    position: absolute;
    inset: 0;
    overflow: visible;
  }

  circle {
    fill: none;
    stroke-width: 7;
    stroke-linecap: round;
  }

  .track {
    stroke: rgba(150, 175, 210, 0.12);
  }

  .fill {
    transition: stroke-dasharray 0.6s var(--ease);
  }

  .value {
    font-family: var(--display);
    font-stretch: 110%;
    font-variant-numeric: tabular-nums;
  }

  .value b {
    font-size: 1.7rem;
    font-weight: 800;
  }

  .value small {
    font-size: 0.85rem;
    color: var(--text-dim);
    margin-left: 1px;
  }
</style>
