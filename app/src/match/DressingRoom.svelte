<!-- The dressing room before kick-off and at half-time: the players on
     the benches, a team talk, and every face as they take it. -->
<script lang="ts">
  import type { LiveLineup, LivePlayerInfo, LiveStart, PlayerReaction, TalkReaction, TeamTalk } from '../api';
  import Crest from '../components/Crest.svelte';
  import Portrait, { type Expression } from '../components/Portrait.svelte';

  let {
    start,
    lineup,
    players,
    reactions,
    phase,
    score,
    ontalk,
    ondone,
  }: {
    start: LiveStart;
    lineup: LiveLineup;
    players: Map<number, LivePlayerInfo>;
    reactions: PlayerReaction[];
    phase: 'pre' | 'half';
    score: [number, number];
    ontalk: (talk: TeamTalk) => Promise<string | null>;
    ondone: () => void;
  } = $props();

  let said = $state<TeamTalk | null>(null);
  let error = $state<string | null>(null);
  let talking = $state(false);

  const mine = $derived(start.side === 'home' ? start.fixture.home : start.fixture.away);
  const lead = $derived(start.side === 'home' ? score[0] - score[1] : score[1] - score[0]);
  const options = $derived(start.teamTalks.filter((t) => (phase === 'pre' ? t.preMatch : t.halfTime)));
  const eleven = $derived(lineup.onPitch.filter((p) => p !== null).map((p) => p!.id));
  const byPlayer = $derived(new Map(reactions.map((r) => [r.player, r.reaction])));

  const FACE: Record<TalkReaction, Expression> = {
    firedUp: 'determined',
    focused: 'happy',
    unmoved: 'neutral',
    nervous: 'sad',
    complacent: 'sceptical',
    sulking: 'annoyed',
  };
  const LABEL: Record<TalkReaction, string> = {
    firedUp: 'Fired up',
    focused: 'Focused',
    unmoved: 'Unmoved',
    nervous: 'Nervous',
    complacent: 'Complacent',
    sulking: 'Sulking',
  };
  const TONE: Record<TalkReaction, string> = {
    firedUp: 'good',
    focused: 'good',
    unmoved: '',
    nervous: 'warn',
    complacent: 'warn',
    sulking: 'bad',
  };

  /** Before a talk: how the score sits with them. */
  const resting = $derived<Expression>(phase === 'pre' ? 'neutral' : lead > 0 ? 'happy' : lead < 0 ? 'sad' : 'neutral');

  const summary = $derived.by(() => {
    const rs = eleven.map((id) => byPlayer.get(id)).filter((r): r is TalkReaction => r !== undefined);
    if (rs.length === 0) return '';
    const up = rs.filter((r) => r === 'firedUp' || r === 'focused').length;
    const down = rs.filter((r) => r === 'nervous' || r === 'complacent' || r === 'sulking').length;
    if (up >= 8) return 'The room is buzzing. They are ready.';
    if (up >= 6) return 'Most of the players responded well.';
    if (down >= 5) return 'That did not land. Several players look worse for it.';
    if (down >= 3) return 'A mixed response: some took it badly.';
    return 'They heard you. Not much has changed.';
  });

  async function talk(t: TeamTalk) {
    talking = true;
    error = await ontalk(t);
    if (!error) said = t;
    talking = false;
  }
</script>

<div class="room" style:--club={mine.colors.primary} data-testid="dressing-room">
  <header>
    <Crest club={mine} size={44} />
    <div>
      <span class="eyebrow">{phase === 'pre' ? 'Before kick-off' : 'Half-time'}</span>
      <h2>The dressing room</h2>
    </div>
    {#if phase === 'half'}
      <span class="score">{score[0]}–{score[1]}</span>
    {/if}
  </header>

  <div class="bench">
    {#each eleven as id (id)}
      {@const p = players.get(id)}
      {@const r = byPlayer.get(id)}
      {#if p}
        <div class="player" class:reacted={r !== undefined}>
          <Portrait
            look={p.player.look}
            size={64}
            ring
            expression={r ? FACE[r] : resting}
            shirt={mine.colors.primary}
            trim={mine.colors.secondary}
          />
          <span class="name">{p.player.shortName}</span>
          {#if r}<span class="pill {TONE[r]}">{LABEL[r]}</span>{/if}
        </div>
      {/if}
    {/each}
  </div>

  {#if said === null}
    <p class="prompt">What do you tell them?</p>
    <div class="talks">
      {#each options as o (o.talk)}
        <button class="talk" disabled={talking} onclick={() => talk(o.talk)}>
          <strong>{o.name}</strong>
          <span>“{o.line}”</span>
        </button>
      {/each}
    </div>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="row">
      <span class="spacer"></span>
      <button class="link" onclick={ondone}>Say nothing</button>
    </div>
  {:else}
    {@const o = start.teamTalks.find((t) => t.talk === said)}
    <blockquote>“{o?.line}”</blockquote>
    <p class="summary" data-testid="talk-summary">{summary}</p>
    <div class="row">
      <span class="spacer"></span>
      <button class="primary" onclick={ondone}>{phase === 'pre' ? 'Out they go' : 'Second half'} ▸</button>
    </div>
  {/if}
</div>

<style>
  .room {
    position: absolute;
    inset: 0;
    z-index: 5;
    display: grid;
    align-content: start;
    gap: 16px;
    padding: 22px;
    overflow-y: auto;
    background:
      radial-gradient(70% 50% at 50% 0%, color-mix(in srgb, var(--club) 35%, transparent), transparent 70%),
      repeating-linear-gradient(90deg, rgba(255, 255, 255, 0.025) 0 2px, transparent 2px 60px),
      rgba(7, 11, 17, 0.96);
    animation: rise 0.3s var(--ease) both;
  }

  header {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  header h2 {
    margin: 2px 0 0;
    font-size: 1.6rem;
    font-stretch: 110%;
  }

  .score {
    margin-left: auto;
    font-family: var(--display);
    font-weight: 900;
    font-size: 2rem;
    font-stretch: 115%;
  }

  .bench {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(92px, 1fr));
    gap: 10px;
    padding: 14px;
    border-radius: var(--radius-lg);
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--line);
  }

  .player {
    display: grid;
    justify-items: center;
    gap: 4px;
    text-align: center;
  }

  .player.reacted {
    animation: rise 0.35s var(--ease) both;
  }

  .name {
    font-size: 0.8rem;
    font-weight: 600;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .prompt {
    margin: 0;
    font-family: var(--display);
    font-weight: 700;
    font-size: 1.1rem;
  }

  .talks {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 10px;
  }

  .talk {
    display: grid;
    align-content: start;
    gap: 4px;
    padding: 12px 14px;
    min-height: 0;
    text-align: left;
    border-radius: var(--radius);
    background: var(--bg-sunken);
    border-color: var(--line);
    font-weight: 400;
  }

  .talk strong {
    color: var(--accent);
    font-family: var(--display);
    font-stretch: 90%;
    letter-spacing: 0.04em;
  }

  .talk span {
    color: var(--text);
    font-size: 0.9rem;
  }

  .talk:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  blockquote {
    margin: 0;
    padding: 12px 16px;
    border-left: 3px solid var(--accent);
    border-radius: 0 var(--radius) var(--radius) 0;
    background: var(--accent-soft);
    font-size: 1.05rem;
  }

  .summary {
    margin: 0;
    font-weight: 600;
  }

  .error {
    color: var(--bad);
  }
</style>
