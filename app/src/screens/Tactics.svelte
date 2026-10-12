<!-- Formation, team instructions and team selection. -->
<script lang="ts">
  import type { Formation, SelectablePlayer, Tactic, TacticsView } from '../api';
  import FormationPitch from '../components/FormationPitch.svelte';
  import Meter from '../components/Meter.svelte';
  import RolesCard from '../components/RolesCard.svelte';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  const game = useGame();
  const view = load(() => game.client.tactics());

  // Local edits, reset whenever a fresh view arrives.
  let tactic = $state<Tactic | null>(null);
  let starters = $state<number[]>([]);
  let bench = $state<number[]>([]);
  let tacticDirty = $state(false);
  let lineupDirty = $state(false);
  let selected = $state<{ kind: 'slot' | 'bench'; index: number } | null>(null);

  $effect(() => {
    const v = view.data;
    if (!v) return;
    tactic = { ...v.tactic };
    starters = [...v.starters];
    bench = [...v.bench];
    tacticDirty = false;
    lineupDirty = false;
  });

  const v = $derived<TacticsView | null>(view.data);
  const byId = $derived(new Map((v?.players ?? []).map((p) => [p.id, p])));
  const shape = $derived(v && tactic ? v.formations.find((f) => f.formation === tactic!.formation) : undefined);
  const codes = $derived(v?.positions.map((p) => p.code) ?? []);

  /** The position index the selection is for (bench: none). */
  const selectedPosition = $derived(
    selected?.kind === 'slot' && shape ? shape.slots[selected.index].position : null,
  );

  const candidates = $derived.by(() => {
    const list = [...(v?.players ?? [])];
    const pos = selectedPosition;
    if (pos !== null) list.sort((a, b) => b.suitability[pos] - a.suitability[pos]);
    else list.sort((a, b) => Math.max(...b.suitability) - Math.max(...a.suitability));
    return list;
  });

  function role(id: number): string {
    const s = starters.indexOf(id);
    if (s >= 0 && shape) return codes[shape.slots[s].position];
    if (bench.includes(id)) return 'SUB';
    return '';
  }

  function assign(id: number) {
    if (!selected) return;
    const from = starters.includes(id) ? { kind: 'slot' as const, index: starters.indexOf(id) } : bench.includes(id) ? { kind: 'bench' as const, index: bench.indexOf(id) } : null;
    const list = (k: 'slot' | 'bench') => (k === 'slot' ? starters : bench);
    const target = list(selected.kind);
    const displaced = target[selected.index];
    if (from) {
      const src = list(from.kind);
      if (displaced === undefined) src.splice(from.index, 1);
      else src[from.index] = displaced;
    }
    if (selected.index >= target.length) target.push(id);
    else target[selected.index] = id;
    lineupDirty = true;
    selected = null;
  }

  function setFormation(f: Formation) {
    if (!tactic) return;
    tactic.formation = f;
    tacticDirty = true;
  }

  function setSlider(key: string, value: number) {
    if (!tactic) return;
    (tactic as unknown as Record<string, number>)[key] = value;
    tacticDirty = true;
  }

  async function saveTactic(t: Tactic | null) {
    const r = await game.run('Saving tactic', () => game.client.setTactic(t));
    if (r) game.changed();
  }

  async function saveLineup(auto: boolean) {
    const r = await game.run('Saving lineup', () =>
      game.client.setLineup(auto ? null : { starters: [...starters], bench: [...bench] }),
    );
    if (r) game.changed();
  }

  function occupant(id: number | undefined, slot: number | null) {
    if (id === undefined) return null;
    const p: SelectablePlayer | undefined = byId.get(id);
    if (!p) return null;
    const fit = slot !== null && shape ? p.suitability[shape.slots[slot].position] : null;
    return { label: p.shortName, sub: fit !== null ? `${fit}` : undefined, warn: !p.available || (fit !== null && fit < 40) };
  }
</script>

