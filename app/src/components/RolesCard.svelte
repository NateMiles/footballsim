<!-- Roles and duties for each slot of the formation, and set-piece takers. -->
<script lang="ts">
  import type { Duty, MarkingChoice, PlayerRole, RoleChoice, TacticsView } from '../api';
  import { useGame } from '../state/game.svelte';

  let { view, formationChanged }: { view: TacticsView; formationChanged: boolean } = $props();
  const game = useGame();

  const DUTIES: Duty[] = ['defend', 'support', 'attack'];

  let roles = $state<RoleChoice[]>([]);
  let penalty = $state<number | null>(null);
  let freeKick = $state<number | null>(null);
  let corner = $state<number | null>(null);
  let dirty = $state(false);

  $effect(() => {
    const ins = view.instructions;
    roles = ins.roles.map((r) => ({ role: r.role, duty: r.duty }));
    penalty = ins.penaltyTaker;
    freeKick = ins.freeKickTaker;
    corner = ins.cornerTaker;
    dirty = false;
  });

  const byId = $derived(new Map(view.players.map((p) => [p.id, p])));
  const starters = $derived(view.starters.map((id) => byId.get(id)).filter((p) => p !== undefined));

  function setRole(i: number, role: PlayerRole) {
    roles[i] = { ...roles[i], role };
    dirty = true;
  }

  function setDuty(i: number, duty: Duty) {
    roles[i] = { ...roles[i], duty };
    dirty = true;
  }

  function describe(i: number): string {
    const slot = view.instructions.roles[i];
    return slot.options.find((o) => o.role === roles[i]?.role)?.description ?? '';
  }

  async function save(reset: boolean) {
    const marking: MarkingChoice[] = view.instructions.marking.map((m) => ({
      player: m.player.id,
      marking: m.marking,
    }));
    const r = reset
      ? view.instructions.roles.map((s) => ({ role: s.options[0].role, duty: 'support' as Duty }))
      : roles;
    const v = await game.run('Saving instructions', () =>
      game.client.setInstructions(r, reset ? { penalty: null, freeKick: null, corner: null } : { penalty, freeKick, corner }, marking),
    );
    if (v) game.changed();
  }

  const takerValue = (id: number | null) => (id === null ? '' : String(id));
  const fromValue = (s: string) => (s === '' ? null : Number(s));
</script>

{#snippet taker(label: string, value: number | null, set: (v: number | null) => void)}
  <label class="field">
    <span>{label}</span>
    <select
      value={takerValue(value)}
      onchange={(e) => {
        set(fromValue(e.currentTarget.value));
        dirty = true;
      }}
    >
      <option value="">Best on the pitch</option>
      {#each starters as s (s.id)}<option value={String(s.id)}>{s.name}</option>{/each}
    </select>
  </label>
{/snippet}

<section class="card grid roles">
  <h2>Roles and duties</h2>
  {#if formationChanged}
    <p class="pill warn">Save the new formation first; these roles are for {view.tactic.formation}.</p>
  {/if}
  <div class="slots" data-testid="roles">
    {#each view.instructions.roles as slot, i (i)}
      {@const p = byId.get(view.starters[i])}
      <div class="slot">
        <span class="pos">{slot.position}</span>
        <span class="who">{p?.shortName ?? ''}</span>
        <select
          aria-label="Role for {slot.position}"
          value={roles[i]?.role}
          onchange={(e) => setRole(i, e.currentTarget.value as PlayerRole)}
        >
          {#each slot.options as o (o.role)}<option value={o.role}>{o.name}</option>{/each}
        </select>
        {#if slot.hasDuty}
          <select
            aria-label="Duty for {slot.position}"
            value={roles[i]?.duty}
            onchange={(e) => setDuty(i, e.currentTarget.value as Duty)}
          >
            {#each DUTIES as d (d)}<option value={d}>{d[0].toUpperCase() + d.slice(1)}</option>{/each}
          </select>
        {:else}
          <span></span>
        {/if}
        <span class="dim desc">{describe(i)}</span>
      </div>
    {/each}
  </div>

  <h3>Set pieces</h3>
  <div class="takers">
    {@render taker('Penalties', penalty, (v) => (penalty = v))}
    {@render taker('Free kicks', freeKick, (v) => (freeKick = v))}
    {@render taker('Corners', corner, (v) => (corner = v))}
  </div>
  <p class="dim small">Attacking duties create more at both ends; defensive duties keep the shape. No role wins on its own.</p>
  <div class="row">
    <button class="primary" disabled={!dirty || game.busy !== null} onclick={() => save(false)}>Save instructions</button>
    <button disabled={game.busy !== null} onclick={() => save(true)}>Reset</button>
  </div>
</section>

<style>
  .roles h2,
  .roles h3 {
    margin: 0;
  }

  .slots {
    display: grid;
    gap: 6px;
  }

  .slot {
    display: grid;
    grid-template-columns: 3em 7em minmax(9em, 1fr) 7em;
    gap: 6px;
    align-items: center;
  }

  .desc {
    grid-column: 3 / -1;
    font-size: 0.8rem;
    margin-top: -4px;
  }

  .pos {
    font-weight: 700;
    color: var(--accent);
    font-size: 0.85rem;
  }

  .who {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.9rem;
  }

  .takers {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 8px;
  }

  .small {
    font-size: 0.85rem;
    margin: 0;
  }

  @media (max-width: 600px) {
    .slot {
      grid-template-columns: 3em 1fr;
    }

    .slot select {
      grid-column: 1 / -1;
    }

    .desc {
      grid-column: 1 / -1;
    }
  }
</style>
