import type { Frame, PlayerFrame, Pose } from '../api';

/** A frame at an arbitrary time, blended from the two nearest frames. */
export interface Sample {
  t: number;
  ball: [number, number, number];
  players: PlayerFrame[];
}

function lerp(a: number, b: number, k: number): number {
  return a + (b - a) * k;
}

function lerpAngle(a: number, b: number, k: number): number {
  let d = b - a;
  while (d > Math.PI) d -= 2 * Math.PI;
  while (d < -Math.PI) d += 2 * Math.PI;
  return a + d * k;
}

/**
 * Frames received from the core, in time order, sampled at any time
 * between them. Frames arrive a few times a second of match time; the
 * screen draws at its refresh rate, so positions are interpolated.
 */
export class Timeline {
  private frames: Frame[] = [];

  /** Adds frames that follow the ones already held. */
  push(frames: Frame[]): void {
    for (const f of frames) {
      if (this.frames.length === 0 || f.t > this.frames[this.frames.length - 1].t) this.frames.push(f);
    }
  }

  /** The time of the last frame held, or -1 when empty. */
  get end(): number {
    return this.frames.length ? this.frames[this.frames.length - 1].t : -1;
  }

  get length(): number {
    return this.frames.length;
  }

  /** Drops frames no longer needed to sample at `t` or later. */
  trim(t: number): void {
    let i = 0;
    while (i + 1 < this.frames.length && this.frames[i + 1].t <= t) i++;
    if (i > 0) this.frames.splice(0, i);
  }

  /** The scene at time `t`, or null before any frame. */
  sample(t: number): Sample | null {
    const fs = this.frames;
    if (fs.length === 0) return null;
    if (t <= fs[0].t) return { t, ball: [...fs[0].ball], players: fs[0].players };
    const last = fs[fs.length - 1];
    if (t >= last.t) return { t, ball: [...last.ball], players: last.players };
    // Binary search for the frame at or before t.
    let lo = 0;
    let hi = fs.length - 1;
    while (hi - lo > 1) {
      const mid = (lo + hi) >> 1;
      if (fs[mid].t <= t) lo = mid;
      else hi = mid;
    }
    const a = fs[lo];
    const b = fs[hi];
    const k = (t - a.t) / (b.t - a.t);
    const next = new Map(b.players.map((p) => [p.id, p]));
    const players = a.players.map((p) => {
      const q = next.get(p.id);
      if (!q) return p;
      const pose: Pose = k < 0.5 ? p.pose : q.pose;
      return {
        id: p.id,
        side: p.side,
        x: lerp(p.x, q.x, k),
        y: lerp(p.y, q.y, k),
        facing: lerpAngle(p.facing, q.facing, k),
        pose,
      };
    });
    return {
      t,
      ball: [lerp(a.ball[0], b.ball[0], k), lerp(a.ball[1], b.ball[1], k), lerp(a.ball[2], b.ball[2], k)],
      players,
    };
  }
}