{#if v && tactic && shape}
  <div class="screen tactics">
    <section class="card board">
      <div class="row">
        <h2>Formation</h2>
        <span class="spacer"></span>
        <span class="pill" class:good={v.ownTactic}>{v.ownTactic ? 'Your tactic' : "Assistant's tactic"}</span>
      </div>
      <div class="formations">
        {#each v.formations as f (f.formation)}
          <button class:active={tactic.formation === f.formation} onclick={() => setFormation(f.formation)}>{f.formation}</button>
        {/each}
      </div>
      <FormationPitch
        slots={shape.slots}
        occupants={shape.slots.map((_, i) => occupant(starters[i], i))}
        positions={codes}
        selected={selected?.kind === 'slot' ? selected.index : null}
        onselect={(i) => (selected = selected?.kind === 'slot' && selected.index === i ? null : { kind: 'slot', index: i })}
      />
      <h3 class="gap">Substitutes</h3>
      <div class="bench">
        {#each Array.from({ length: v.benchSize }, (_, i) => i) as i (i)}
          {@const o = occupant(bench[i], null)}
          <button class="benchslot" class:selected={selected?.kind === 'bench' && selected.index === i} onclick={() => (selected = { kind: 'bench', index: Math.min(i, bench.length) })}>
            {o?.label ?? '—'}
          </button>
        {/each}
      </div>
      <div class="row gap">
        <button class="primary" disabled={!lineupDirty} onclick={() => saveLineup(false)}>Save lineup</button>
        <button onclick={() => saveLineup(true)} disabled={!v.ownLineup && !lineupDirty}>Let assistant pick</button>
        <span class="pill" class:good={v.ownLineup}>{v.ownLineup ? 'Your lineup' : "Assistant's lineup"}</span>
      </div>
    </section>

    <section class="card players">
      <h2>{selected ? (selected.kind === 'slot' && selectedPosition !== null ? `Pick a ${codes[selectedPosition]}` : 'Pick a substitute') : 'Players'}</h2>
      <p class="dim hint">{selected ? 'Tap a player to put them here.' : 'Tap a position on the pitch, then a player.'}</p>
      <div class="plist">
        {#each candidates as p (p.id)}
          <button class="prow" class:out={!p.available} disabled={!selected} onclick={() => assign(p.id)}>
            <span class="role">{role(p.id)}</span>
            <span class="pname">{p.name}</span>
            <span class="dim">{p.position}</span>
            <Meter value={p.fitness} label="Fitness" />
            <span class="fit num">{selectedPosition !== null ? p.suitability[selectedPosition] : Math.max(...p.suitability)}</span>
          </button>
        {/each}
      </div>
    </section>

    <section class="card sliders">
      <h2>Instructions</h2>
      {#each v.sliders as s (s.key)}
        {@const val = (tactic as unknown as Record<string, number>)[s.key]}
        <label class="slider">
          <span class="sname">{s.name}</span>
          <span class="dim lo">{s.low}</span>
          <input type="range" min="-2" max="2" step="1" value={val} oninput={(e) => setSlider(s.key, Number(e.currentTarget.value))} />
          <span class="dim hi">{s.high}</span>
        </label>
      {/each}
      <div class="row gap">
        <button class="primary" disabled={!tacticDirty} onclick={() => saveTactic(tactic)}>Save tactic</button>
        <button onclick={() => saveTactic(null)} disabled={!v.ownTactic}>Let assistant choose</button>
      </div>
    </section>

    <div class="rolesarea">
      <RolesCard view={v} formationChanged={tactic.formation !== v.tactic.formation} />
    </div>
  </div>
{/if}

<style>
  .tactics {
    display: grid;
    grid-template-columns: minmax(320px, 480px) 1fr;
    grid-template-areas: 'board players' 'sliders players' 'roles roles';
    gap: var(--gap);
    align-items: start;
  }

  .board {
    grid-area: board;
  }

  .players {
    grid-area: players;
  }

  .sliders {
    grid-area: sliders;
  }

  .rolesarea {
    grid-area: roles;
  }

  .spacer {
    flex: 1;
  }

  .formations {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 8px 0 12px;
  }

  .formations button.active {
    border-color: var(--accent);
    color: var(--accent);
  }

  .gap {
    margin-top: 14px;
  }

  .bench {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
    gap: 6px;
  }

  .benchslot {
    font-size: 0.85rem;
    padding: 0 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .benchslot.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }

  .hint {
    margin: 0 0 8px;
  }

  .plist {
    display: grid;
    gap: 4px;
    max-height: calc(100vh - 220px);
    overflow-y: auto;
  }

  .prow {
    display: grid;
    grid-template-columns: 3em 1fr 3.5em auto 2.5em;
    align-items: center;
    gap: 8px;
    text-align: left;
    padding: 0 12px;
    border-radius: var(--radius);
  }

  .prow:disabled {
    opacity: 1;
  }

  .prow.out .pname {
    color: var(--bad);
  }

  .role {
    font-size: 0.75rem;
    color: var(--accent);
    font-weight: 700;
  }

  .slider {
    display: grid;
    grid-template-columns: 8em 5.5em 1fr 5.5em;
    gap: 8px;
    align-items: center;
    min-height: var(--tap);
  }

  .lo {
    text-align: right;
    font-size: 0.85rem;
  }

  .hi {
    font-size: 0.85rem;
  }

  input[type='range'] {
    accent-color: var(--accent);
    width: 100%;
  }

  @media (max-width: 900px) {
    .tactics {
      grid-template-columns: 1fr;
      grid-template-areas: 'board' 'players' 'sliders' 'roles';
    }

    .slider {
      grid-template-columns: 1fr 1fr;
    }

    .sname {
      grid-column: 1 / -1;
    }

    .lo,
    .hi {
      display: none;
    }

    input[type='range'] {
      grid-column: 1 / -1;
    }
  }
</style>
