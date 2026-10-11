<!-- Backroom staff: who the club has, how good its departments are, and
     who is free to hire. -->
<script lang="ts">
  import type { StaffMember, StaffRole } from '../api';
  import Meter from '../components/Meter.svelte';
  import { shortDate, wage } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  const game = useGame();
  const view = load(() => game.client.staff());
  let role = $state<StaffRole | 'all'>('all');
  let confirm = $state<number | null>(null);

  const ROLES: { role: StaffRole; label: string }[] = [
    { role: 'assistantManager', label: 'Assistant manager' },
    { role: 'coach', label: 'Coach' },
    { role: 'scout', label: 'Scout' },
    { role: 'physio', label: 'Physio' },
  ];
  const label = (r: StaffRole) => ROLES.find((x) => x.role === r)?.label ?? r;
  const roleIndex = (r: StaffRole) => ROLES.findIndex((x) => x.role === r);

  const free = $derived((view.data?.available ?? []).filter((m) => role === 'all' || m.role === role));
  const full = (r: StaffRole) =>
    (view.data?.staff.filter((m) => m.role === r).length ?? 0) >= (view.data?.limits[roleIndex(r)] ?? 0);

  async function hire(m: StaffMember) {
    const r = await game.run('Hiring', () => game.client.hireStaff(m.id, m.demand, 2));
    if (r) game.changed();
  }

  async function renew(m: StaffMember) {
    const r = await game.run('Renewing', () => game.client.hireStaff(m.id, m.demand, 2));
    if (r) game.changed();
  }

  async function release(m: StaffMember) {
    confirm = null;
    const r = await game.run('Releasing', () => game.client.releaseStaff(m.id));
    if (r) game.changed();
  }

  // Ratings as percentages of 20, for meters.
  const pct = (x: number) => Math.round((x / 20) * 100);
</script>

{#if view.data}
  {@const v = view.data}
  <div class="screen grid">
    <h1>Staff</h1>
    <section class="card depts">
      <div><span class="dim">Coaching</span> <Meter value={pct(v.ratings.coaching)} label="Coaching" good={55} ok={40} /> {v.ratings.coaching.toFixed(1)}</div>
      <div><span class="dim">Youth coaching</span> <Meter value={pct(v.ratings.youthCoaching)} label="Youth coaching" good={55} ok={40} /> {v.ratings.youthCoaching.toFixed(1)}</div>
      <div><span class="dim">Scouting</span> <Meter value={pct(v.ratings.scoutingAbility)} label="Scouting" good={55} ok={40} /> {v.ratings.scoutingAbility.toFixed(1)}</div>
      <div><span class="dim">Medical</span> <Meter value={pct(v.ratings.physiotherapy)} label="Medical" good={55} ok={40} /> {v.ratings.physiotherapy.toFixed(1)}</div>
      <p class="dim small">10 is average. Coaches speed up player development, scouts learn about players faster and more accurately, physios cut injuries.</p>
    </section>

    <section class="card scroll-x">
      <h3>Your staff</h3>
      <table class="data">
        <thead><tr><th>Name</th><th>Role</th><th class="num">Rating</th><th class="num">Age</th><th>Wage</th><th>Contract</th><th></th></tr></thead>
        <tbody>
          {#each v.staff as m (m.id)}
            <tr>
              <td>{m.name}</td>
              <td>{label(m.role)}</td>
              <td class="num"><strong>{m.rating}</strong></td>
              <td class="num">{m.age}</td>
              <td>{m.wage !== null ? wage(m.wage) : ''}</td>
              <td class="dim">{m.expires ? `${shortDate(m.expires)} ${m.expires.slice(0, 4)}` : ''}</td>
              <td class="actions">
                <button onclick={() => renew(m)} title="Two more years at {wage(m.demand)}">Renew</button>
                {#if confirm === m.id}
                  <button class="danger" onclick={() => release(m)}>Confirm</button>
                {:else}
                  <button class="danger" onclick={() => (confirm = m.id)}>Release</button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>

    <section class="card scroll-x">
      <div class="row">
        <h3>Available</h3>
        <span class="spacer"></span>
        <select bind:value={role} aria-label="Role">
          <option value="all">All roles</option>
          {#each ROLES as r (r.role)}<option value={r.role}>{r.label}</option>{/each}
        </select>
      </div>
      <table class="data">
        <thead><tr><th>Name</th><th>Role</th><th class="num">Rating</th><th class="num">Age</th><th>Asks</th><th></th></tr></thead>
        <tbody>
          {#each free as m (m.id)}
            <tr class:dim={!m.wouldJoin}>
              <td>{m.name}</td>
              <td>{label(m.role)}</td>
              <td class="num"><strong>{m.rating}</strong></td>
              <td class="num">{m.age}</td>
              <td>{wage(m.demand)}</td>
              <td>
                {#if !m.wouldJoin}
                  <span class="pill">Not interested</span>
                {:else if full(m.role)}
                  <span class="pill">Department full</span>
                {:else}
                  <button class="primary" onclick={() => hire(m)}>Hire</button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>
  </div>
{/if}

<style>
  .depts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 8px 16px;
    align-items: center;
  }

  .depts div {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .depts .dim:first-child {
    min-width: 7.5em;
  }

  .small {
    grid-column: 1 / -1;
    font-size: 0.85rem;
    margin: 4px 0 0;
  }

  .spacer {
    flex: 1;
  }

  .actions {
    display: flex;
    gap: 6px;
  }

  .actions button {
    min-height: 34px;
    padding: 0 10px;
  }
</style>
