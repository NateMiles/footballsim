<!-- A 0-100 bar, coloured by level: green from `good`, amber from `ok`. -->
<script lang="ts">
  let { value, label = '', good = 75, ok = 50 }: { value: number; label?: string; good?: number; ok?: number } = $props();
  const color = $derived(value >= good ? 'var(--good)' : value >= ok ? 'var(--warn)' : 'var(--bad)');
</script>

<span class="meter" title="{label} {value}%" role="meter" aria-label={label} aria-valuenow={value} aria-valuemin={0} aria-valuemax={100}>
  <span class="fill" style:width="{Math.max(0, Math.min(100, value))}%" style:background={color}></span>
</span>

<style>
  .meter {
    display: inline-block;
    width: 48px;
    height: 6px;
    border-radius: 3px;
    background: var(--bg-sunken);
    overflow: hidden;
    vertical-align: middle;
  }

  .fill {
    display: block;
    height: 100%;
  }
</style>
