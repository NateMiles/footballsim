<!-- The manager's side during a match: substitutions, shouts and
     tactical changes. -->
<script lang="ts">
  import type { Formation, Instruction, LiveLineup, LivePlayerInfo, Shout } from '../api';
  import Meter from '../components/Meter.svelte';

  let {
    lineup,
    players,
    shouts,
    formations,
    oninstruct,
    onselect,
  }: {
    lineup: LiveLineup;
    players: Map<number, LivePlayerInfo>;
    shouts: Shout[];
    formations: Formation[];
    oninstruct: (i: Instruction) => Promise<string | null>;
    onselect?: (id: number | null) => void;
  } = $props();

  let off = $state<number | null>(null);
  let message = $state<string | null>(null);
  const name = (id: number) => players.get(id)?.player.shortName ?? `#${id}`;
  const SHOUTS: Record<Shout, string> = {
    encourage: 'Encourage',
    demandMore: 'Demand more',
    calmDown: 'Calm down',
    tightenUp: 'Tighten up',
    pushForward: 'Push forward',
  };

  async function send(i: Instruction, done: string) {
    message = (await oninstruct(i)) ?? done;
  }

  function pickOff(id: number) {
    off = off === id ? null : id;
    onselect?.(off);
  }

  async function bringOn(on: number) {
    if (off === null) return;
    const o = off;
    off = null;
    onselect?.(null);
    await send({ type: 'substitute', off: o, on }, `${name(on)} replaces ${name(o)}.`);
  }

  async function mentality(m: number) {
    await send({ type: 'changeTactic', tactic: { ...lineup.tactic, mentality: m } }, 'Mentality changed.');
  }

  async function formation(f: Formation) {
    await send({ type: 'changeTactic', tactic: { ...lineup.tactic, formation: f } }, `Switched to ${f}.`);
  }
</script>

<div class="panel">
  <h3>On the pitch · {lineup.substitutionsLeft} subs left</h3>
  <div class="list">
    {#each lineup.onPitch as p, slot (slot)}
      {#if p}
        <button class="p" class:sel={off === p.id} onclick={() => pickOff(p.id)} disabled={lineup.substitutionsLeft === 0}>
          <span class="n">{name(p.id)}</span>
          {#if p.yellow}<span class="card-y" title="Booked"></span>{/if}
          <Meter value={Math.round(p.energy * 100)} label="Energy" />
        </button>
      {/if}
    {/each}
  </div>
  {#if off !== null}
    <h3>Bring on</h3>
    <div class="list">
      {#each lineup.bench as id (id)}
        <button class="p" onclick={() => bringOn(id)}>
          <span class="n">{name(id)}</span>
          <span class="dim">{players.get(id)?.player.position}</span>
        </button>
      {/each}
    </div>
  {/if}

  <h3>Shout</h3>
  <div class="chips">
    {#each shouts as s (s)}
      <button class:active={lineup.shout === s} onclick={() => send({ type: 'shout', shout: s }, `${SHOUTS[s]}!`)}>{SHOUTS[s]}</button>
    {/each}
  </div>

  <h3>Mentality</h3>
  <div class="chips">
    {#each [-2, -1, 0, 1, 2] as m (m)}
      <button class:active={lineup.tactic.mentality === m} onclick={() => mentality(m)}>
        {['Very defensive', 'Defensive', 'Balanced', 'Attacking', 'Very attacking'][m + 2]}
      </button>
    {/each}
  </div>

  <h3>Formation</h3>
  <div class="chips">
    {#each formations as f (f)}
      <button class:active={lineup.tactic.formation === f} onclick={() => formation(f)}>{f}</button>
    {/each}
  </div>

  {#if message}<p class="msg" role="status">{message}</p>{/if}
</div>

<style>
  .panel h3 {
    margin-top: 12px;
  }

  .list {
    display: grid;
    gap: 4px;
  }

  .p {
    display: flex;
    gap: 8px;
    align-items: center;
    justify-content: space-between;
    min-height: 38px;
    padding: 0 10px;
  }

  .p.sel {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }

  .n {
    flex: 1;
    text-align: left;
  }

  .card-y {
    width: 9px;
    height: 12px;
    border-radius: 1px;
    background: #f5d742;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chips button {
    min-height: 36px;
    padding: 0 10px;
    font-size: 0.85rem;
  }

  .chips button.active {
    border-color: var(--accent);
    color: var(--accent);
  }

  .msg {
    color: var(--accent);
  }
</style>
