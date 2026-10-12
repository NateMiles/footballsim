<!-- The transfer market: find players, handle offers, follow scouting,
     and see what is moving. -->
<script lang="ts">
  import type { Offer, OfferStatus, PlayerSearch, SearchView } from '../api';
  import ClubName from '../components/ClubName.svelte';
  import { longDate, money, shortDate, wage } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';
  import type { TransferTab } from '../state/router.svelte';

  let { tab = 'search' }: { tab?: TransferTab } = $props();
  const game = useGame();
  const view = load(() => game.client.transfers());

  const TABS: { tab: TransferTab; label: string }[] = [
    { tab: 'search', label: 'Search' },
    { tab: 'offers', label: 'Offers' },
    { tab: 'scouting', label: 'Scouting' },
    { tab: 'squad', label: 'Listed and loans' },
    { tab: 'news', label: 'News' },
  ];

  // Search.
  const POSITIONS = ['GK', 'DR', 'DC', 'DL', 'WBR', 'WBL', 'DM', 'MR', 'MC', 'ML', 'AMR', 'AMC', 'AML', 'ST'];
  let position = $state('');
  let maxAge = $state<number | null>(null);
  let maxValue = $state<number | null>(null);
  let minAbility = $state<number | null>(null);
  let listedOnly = $state(false);
  let freeOnly = $state(false);
  let name = $state('');
  let results = $state<SearchView | null>(null);

  async function search() {
    const q: PlayerSearch = {
      positions: position ? [position] : [],
      minAge: null,
      maxAge: maxAge || null,
      maxValue: maxValue ? maxValue * 1_000_000 : null,
      minAbility: minAbility || null,
      listedOnly,
      freeAgentsOnly: freeOnly,
      name: name.trim() || null,
    };
    results = (await game.run('Searching', () => game.client.searchPlayers(q))) ?? results;
  }

  // Offers.
  let counter = $state<Record<number, number>>({});

  function statusText(s: OfferStatus): string {
    switch (s.type) {
      case 'pending':
        return 'Waiting for an answer';
      case 'countered':
        return `They want ${money(s.fee)}`;
      case 'agreed':
        return 'Agreed: completes tomorrow';
      case 'completed':
        return 'Completed';
      case 'refused':
        return s.reason;
    }
  }

  function terms(o: Offer): string {
    return o.kind.type === 'transfer'
      ? `${money(o.kind.fee)}, ${wage(o.wage)} for ${o.years} years`
      : `Loan, paying ${o.kind.wageShare}% of wages`;
  }

  async function respond(o: Offer, response: 'accept' | 'reject' | 'counter') {
    const r = await game.run('Answering', () =>
      game.client.respondToOffer(
        o.id,
        response === 'counter' ? { type: 'counter', fee: (counter[o.id] ?? 0) * 1_000_000 } : { type: response },
      ),
    );
    if (r) game.changed();
  }

  async function acceptCounter(o: Offer) {
    const r = await game.run('Accepting', () => game.client.acceptCounter(o.id));
    if (r) game.changed();
  }

  async function withdraw(o: Offer) {
    const r = await game.run('Withdrawing', () => game.client.withdrawOffer(o.id));
    if (r) game.changed();
  }

  async function cancelScouting(player: number) {
    const r = await game.run('Updating', () => game.client.cancelScouting(player));
    if (r) game.changed();
  }

  const open = (o: Offer) => o.status.type === 'pending' || o.status.type === 'countered' || o.status.type === 'agreed';
</script>

