<!-- The club's money: balance, budgets, this season and past seasons. -->
<script lang="ts">
  import type { Ledger } from '../api';
  import { money } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  const game = useGame();
  const view = load(() => game.client.finances());

  const income: [keyof Ledger, string][] = [
    ['matchday', 'Gate receipts'],
    ['broadcast', 'Television'],
    ['commercial', 'Sponsorship and merchandise'],
    ['prizeMoney', 'Prize money'],
    ['playerSales', 'Player sales'],
  ];
  const spending: [keyof Ledger, string][] = [
    ['playerWages', 'Player wages'],
    ['staffWages', 'Staff wages'],
    ['playerPurchases', 'Transfer fees'],
    ['runningCosts', 'Running costs'],
    ['investment', 'Facilities'],
    ['bonuses', 'Bonuses'],
    ['signingFees', 'Signing and agent fees'],
  ];
</script>

{#if view.data}
  {@const f = view.data}
  <div class="screen grid">
    <h1>Finances</h1>
    <section class="tiles">
      <div class="card tile">
        <span class="dim">Balance</span>
        <strong class:bad={f.balance < 0}>{money(f.balance)}</strong>
      </div>
      <div class="card tile">
        <span class="dim">Transfer budget</span>
        <strong>{money(f.transferBudget)}</strong>
      </div>
      <div class="card tile">
        <span class="dim">Wages per week</span>
        <strong class:bad={f.wageBill > f.wageBudget}>{money(f.wageBill)}</strong>
        <span class="dim small">of {money(f.wageBudget)} budget</span>
      </div>
      <div class="card tile">
        <span class="dim">Staff per week</span>
        <strong>{money(f.staffBill)}</strong>
      </div>
    </section>

    <div class="cols">
      <section class="card">
        <h3>This season</h3>
        <table class="data">
          <tbody>
            {#each income as [k, label] (k)}
              <tr><td>{label}</td><td class="num good">{money(f.season[k])}</td></tr>
            {/each}
            {#each spending as [k, label] (k)}
              <tr><td>{label}</td><td class="num">−{money(f.season[k])}</td></tr>
            {/each}
            <tr class="total">
              <td>Net</td>
              <td class="num" class:bad={f.season.income < f.season.expenses}>{money(f.season.income - f.season.expenses)}</td>
            </tr>
          </tbody>
        </table>
      </section>
      <section class="card">
        <h3>Ground and facilities</h3>
        <p><span class="dim">Capacity</span> <strong>{f.facilities.capacity.toLocaleString('en-GB')}</strong></p>
        <p><span class="dim">Training ground</span> <strong>{f.facilities.training}/20</strong></p>
        <p><span class="dim">Youth academy</span> <strong>{f.facilities.youth}/20</strong></p>
        <p class="dim small">Facilities wear over time. The board invests spare money in them each summer.</p>
      </section>
    </div>

    {#if f.history.length > 0}
      <section class="card scroll-x">
        <h3>Past seasons</h3>
        <table class="data">
          <thead>
            <tr><th>Season</th><th class="num">Income</th><th class="num">Spending</th><th class="num">Wages</th><th class="num">Balance</th></tr>
          </thead>
          <tbody>
            {#each f.history as h (h.season)}
              <tr>
                <td>{h.season}</td>
                <td class="num">{money(h.ledger.income)}</td>
                <td class="num">{money(h.ledger.expenses)}</td>
                <td class="num">{money(h.ledger.playerWages)}</td>
                <td class="num" class:bad={h.closingBalance < 0}>{money(h.closingBalance)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>
    {/if}
  </div>
{/if}

<style>
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
    gap: var(--gap);
  }

  .tile {
    display: grid;
    gap: 4px;
  }

  .tile strong {
    font-size: 1.4rem;
  }

  .small {
    font-size: 0.85rem;
  }

  .cols {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: var(--gap);
  }

  .good {
    color: var(--good);
  }

  .bad {
    color: var(--bad);
  }

  tr.total td {
    font-weight: 700;
  }
</style>
