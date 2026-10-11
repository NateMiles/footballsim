<!-- Save, load, export and import games. -->
<script lang="ts">
  import Benchmark from '../components/Benchmark.svelte';
  import { longDate } from '../format';
  import { useGame } from '../state/game.svelte';
  import { AUTOSAVE, type SaveSlot } from '../storage/saves';

  const game = useGame();
  let slots = $state<SaveSlot[]>([]);
  let name = $state('');
  let message = $state<string | null>(null);

  async function refresh() {
    slots = await game.saves.list();
  }

  $effect(() => {
    refresh().catch(game.fail);
  });

  async function save() {
    const slot = name.trim() || `${game.overview?.club?.shortName ?? 'Game'} ${game.overview?.date ?? ''}`;
    const s = await game.run('Saving', () => game.client.save());
    if (!s) return;
    await game.saves.put(slot, s);
    name = '';
    message = `Saved as “${slot}”.`;
    await refresh();
  }

  async function loadSlot(slot: SaveSlot) {
    const s = await game.saves.get(slot.name);
    if (!s) return;
    const o = await game.run('Loading', () => game.client.load(s.data));
    if (o) {
      await game.enter(o);
      game.router.go({ name: 'home' });
    }
  }

  async function remove(slot: SaveSlot) {
    await game.saves.remove(slot.name);
    await refresh();
  }

  async function exportSlot(slot: SaveSlot) {
    const s = await game.saves.get(slot.name);
    if (!s) return;
    const blob = new Blob([JSON.stringify(s)], { type: 'application/json' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = `${slot.name.replace(/[^\w-]+/g, '_')}.fsim.json`;
    a.click();
    URL.revokeObjectURL(a.href);
  }

  async function importFile(e: Event & { currentTarget: HTMLInputElement }) {
    const file = e.currentTarget.files?.[0];
    if (!file) return;
    try {
      const save = JSON.parse(await file.text());
      if (typeof save?.data !== 'string' || typeof save?.summary !== 'object') throw new Error('Not a footballsim save.');
      await game.saves.put(file.name.replace(/\.fsim\.json$/, ''), save);
      message = `Imported ${file.name}.`;
      await refresh();
    } catch (err) {
      game.fail(err);
    }
    e.currentTarget.value = '';
  }
</script>

<div class="screen grid">
  <h1>Saves</h1>
  <section class="card row">
    <input type="text" placeholder="Save name" bind:value={name} aria-label="Save name" />
    <button class="primary" onclick={save}>Save game</button>
    <label class="import">
      <input type="file" accept=".json,application/json" onchange={importFile} />
      <span>Import…</span>
    </label>
    <span class="spacer"></span>
    <button onclick={() => game.quitToTitle()}>Main menu</button>
  </section>
  {#if message}<p class="dim">{message}</p>{/if}
  <section class="card">
    {#each slots as slot (slot.name)}
      <div class="slot">
        <div>
          <strong>{slot.name}</strong>
          {#if slot.name === AUTOSAVE}<span class="pill">auto</span>{/if}
          <div class="dim">{slot.summary.club} · {slot.summary.manager} · {longDate(slot.summary.date)}</div>
        </div>
        <span class="spacer"></span>
        <button onclick={() => loadSlot(slot)}>Load</button>
        <button onclick={() => exportSlot(slot)}>Export</button>
        <button class="danger" onclick={() => remove(slot)}>Delete</button>
      </div>
    {:else}
      <p class="dim">No saves yet.</p>
    {/each}
  </section>
  <Benchmark />
</div>

<style>
  .spacer {
    flex: 1;
  }

  .slot {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    padding: 10px 0;
    border-bottom: 1px solid var(--line);
  }

  .import {
    position: relative;
    display: inline-grid;
    place-items: center;
    min-height: var(--tap);
    padding: 0 16px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    cursor: pointer;
  }

  .import input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }
</style>