{#if view.data}
  {@const v = view.data}
  <div class="screen grid">
    <div class="row head">
      <h1>Transfers</h1>
      <span class="pill" class:good={v.windowOpen}>
        Window {v.windowOpen ? 'open' : 'closed'} · {v.windowOpen ? 'closes' : 'opens'} {longDate(v.windowChanges)}
      </span>
      <span class="spacer"></span>
      <span><span class="dim">Budget</span> <strong>{money(v.transferBudget)}</strong></span>
    </div>

    <div class="tabs" role="tablist">
      {#each TABS as t (t.tab)}
        <a role="tab" aria-selected={tab === t.tab} class:active={tab === t.tab} href="#/transfers/{t.tab}">
          {t.label}
          {#if t.tab === 'offers' && v.incoming.some((o) => o.status.type === 'pending')}<span class="dot"></span>{/if}
        </a>
      {/each}
    </div>

    {#if tab === 'search'}
      <section class="card filters">
        <select bind:value={position} aria-label="Position">
          <option value="">Any position</option>
          {#each POSITIONS as p (p)}<option value={p}>{p}</option>{/each}
        </select>
        <input type="number" min="16" max="40" placeholder="Max age" bind:value={maxAge} aria-label="Max age" />
        <input type="number" min="0" placeholder="Max £m" bind:value={maxValue} aria-label="Max value in millions" />
        <input type="number" min="0" max="100" placeholder="Min ability" bind:value={minAbility} aria-label="Min ability" />
        <input type="text" placeholder="Name" bind:value={name} aria-label="Name" />
        <label class="check"><input type="checkbox" bind:checked={listedOnly} /> Listed</label>
        <label class="check"><input type="checkbox" bind:checked={freeOnly} /> Free agents</label>
        <button class="primary" onclick={search}>Search</button>
      </section>
      {#if results}
        <p class="dim">{results.total} players match{results.total > results.rows.length ? `; the best ${results.rows.length} by estimated ability` : ''}. Ability is your scouts' estimate.</p>
        <section class="card scroll-x">
          <table class="data">
            <thead>
              <tr><th>Name</th><th>Pos</th><th class="num">Age</th><th>Club</th><th class="num">Ability</th><th class="num">Pot</th><th class="num">Known</th><th class="num">Value</th><th>Wants</th><th></th></tr>
            </thead>
            <tbody>
              {#each results.rows as r (r.player.id)}
                <tr class="clickable" onclick={() => game.router.go({ name: 'player', id: r.player.id })}>
                  <td><a href="#/player/{r.player.id}">{r.player.name}</a></td>
                  <td>{r.player.position}</td>
                  <td class="num">{r.age}</td>
                  <td>{#if r.club}<ClubName club={r.club} short link={false} />{:else}<span class="dim">Free agent</span>{/if}</td>
                  <td class="num" title="{r.abilityLow}-{r.abilityHigh}">{r.ability}{#if r.abilityHigh > r.abilityLow}<span class="dim">±{Math.round((r.abilityHigh - r.abilityLow) / 2)}</span>{/if}</td>
                  <td class="num dim">{r.potential}</td>
                  <td class="num dim">{r.knowledge}%</td>
                  <td class="num">{r.value ? money(r.value) : '–'}</td>
                  <td>{wage(r.wageDemand)}</td>
                  <td>{#if r.listing}<span class="pill warn">{r.listing === 'loan' ? 'Loan' : 'For sale'}</span>{/if}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </section>
      {/if}
    {:else if tab === 'offers'}
      <section class="card">
        <h3>Offers for your players</h3>
        {#each v.incoming as o (o.id)}
          <div class="offer">
            <div>
              <strong><a href="#/player/{o.player.id}">{o.player.name}</a></strong>
              <span class="dim">· {o.buyer.name} · {shortDate(o.date)}</span>
              <div>{terms(o)}</div>
              <div class="dim">{statusText(o.status)}{o.status.type === 'pending' ? ` (answer by ${shortDate(o.respondBy)})` : ''}</div>
            </div>
            {#if o.status.type === 'pending'}
              <div class="actions">
                <button class="primary" onclick={() => respond(o, 'accept')}>Accept</button>
                <button onclick={() => respond(o, 'reject')}>Reject</button>
                {#if o.kind.type === 'transfer'}
                  <input type="number" min="0" step="0.1" placeholder="£m" bind:value={counter[o.id]} aria-label="Counter fee in millions" />
                  <button disabled={!counter[o.id]} onclick={() => respond(o, 'counter')}>Ask for</button>
                {/if}
              </div>
            {/if}
          </div>
        {:else}
          <p class="dim">No offers.</p>
        {/each}
      </section>
      <section class="card">
        <h3>Your offers</h3>
        {#each v.outgoing as o (o.id)}
          <div class="offer">
            <div>
              <strong><a href="#/player/{o.player.id}">{o.player.name}</a></strong>
              <span class="dim">· {o.seller.name} · {shortDate(o.date)}</span>
              <div>{terms(o)}</div>
              <div class:dim={!open(o)}>{statusText(o.status)}</div>
            </div>
            <div class="actions">
              {#if o.status.type === 'countered'}
                <button class="primary" onclick={() => acceptCounter(o)}>Pay {money(o.status.fee)}</button>
              {/if}
              {#if o.status.type === 'pending' || o.status.type === 'countered'}
                <button onclick={() => withdraw(o)}>Withdraw</button>
              {/if}
            </div>
          </div>
        {:else}
          <p class="dim">You have made no offers. Find a player and bid from their page.</p>
        {/each}
      </section>
    {:else if tab === 'scouting'}
      <section class="card">
        <h3>Being scouted · {v.scouts} scout{v.scouts === 1 ? '' : 's'} at work</h3>
        <p class="dim">Each scout watches one player at a time, in this order. Reports arrive in your inbox.</p>
        {#each v.scouting as a, i (a.player.id)}
          <div class="offer">
            <div>
              <strong><a href="#/player/{a.player.id}">{a.player.name}</a></strong>
              <span class="dim">· {a.club?.name ?? 'Free agent'}{i < v.scouts ? ' · being watched' : ' · waiting'}</span>
              <div class="bar"><span style:width="{a.knowledge}%"></span></div>
            </div>
            <div class="actions"><button onclick={() => cancelScouting(a.player.id)}>Stop</button></div>
          </div>
        {:else}
          <p class="dim">No assignments. Open a player's page and choose Scout.</p>
        {/each}
      </section>
    {:else if tab === 'squad'}
      <div class="cols">
        <section class="card">
          <h3>Listed</h3>
          {#each v.listed as l (l.player.id)}
            <p><a href="#/player/{l.player.id}">{l.player.name}</a> <span class="pill">{l.listing === 'loan' ? 'Loan' : 'For sale'}</span></p>
          {:else}
            <p class="dim">List players from their page to attract offers.</p>
          {/each}
        </section>
        <section class="card">
          <h3>Out on loan</h3>
          {#each v.loansOut as l (l.player.id)}
            <p><a href="#/player/{l.player.id}">{l.player.name}</a> <span class="dim">at {l.borrower.name} until {shortDate(l.until)}</span></p>
          {:else}
            <p class="dim">Nobody.</p>
          {/each}
          <h3 class="gap">On loan here</h3>
          {#each v.loansIn as l (l.player.id)}
            <p><a href="#/player/{l.player.id}">{l.player.name}</a> <span class="dim">from {l.parent.name}, paying {l.wageShare}%</span></p>
          {:else}
            <p class="dim">Nobody.</p>
          {/each}
        </section>
      </div>
    {:else}
      <section class="card scroll-x">
        <h3>Recent moves</h3>
        <table class="data">
          <tbody>
            {#each v.recent as r, i (i)}
              <tr>
                <td class="dim">{shortDate(r.date)}</td>
                <td><a href="#/player/{r.player.id}">{r.player.name}</a></td>
                <td><ClubName club={r.from} short /> → <ClubName club={r.to} short /></td>
                <td class="num">{r.loan ? 'Loan' : money(r.fee)}</td>
              </tr>
            {:else}
              <tr><td class="dim">No moves in the last two months.</td></tr>
            {/each}
          </tbody>
        </table>
      </section>
    {/if}
  </div>
{/if}

<style>
  .head {
    gap: 12px;
  }

  .spacer {
    flex: 1;
  }

  .tabs {
    display: flex;
    gap: 4px;
    overflow-x: auto;
    border-bottom: 1px solid var(--line);
  }

  .tabs a {
    padding: 10px 14px;
    color: var(--text-dim);
    border-bottom: 2px solid transparent;
    white-space: nowrap;
    position: relative;
  }

  .tabs a.active {
    color: var(--accent);
    border-color: var(--accent);
  }

  .dot {
    position: absolute;
    top: 8px;
    right: 4px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
  }

  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .filters input[type='number'] {
    width: 130px;
  }

  .check {
    display: inline-flex;
    gap: 6px;
    align-items: center;
  }

  .offer {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    padding: 10px 0;
    border-bottom: 1px solid var(--line);
  }

  .actions {
    display: flex;
    gap: 6px;
    align-items: center;
    flex-wrap: wrap;
  }

  .actions input {
    width: 90px;
  }

  .bar {
    width: 200px;
    height: 6px;
    margin-top: 6px;
    border-radius: 3px;
    background: var(--bg-sunken);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
  }

  .cols {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: var(--gap);
  }

  .gap {
    margin-top: 14px;
  }

  .pill.good {
    color: var(--good);
    border-color: var(--good);
  }
</style>
