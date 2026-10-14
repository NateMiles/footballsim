<!-- One player: attributes, condition, contract and career. -->
<script lang="ts">
  import AttrValue from '../components/AttrValue.svelte';
  import ClubName from '../components/ClubName.svelte';
  import DevelopmentChart from '../components/DevelopmentChart.svelte';
  import Meter from '../components/Meter.svelte';
  import Negotiation from '../components/Negotiation.svelte';
  import OfferForm from '../components/OfferForm.svelte';
  import PlayerMood from '../components/PlayerMood.svelte';
  import Portrait from '../components/Portrait.svelte';
  import type { ContractTerms, Listing, OfferKind } from '../api';
  import { longDate, money, wage } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  let { id }: { id: number } = $props();
  const game = useGame();
  const player = load(() => game.client.player(id));
  const p = $derived(player.data);
  let confirmRelease = $state(false);

  const positions = ['GK', 'DR', 'DC', 'DL', 'WBR', 'WBL', 'DM', 'MR', 'MC', 'ML', 'AMR', 'AMC', 'AML', 'ST'];


  const own = $derived(p?.own ?? false);
  const mine = $derived(game.overview?.club ?? null);
  const development = $derived(
    p ? [...p.career.map((c) => ({ label: c.season.slice(2), value: c.ability })), { label: 'now', value: p.ability }] : [],
  );

  async function bid(kind: OfferKind, wage: number, years: number, terms?: ContractTerms, sellOn?: number) {
    if (!p) return;
    const r = await game.run('Making the offer', () => game.client.makeOffer(id, kind, wage, years, terms, sellOn));
    if (r) game.changed();
  }

  async function scout(on: boolean) {
    const r = await game.run('Updating', () => (on ? game.client.scoutPlayer(id) : game.client.cancelScouting(id)));
    if (r) game.changed();
  }

  async function list(listing: Listing | null) {
    const r = await game.run('Updating', () => game.client.setListing(id, listing));
    if (r) game.changed();
  }

  async function release() {
    const r = await game.run('Releasing', () => game.client.releasePlayer(id));
    if (r) {
      await game.refresh();
      game.router.go({ name: 'squad' });
    }
  }
</script>

