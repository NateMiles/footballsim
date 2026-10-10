<!-- A formation on a vertical pitch, own goal at the bottom. Tapping a
     slot selects it. -->
<script lang="ts">
  import type { SlotInfo } from '../api';

  type Occupant = { label: string; sub?: string; warn?: boolean } | null;
  let {
    slots,
    occupants,
    positions,
    selected = null,
    onselect,
  }: {
    slots: SlotInfo[];
    occupants: Occupant[];
    positions: string[];
    selected?: number | null;
    onselect?: (slot: number) => void;
  } = $props();
</script>

<div class="pitch" role="group" aria-label="Formation">
  <div class="lines" aria-hidden="true">
    <span class="half"></span>
    <span class="circle"></span>
    <span class="box top"></span>
    <span class="box bottom"></span>
  </div>
  {#each slots as s, i (i)}
    {@const o = occupants[i]}
    <button
      class="slot"
      class:selected={selected === i}
      class:warn={o?.warn}
      style:left="{s.y * 100}%"
      style:top="{(1 - s.x) * 88 + 4}%"
      onclick={() => onselect?.(i)}
      aria-label="{positions[s.position]}: {o?.label ?? 'empty'}"
    >
      <span class="pos">{positions[s.position]}</span>
      <span class="who">{o?.label ?? '—'}</span>
      {#if o?.sub}<span class="sub">{o.sub}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .pitch {
    position: relative;
    aspect-ratio: 68 / 90;
    width: 100%;
    max-width: 460px;
    background: repeating-linear-gradient(0deg, var(--pitch) 0 10%, #1c6335 10% 20%);
    border-radius: var(--radius);
    border: 2px solid var(--pitch-line);
    overflow: hidden;
  }

  .lines span {
    position: absolute;
    border: 2px solid var(--pitch-line);
  }

  .half {
    left: 0;
    right: 0;
    top: 50%;
    border-width: 2px 0 0 0 !important;
  }

  .circle {
    left: 50%;
    top: 50%;
    width: 26%;
    aspect-ratio: 1;
    border-radius: 50%;
    transform: translate(-50%, -50%);
  }

  .box {
    left: 22%;
    right: 22%;
    height: 14%;
  }

  .box.top {
    top: -2px;
  }

  .box.bottom {
    bottom: -2px;
  }

  .slot {
    position: absolute;
    transform: translate(-50%, -50%);
    width: 86px;
    min-height: 0;
    padding: 4px 2px;
    display: grid;
    justify-items: center;
    gap: 1px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.35);
    background: rgba(8, 20, 12, 0.82);
    font-size: 0.78rem;
    line-height: 1.15;
  }

  .slot.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent);
  }

  .slot.warn .who {
    color: var(--warn);
  }

  .pos {
    font-size: 0.68rem;
    color: var(--accent);
    font-weight: 700;
  }

  .who {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub {
    font-size: 0.68rem;
    color: var(--text-dim);
  }

  @media (max-width: 500px) {
    .slot {
      width: 70px;
      font-size: 0.7rem;
    }
  }
</style>
