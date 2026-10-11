<!-- A bid for a player: a transfer fee and contract, or a loan. -->
<script lang="ts">
  import { untrack } from 'svelte';
  import type { OfferKind, PlayerView } from '../api';
  import { money, wage } from '../format';

  let {
    player,
    budget,
    onsubmit,
  }: {
    player: PlayerView;
    budget: number;
    onsubmit: (kind: OfferKind, wage: number, years: number) => void;
  } = $props();

  // The form starts from the player as first shown; edits are the user's.
  const start = untrack(() => ({
    kind: player.listing === 'loan' ? ('loan' as const) : ('transfer' as const),
    fee: Math.round(((player.askingPrice ?? player.value) / 1e6) * 10) / 10,
    wage: Math.ceil((player.wageDemand * 1.1) / 100) * 100,
  }));
  let kind = $state<'transfer' | 'loan'>(start.kind);
  // Fees in millions, wages per week.
  let fee = $state(start.fee);
  let weekly = $state(start.wage);
  let years = $state(3);
  let share = $state(100);
  const free = $derived(player.club === null);

  function submit() {
    const k: OfferKind = kind === 'loan' ? { type: 'loan', wageShare: share } : { type: 'transfer', fee: Math.round(fee * 1e6) };
    onsubmit(k, kind === 'loan' ? 0 : weekly, kind === 'loan' ? 0 : years);
  }
</script>

<div class="form">
  {#if !free}
    <div class="seg" role="group" aria-label="Kind of offer">
      <button class:active={kind === 'transfer'} onclick={() => (kind = 'transfer')}>Transfer</button>
      <button class:active={kind === 'loan'} onclick={() => (kind = 'loan')}>Loan</button>
    </div>
  {/if}
  {#if kind === 'transfer'}
    {#if !free}
      <label>Fee (£m) <input type="number" min="0" step="0.1" bind:value={fee} /></label>
    {/if}
    <label>Wage <input type="number" min="0" step="100" bind:value={weekly} /></label>
    <label>Years
      <select bind:value={years}>{#each [1, 2, 3, 4, 5] as y (y)}<option value={y}>{y}</option>{/each}</select>
    </label>
    <p class="dim">
      {#if player.askingPrice}They would sell for about {money(player.askingPrice)}.{/if}
      The player wants about {wage(player.wageDemand)}. Your budget: {money(budget)}.
    </p>
  {:else}
    <label>You pay <input type="number" min="0" max="100" step="5" bind:value={share} />% of wages</label>
  {/if}
  <button class="primary" onclick={submit}>{free ? 'Offer contract' : 'Make offer'}</button>
</div>

<style>
  .form {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  label {
    display: inline-flex;
    gap: 6px;
    align-items: center;
  }

  input[type='number'] {
    width: 110px;
  }

  .seg button.active {
    border-color: var(--accent);
    color: var(--accent);
  }

  p {
    flex-basis: 100%;
    margin: 0;
  }
</style>