{#if p}
  <div class="screen grid">
    <section
      class="card head"
      style:--club={p.club?.colors.primary ?? '#2a3a52'}
    >
      <div class="face">
        <Portrait
          look={p.look}
          size={112}
          ring
          shirt={p.club?.colors.primary}
          trim={p.club?.colors.secondary}
          title={p.name}
        />
        <span class="num-badge">{p.number ?? '–'}</span>
      </div>
      <div class="who">
        <h1>{p.name}</h1>
        <p class="dim">
          {p.position} · {p.age} years ({longDate(p.birthDate)}) · {p.nation} · {p.heightCm} cm · {p.foot} foot
        </p>
        {#if p.club}<ClubName club={p.club} />{:else}<span class="pill">{p.retired ? 'Retired' : 'Free agent'}</span>{/if}
      </div>
      <div class="abil">
        <span class="dim">Ability</span>
        <strong>{p.ability}{#if p.abilityHigh > p.abilityLow}<small class="dim"> {p.abilityLow}-{p.abilityHigh}</small>{/if}</strong>
        <span class="dim">Potential</span>
        <strong>{p.potential}{#if p.potentialHigh > p.potentialLow}<small class="dim"> {p.potentialLow}-{p.potentialHigh}</small>{/if}</strong>
        {#if p.knowledge < 100}
          <span class="dim">Known</span> <strong>{p.knowledge}%</strong>
        {/if}
      </div>
    </section>

    <section class="attrs">
      {#each p.attributes as g (g.name)}
        <div class="card">
          <h3>{g.name}</h3>
          {#each g.attributes as a (a.name)}
            <div class="attr">
              <span>{a.name}</span>
              <span>
                {#if a.high > a.low}<small class="dim">{a.low}-{a.high}</small>{/if}
                <AttrValue value={a.value} />
              </span>
            </div>
          {/each}
        </div>
      {/each}
      <div class="card">
        <h3>Positions</h3>
        <div class="positions">
          {#each positions as code, i (code)}
            {@const r = p.positionRatings[i]}
            <span class="posr" class:natural={r >= 18} class:good={r >= 13 && r < 18} class:none={r < 8} title="{code}: {r}/20">{code}</span>
          {/each}
        </div>
        <h3 class="gap">Condition</h3>
        <div class="attr"><span>Fitness</span><Meter value={p.fitness} label="Fitness" /></div>
        <div class="attr"><span>Sharpness</span><Meter value={p.sharpness} label="Sharpness" /></div>
        <div class="attr"><span>Morale</span><Meter value={p.morale} label="Morale" good={60} ok={40} /></div>
        {#if p.injury}<p class="pill bad">Injured ({p.injury.severity}) until {longDate(p.injury.until)}</p>{/if}
        {#if p.suspension > 0}<p class="pill warn">Suspended for {p.suspension} matches</p>{/if}
      </div>
    </section>

    <PlayerMood {p} />

    {#if !p.retired && mine}
      <section class="card market">
        <h3>Market</h3>
        <p>
          {#if p.club}<span class="dim">Estimated value</span> <strong>{money(p.value)}</strong>{/if}
          {#if p.askingPrice}· <span class="dim">They would sell for about</span> <strong>{money(p.askingPrice)}</strong>{/if}
          {#if p.listing}<span class="pill warn">{p.listing === 'loan' ? 'Available for loan' : 'Listed for sale'}</span>{/if}
          {#if p.loan}<span class="pill">On loan at {p.loan.borrower.name} from {p.loan.parent.name}</span>{/if}
        </p>
        {#if own}
          {#if !p.loan}
            <div class="row">
              <span class="dim">List the player:</span>
              <button class:active={p.listing === null} onclick={() => list(null)}>Not listed</button>
              <button class:active={p.listing === 'transfer'} onclick={() => list('transfer')}>For sale</button>
              <button class:active={p.listing === 'loan'} onclick={() => list('loan')}>For loan</button>
            </div>
          {/if}
        {:else}
          <div class="row">
            {#if p.knowledge < 100}
              {#if p.scouting}
                <span class="pill good">Being scouted</span>
                <button onclick={() => scout(false)}>Stop scouting</button>
              {:else}
                <button onclick={() => scout(true)}>Scout this player</button>
              {/if}
            {/if}
          </div>
          {#if p.offer}
            <p>Your offer: <strong>{p.offer.status.type === 'refused' ? p.offer.status.reason : p.offer.status.type}</strong> · <a href="#/transfers/offers">Transfers</a></p>
          {:else if !p.club && !p.retired}
            <Negotiation player={p.id} />
          {:else if !p.loan && game.overview?.windowOpen}
            <OfferForm player={p} budget={game.overview?.transferBudget ?? 0} onsubmit={bid} />
          {:else if !p.loan}
            <p class="dim">The transfer window is closed.</p>
          {/if}
        {/if}
      </section>
    {/if}

    <div class="cols">
      <section class="card">
        <h3>Contract</h3>
        {#if p.contract}
          <p>{wage(p.contract.wage)} until {longDate(p.contract.expires)}</p>
          {#if p.contract.appearanceBonus > 0 || p.contract.goalBonus > 0}
            <p class="dim">
              Bonuses: {money(p.contract.appearanceBonus)} an appearance, {money(p.contract.goalBonus)} a goal.
            </p>
          {/if}
          {#if p.contract.releaseClause !== null}
            <p class="dim">Release clause: {money(p.contract.releaseClause)}.</p>
          {/if}
        {:else}
          <p class="dim">No contract.</p>
        {/if}
        {#if p.sellOn}<p class="dim">{p.sellOn.club.name} are owed {p.sellOn.percent}% of the next fee.</p>{/if}
        <p class="dim">Worth about {wage(p.wageDemand)}.</p>
        {#if p.own && !p.loan}
          <Negotiation player={p.id} />
        {/if}
        {#if p.own}
          <div class="row gap">
            {#if confirmRelease}
              <span>Release {p.lastName}? They leave today.</span>
              <button class="danger" onclick={release}>Release</button>
              <button onclick={() => (confirmRelease = false)}>Keep</button>
            {:else}
              <button class="danger" onclick={() => (confirmRelease = true)}>Release player…</button>
            {/if}
          </div>
        {/if}
      </section>

      <section class="card">
        <h3>This season</h3>
        <div class="stats">
          <span>Apps</span><strong>{p.season.appearances} ({p.season.starts})</strong>
          <span>Goals</span><strong>{p.season.goals}</strong>
          <span>Assists</span><strong>{p.season.assists}</strong>
          <span>Rating</span><strong>{p.season.rating?.toFixed(2) ?? '–'}</strong>
          <span>Cards</span><strong>{p.season.yellows} / {p.season.reds}</strong>
        </div>
      </section>
    </div>

    {#if development.length >= 2}
      <section class="card">
        <h3>Development</h3>
        <DevelopmentChart points={development} />
      </section>
    {/if}

    {#if p.career.length > 0}
      <section class="card scroll-x">
        <h3>Career</h3>
        <table class="data">
          <thead><tr><th>Season</th><th>Club</th><th class="num">Apps</th><th class="num">Gls</th><th class="num">Ast</th><th class="num">Rating</th><th class="num">Abil</th></tr></thead>
          <tbody>
            {#each p.career as c (c.season)}
              <tr>
                <td>{c.season}</td>
                <td>{#if c.club}<ClubName club={c.club} short />{:else}<span class="dim">–</span>{/if}</td>
                <td class="num">{c.stats.appearances}</td>
                <td class="num">{c.stats.goals}</td>
                <td class="num">{c.stats.assists}</td>
                <td class="num">{c.stats.rating?.toFixed(2) ?? '–'}</td>
                <td class="num">{c.ability}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>
    {/if}
  </div>
{/if}

<style>
  .head {
    display: flex;
    gap: 22px;
    align-items: center;
    flex-wrap: wrap;
    overflow: hidden;
    background:
      linear-gradient(110deg, color-mix(in srgb, var(--club) 30%, transparent), transparent 55%),
      var(--bg-raised);
  }

  .face {
    position: relative;
  }

  .num-badge {
    position: absolute;
    right: -4px;
    bottom: 2px;
    min-width: 34px;
    height: 34px;
    padding: 0 6px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    background: var(--bg);
    border: 1px solid var(--line-strong);
    font-family: var(--display);
    font-size: 1.05rem;
    font-weight: 800;
  }

  .who h1 {
    font-size: clamp(1.6rem, 3.5vw, 2.4rem);
    margin-bottom: 4px;
  }

  .abil {
    margin-left: auto;
    display: grid;
    grid-template-columns: auto auto;
    gap: 2px 10px;
    align-items: baseline;
  }

  .abil strong {
    font-family: var(--display);
    font-size: 1.6rem;
    font-weight: 800;
  }

  .attrs {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: var(--gap);
  }

  .attr {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 3px 0;
  }

  .positions {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 4px;
  }

  .posr {
    text-align: center;
    font-size: 0.75rem;
    padding: 4px 0;
    border-radius: 4px;
    background: var(--bg-sunken);
    color: var(--text-dim);
  }

  .posr.natural {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 700;
  }

  .posr.good {
    background: rgba(61, 220, 132, 0.3);
    color: var(--text);
  }

  .posr.none {
    opacity: 0.4;
  }

  .gap {
    margin-top: 14px;
  }

  .cols {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: var(--gap);
  }

  .stats {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 16px;
  }

  .market .row button.active {
    border-color: var(--accent);
    color: var(--accent);
  }

  small {
    font-size: 0.75rem;
    font-weight: 400;
    margin-right: 4px;
  }
</style>
