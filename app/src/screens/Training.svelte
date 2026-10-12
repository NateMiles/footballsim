<!-- The squad's training schedule, formations being learnt, and each
     player's own training. -->
<script lang="ts">
  import type { IndividualTraining, TrainingFocus, TrainingIntensity, TrainingView } from '../api';
  import Meter from '../components/Meter.svelte';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  const game = useGame();
  const loaded = load(() => game.client.training());
  let view = $state<TrainingView | null>(null);
  $effect(() => {
    if (loaded.data) view = loaded.data;
  });

  async function setSchedule(focus: TrainingFocus, intensity: TrainingIntensity) {
    const v = await game.run('Saving', () => game.client.setTraining(focus, intensity));
    if (v) view = v;
  }

  function encode(t: IndividualTraining | null): string {
    if (!t) return '';
    return t.type === 'position' ? `p:${t.position}` : `a:${t.attribute}`;
  }

  async function setIndividual(player: number, value: string) {
    const training: IndividualTraining | null =
      value === ''
        ? null
        : value.startsWith('p:')
          ? { type: 'position', position: value.slice(2) }
          : { type: 'attribute', attribute: value.slice(2) };
    const v = await game.run('Saving', () => game.client.setIndividualTraining(player, training));
    if (v) view = v;
  }

  const signed = (n: number) => (n > 0 ? `+${n}%` : `${n}%`);
</script>

<div class="screen grid">
  <h1>Training</h1>
  {#if view}
    {@const v = view}
    <div class="cols">
      <section class="card grid">
        <h2>Schedule</h2>
        <div class="focuses" role="radiogroup" aria-label="Focus">
          {#each v.focuses as f (f.focus)}
            <button
              role="radio"
              aria-checked={v.focus === f.focus}
              class="focus"
              class:active={v.focus === f.focus}
              disabled={game.busy !== null}
              onclick={() => setSchedule(f.focus, v.intensity)}
            >
              <strong>{f.name}</strong>
              <span class="dim">{f.description}</span>
            </button>
          {/each}
        </div>
        <div class="row" role="radiogroup" aria-label="Intensity">
          <span class="dim">Intensity</span>
          {#each v.intensities as i (i.intensity)}
            <button
              role="radio"
              aria-checked={v.intensity === i.intensity}
              class:active={v.intensity === i.intensity}
              title={i.description}
              disabled={game.busy !== null}
              onclick={() => setSchedule(v.focus, i.intensity)}>{i.name}</button
            >
          {/each}
        </div>
        <p data-testid="effects">
          Growth <strong>{signed(v.growth)}</strong> · recovery <strong>{signed(v.recovery)}</strong> · training
          injuries <strong>{signed(v.injuryRisk)}</strong>
        </p>
      </section>

      <section class="card grid">
        <h2>Formations</h2>
        <p class="dim">
          The team drills the shape it plays every day (about {v.drillPerWeek} points a week on this schedule); others
          fade. Below 90 a team plays slightly below itself.
        </p>
        {#each v.familiarity as f (f.formation)}
          <div class="fam" class:current={f.current}>
            <span>{f.formation}</span>
            <Meter value={f.familiarity} label="Familiarity with {f.formation}" good={90} ok={60} />
            <span class="num">{f.familiarity}</span>
            {#if f.current}<span class="pill">Playing</span>{/if}
          </div>
        {/each}
      </section>
    </div>

    <section class="card scroll-x">
      <h2>Individual training</h2>
      <p class="dim">Learn a new position (a point or two a month, up to natural at 20), or work on one attribute.</p>
      <table class="data">
        <thead>
          <tr><th>Pos</th><th>Name</th><th class="num">Age</th><th>Fitness</th><th>Own training</th><th class="num">Progress</th></tr>
        </thead>
        <tbody>
          {#each v.players as p (p.player.id)}
            <tr>
              <td class="dim">{p.player.position}</td>
              <td><a href="#/player/{p.player.id}">{p.player.name}</a>{#if p.injured}<span class="pill bad">Out</span>{/if}</td>
              <td class="num">{p.age}</td>
              <td><Meter value={p.fitness} label="Fitness" /></td>
              <td>
                <select
                  aria-label="Own training for {p.player.name}"
                  value={encode(p.individual)}
                  onchange={(e) => setIndividual(p.player.id, e.currentTarget.value)}
                >
                  <option value="">None</option>
                  <optgroup label="Learn a position">
                    {#each v.positions as code (code)}<option value="p:{code}">{code}</option>{/each}
                  </optgroup>
                  <optgroup label="Work on">
                    {#each v.attributes as a (a)}<option value="a:{a}">{a}</option>{/each}
                  </optgroup>
                </select>
              </td>
              <td class="num">{p.progress === null ? '' : `${p.progress}/20`}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>
  {/if}
</div>

<style>
  .cols {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: var(--gap);
  }

  .focuses {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 8px;
  }

  .focus {
    display: grid;
    align-content: start;
    gap: 4px;
    text-align: left;
    padding: 12px 14px;
    border-radius: var(--radius);
    background: var(--bg-sunken);
    border-color: var(--line);
  }

  .focus span {
    font-size: 0.8rem;
    font-weight: 400;
    color: var(--text-dim);
  }

  .focus.active strong,
  .focus.active {
    color: var(--text);
  }

  .focus.active {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .fam {
    display: grid;
    grid-template-columns: 5em 1fr 3em 5em;
    align-items: center;
    gap: 8px;
  }

  .fam.current {
    font-weight: 700;
  }
</style>
