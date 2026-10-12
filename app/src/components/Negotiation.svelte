<!-- Contract talks with one of your players (a renewal) or a free agent:
     what their agent asks, your offer, and the agent's answer. -->
<script lang="ts">
  import type { ContractTerms, NegotiationView, TalksResult } from '../api';
  import { longDate, money, wage } from '../format';
  import { useGame } from '../state/game.svelte';
  import TermsEditor from './TermsEditor.svelte';

  let { player }: { player: number } = $props();
  const game = useGame();

  let view = $state<NegotiationView | null>(null);
  let terms = $state<ContractTerms | null>(null);
  let result = $state<TalksResult | null>(null);

  async function open() {
    const v = await game.run('Calling the agent', () => game.client.contractTalks(player));
    if (v) {
      view = v;
      terms = { ...v.ask };
      result = null;
    }
  }

  async function offer() {
    if (!terms) return;
    const t = { ...terms };
    const r = await game.run('Negotiating', () => game.client.negotiateContract(player, t));
    if (!r) return;
    result = r;
    view = r.negotiation;
    if (r.outcome.type === 'countered') terms = { ...r.outcome.counter };
    if (r.outcome.type === 'agreed') game.changed();
  }

  const tone = (r: TalksResult) => (r.outcome.type === 'agreed' ? 'good' : r.outcome.type === 'countered' ? 'warn' : 'bad');
</script>

{#if !view}
  <button class="primary" onclick={open}>Negotiate a contract</button>
{:else}
  {@const v = view}
  <div class="grid talks" data-testid="negotiation">
    <p>
      <strong>Agent:</strong> {v.agent} ({v.agentFeePercent}% fee on the contract's wages)
      · {v.offersLeft} offer{v.offersLeft === 1 ? '' : 's'} before they walk away
    </p>
    <p class="dim">
      Asks {wage(v.ask.wage)} for {v.ask.years} year{v.ask.years === 1 ? '' : 's'} as
      {v.statuses.find((s) => s.status === v.ask.status)?.name.toLowerCase()}, a {money(v.ask.signingFee)} signing fee{#if v.wantsClause},
        and insists on a release clause (about {money(v.ask.releaseClause ?? 0)}){/if}.
      Wage bill {wage(v.wageBill)} of {wage(v.wageBudget)}.
    </p>
    {#if v.wantsOut}
      <p class="pill bad">Wants to leave and will not sign a new contract.</p>
    {:else if v.closedUntil}
      <p class="pill bad">Talks broke down; they reopen on {longDate(v.closedUntil)}.</p>
    {:else if terms}
      <TermsEditor bind:terms statuses={v.statuses} />
      <div class="row">
        <button class="primary" disabled={game.busy !== null} onclick={offer}>Make offer</button>
        <span class="dim small">Bonuses are valued at the appearances and goals the role promises, a little discounted.</span>
      </div>
    {/if}
    {#if result}
      <p data-testid="talks-result">
        <span class="pill {tone(result)}">{result.outcome.type === 'brokenOff' ? 'Broken off' : result.outcome.type}</span>
        {result.message}
      </p>
    {/if}
  </div>
{/if}

<style>
  .talks p {
    margin: 0;
  }

  .small {
    font-size: 0.85rem;
  }
</style>
