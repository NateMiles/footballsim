<!-- An attribute on the 1-20 scale, coloured like a scouting report. -->
<script lang="ts">
  let { value }: { value: number } = $props();
  const tier = $derived(value >= 16 ? 'top' : value >= 13 ? 'good' : value >= 9 ? 'ok' : 'low');
</script>

<span class="v {tier}">{value}</span>

<style>
  .v {
    display: inline-block;
    min-width: 2ch;
    text-align: right;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .top {
    color: #7cf2a8;
  }

  .good {
    color: #c7e98a;
  }

  .ok {
    color: var(--text);
  }

  .low {
    color: var(--text-dim);
  }
</style>
