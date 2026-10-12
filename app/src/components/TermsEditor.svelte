<!-- A contract package: wage, years, role, bonuses, signing fee and
     release clause. Edits `terms` in place. -->
<script lang="ts">
  import type { ContractTerms, StatusInfo } from '../api';
  import { money } from '../format';

  let { terms = $bindable(), statuses }: { terms: ContractTerms; statuses: StatusInfo[] } = $props();

  // Money in thousands for the fee fields.
  const k = (v: number) => Math.round(v / 1000);
  let clauseOn = $derived(terms.releaseClause !== null);
</script>

<div class="terms">
  <label>Wage (p/w) <input type="number" min="0" step="50" bind:value={terms.wage} /></label>
  <label>Years
    <select bind:value={terms.years}>{#each [1, 2, 3, 4, 5] as y (y)}<option value={y}>{y}</option>{/each}</select>
  </label>
  <label>Role
    <select bind:value={terms.status}>
      {#each statuses as s (s.status)}<option value={s.status}>{s.name}</option>{/each}
    </select>
  </label>
  <label>Per appearance <input type="number" min="0" step="50" bind:value={terms.appearanceBonus} /></label>
  <label>Per goal <input type="number" min="0" step="50" bind:value={terms.goalBonus} /></label>
  <label>Signing fee (£k)
    <input
      type="number"
      min="0"
      step="5"
      value={k(terms.signingFee)}
      oninput={(e) => (terms.signingFee = Number(e.currentTarget.value) * 1000)}
    />
  </label>
  <label class="clause">
    <input
      type="checkbox"
      checked={clauseOn}
      onchange={(e) => (terms.releaseClause = e.currentTarget.checked ? (terms.releaseClause ?? 10_000_000) : null)}
    />
    Release clause
    {#if terms.releaseClause !== null}
      <input
        type="number"
        min="0"
        step="100"
        aria-label="Release clause (£k)"
        value={k(terms.releaseClause)}
        oninput={(e) => (terms.releaseClause = Number(e.currentTarget.value) * 1000)}
      />
      <span class="dim">£k ({money(terms.releaseClause)})</span>
    {/if}
  </label>
</div>

<style>
  .terms {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 8px;
    align-items: center;
  }

  label {
    display: flex;
    gap: 6px;
    align-items: center;
    justify-content: space-between;
  }

  input[type='number'] {
    width: 100px;
  }

  .clause {
    grid-column: 1 / -1;
    justify-content: flex-start;
  }
</style>
