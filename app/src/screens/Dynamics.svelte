<!-- The dressing room: the captain and leaders, everyone's mood and the
     reason for it, squad status, open promises and transfer requests. -->
<script lang="ts">
  import type { DynamicsView, SquadStatus } from '../api';
  import Portrait from '../components/Portrait.svelte';
  import { longDate } from '../format';
  import { useGame } from '../state/game.svelte';
  import { load } from '../state/load.svelte';

  const game = useGame();
  const loaded = load(() => game.client.dynamics());
  let view = $state<DynamicsView | null>(null);
  $effect(() => {
    if (loaded.data) view = loaded.data;
  });

  async function act(f: () => Promise<DynamicsView>) {
    const v = await game.run('Saving', f);
    if (v) view = v;
  }

  const moodClass = (h: number) => (h >= 15 ? 'good' : h <= -15 ? 'bad' : h < 0 ? 'warn' : '');
  const signed = (n: number) => (n > 0 ? `+${n}` : `${n}`);
</script>

<div class="screen grid">
  <h1>Dressing room</h1>
  {#if view}
    {@const v = view}
    {@const pending = v.rows.filter((r) => r.request && !r.request.accepted)}
    <div class="cols">
      <section class="card grid">
        <div class="row">
          <h2>Atmosphere</h2>
          <span class="spacer"></span>
          <strong class="pill {moodClass(v.happiness)}" data-testid="atmosphere">{v.atmosphere}</strong>
        </div>
        <label class="field">
          <span>Captain</span>
          <select
            value={v.captain?.id ?? ''}
            disabled={game.busy !== null}
            onchange={(e) => act(() => game.client.setCaptain(Number(e.currentTarget.value)))}
          >
            {#if !v.captain}<option value="">None</option>{/if}
            {#each v.rows.filter((r) => r.age >= 18) as r (r.player.id)}
              <option value={r.player.id}>{r.player.name}</option>
            {/each}
          </select>
        </label>
        <p>
          <span class="dim">Leaders:</span>
          {#each v.leaders as l, i (l.id)}{i > 0 ? ', ' : ''}<a href="#/player/{l.id}">{l.name}</a>{/each}
        </p>
        <p class="dim small">
          The leaders' mood spreads to the rest of the squad. Happiness sets where morale settles between matches.
        </p>
      </section>

      <section class="card grid">
        <h2>Requests and promises</h2>
        {#each pending as r (r.player.id)}
          <div class="row request">
            <span><a href="#/player/{r.player.id}">{r.player.name}</a> asks to leave</span>
            <span class="spacer"></span>
            <button disabled={game.busy !== null} onclick={() => act(() => game.client.answerTransferRequest(r.player.id, true))}>
              List for sale
            </button>
            <button disabled={game.busy !== null} onclick={() => act(() => game.client.answerTransferRequest(r.player.id, false))}>
              Refuse
            </button>
          </div>
        {/each}
        {#each v.promises as p (p.player.id + p.kind)}
          <div class="row">
            <span><a href="#/player/{p.player.id}">{p.player.name}</a>: {p.text}</span>
            <span class="spacer"></span>
            <span class="pill {p.onTrack ? 'good' : 'warn'}">{p.onTrack ? 'On track' : 'Behind'}</span>
            <span class="dim">by {longDate(p.due)}</span>
          </div>
        {/each}
        {#if pending.length === 0 && v.promises.length === 0}
          <p class="dim">No transfer requests and no promises to keep.</p>
        {/if}
      </section>
    </div>

    <section class="card">
      <div class="scroll">
        <table class="data" data-testid="moods">
          <thead>
            <tr>
              <th>Player</th>
              <th class="num">Age</th>
              <th>Character</th>
              <th>Status</th>
              <th>Playing time</th>
              <th>Mood</th>
              <th>Main reason</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {#each v.rows as r (r.player.id)}
              <tr>
                <td>
                  <span class="dim pos">{r.player.position}</span>
                  <Portrait
                    look={r.player.look}
                    size={30}
                    expression={r.happiness >= 30 ? 'happy' : r.happiness <= -30 ? 'angry' : r.happiness <= -12 ? 'annoyed' : 'neutral'}
                  />
                  <a href="#/player/{r.player.id}">{r.player.name}</a>
                  {#if r.captain}<span class="pill" title="Captain">C</span>{/if}
                  {#if r.influential}<span class="pill" title="Influential">★</span>{/if}
                  {#if r.request}<span class="pill bad">{r.request.accepted ? 'Listed at own request' : 'Wants to leave'}</span>{/if}
                </td>
                <td class="num">{r.age}</td>
                <td class="dim">{r.personality}</td>
                <td>
                  <select
                    value={r.status}
                    aria-label="Status of {r.player.name}"
                    disabled={game.busy !== null}
                    onchange={(e) =>
                      act(() => game.client.setSquadStatus(r.player.id, e.currentTarget.value as SquadStatus))}
                  >
                    {#each v.statuses as s (s.status)}<option value={s.status}>{s.name}</option>{/each}
                  </select>
                </td>
                <td>
                  <span class="share" title="{r.playingTime}% of minutes; the status expects {r.expected}%">
                    <span class="bar"><span style="width: {r.playingTime}%"></span><i style="left: {r.expected}%"></i></span>
                    <span class="dim">{r.playingTime}%</span>
                  </span>
                </td>
                <td><span class="pill {moodClass(r.happiness)}">{r.label}</span></td>
                <td class="dim">
                  {#if r.reason}{r.reason.text} <small>({signed(r.reason.effect)})</small>{/if}
                </td>
                <td><a class="talk" href="#/talk/{r.player.id}" aria-label="Talk to {r.player.name}">Talk</a></td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>
  {/if}
</div>

<style>
  .talk {
    display: inline-flex;
    align-items: center;
    padding: 4px 12px;
    border-radius: 999px;
    border: 1px solid var(--line-strong);
    color: var(--text);
    font-weight: 600;
    font-size: 0.85rem;
  }

  .talk:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .cols {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: var(--gap);
  }

  h2 {
    margin: 0;
  }

  .spacer {
    flex: 1;
  }

  .small {
    font-size: 0.85rem;
  }

  .request {
    flex-wrap: wrap;
  }

  .scroll {
    overflow-x: auto;
  }

  .pos {
    display: inline-block;
    width: 3em;
  }

  .share {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .bar {
    position: relative;
    display: inline-block;
    width: 80px;
    height: 8px;
    border-radius: 4px;
    background: var(--bg-sunken);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
  }

  .bar i {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--text);
  }
</style>
