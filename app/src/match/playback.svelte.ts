import type { Client, Commentary, Instruction, LiveChunk, LiveStart } from '../api';
import { Timeline, type Sample } from './timeline';

/** Match seconds of frames kept ready ahead of the clock. */
const LOOKAHEAD = 12;
/** Match seconds asked for per request. */
const CHUNK = 12;

/** A match clock string, as the core formats it: "12", "45+2". */
export function clockString(t: number, firstHalfLength: number | null): string {
  const first = firstHalfLength === null || t < firstHalfLength;
  const secs = first ? t : t - firstHalfLength! + 45 * 60;
  const cap = first ? 45 : 90;
  const minute = Math.floor(Math.max(0, secs) / 60) + 1;
  return minute > cap ? `${cap}+${minute - cap}` : `${minute}`;
}

/**
 * Plays a live match on a clock: fetches chunks from the core ahead of
 * the clock, reveals commentary and goals as the clock reaches them, and
 * gives the renderer a sample per screen frame.
 *
 * The core runs a little ahead of what is shown (up to `LOOKAHEAD` match
 * seconds), so an instruction takes effect that much after the moment on
 * screen. The screen pauses while the manager makes changes.
 */
export class Playback {
  readonly timeline = new Timeline();
  /** Match seconds shown. */
  t = $state(0);
  speed = $state(4);
  playing = $state(true);
  /** The clock caught up with the core. */
  stalled = $state(false);
  latest = $state<LiveChunk | null>(null);
  commentary = $state<Commentary[]>([]);
  score = $state<[number, number]>([0, 0]);
  clock = $state('1');
  /** The core has played the final whistle. */
  engineDone = $state(false);
  /** The clock has reached the final whistle. */
  over = $state(false);
  error = $state<string | null>(null);

  private queued: Commentary[] = [];
  private finalT = Infinity;
  private firstHalf: number | null = null;
  private fetching: Promise<void> | null = null;

  constructor(
    private readonly client: Client,
    readonly start: LiveStart,
  ) {
    void this.fetch();
  }

  /** Advances the clock by `dt` real seconds and samples the scene. */
  step(dt: number): Sample | null {
    const end = this.timeline.end;
    if (this.playing && end >= 0) {
      const target = Math.min(this.t + dt * this.speed, end, this.finalT);
      this.stalled = target >= end && !this.engineDone;
      this.t = Math.max(this.t, target);
    }
    if (!this.engineDone && this.timeline.end - this.t < LOOKAHEAD) void this.fetch();
    this.reveal();
    this.timeline.trim(this.t - 1);
    if (this.engineDone && this.t >= this.finalT) this.over = true;
    return this.timeline.sample(this.t);
  }

  /** Reveals the commentary (and goals) the clock has reached. */
  private reveal(): void {
    let i = 0;
    while (i < this.queued.length && this.queued[i].t <= this.t) i++;
    if (i === 0) {
      this.clock = clockString(this.t, this.firstHalf);
      return;
    }
    const shown = this.queued.splice(0, i);
    for (const line of shown) {
      if (line.kind === 'goal' && line.side) {
        const s = line.side === 'home' ? 0 : 1;
        this.score = s === 0 ? [this.score[0] + 1, this.score[1]] : [this.score[0], this.score[1] + 1];
      }
      if (line.kind === 'whistle' && line.text.startsWith('Half')) this.firstHalf = line.t;
    }
    this.commentary = [...shown.reverse(), ...this.commentary];
    this.clock = clockString(this.t, this.firstHalf);
  }

  private absorb(chunk: LiveChunk): void {
    this.timeline.push(chunk.frames);
    this.queued.push(...chunk.commentary);
    this.queued.sort((a, b) => a.t - b.t);
    this.latest = chunk;
    if (chunk.finished) {
      this.engineDone = true;
      this.finalT = chunk.elapsed;
    }
  }

  private fetch(): Promise<void> {
    this.fetching ??= (async () => {
      try {
        const until = Math.max(this.timeline.end, this.t) + CHUNK;
        this.absorb(await this.client.liveTick(until));
      } catch (err) {
        this.error = err instanceof Error ? err.message : String(err);
        this.playing = false;
      } finally {
        this.fetching = null;
      }
    })();
    return this.fetching;
  }

  /** Gives an instruction; resolves with an error message if refused. */
  async instruct(instruction: Instruction): Promise<string | null> {
    await this.fetching;
    try {
      this.absorb(await this.client.instruct(instruction));
      return null;
    } catch (err) {
      return err instanceof Error ? err.message : String(err);
    }
  }
}
