import { describe, expect, it } from 'vitest';
import type { Frame } from '../api';
import { Timeline } from './timeline';

const frame = (t: number, x: number): Frame => ({
  t,
  ball: [x, 34, 0],
  players: [{ id: 1, side: 'home', x, y: 10, facing: 0, pose: 'running' }],
});

describe('Timeline', () => {
  it('interpolates between frames', () => {
    const tl = new Timeline();
    tl.push([frame(0, 0), frame(1, 10)]);
    const s = tl.sample(0.25)!;
    expect(s.ball[0]).toBeCloseTo(2.5);
    expect(s.players[0].x).toBeCloseTo(2.5);
  });

  it('holds the ends and ignores frames out of order', () => {
    const tl = new Timeline();
    expect(tl.sample(3)).toBeNull();
    tl.push([frame(1, 5), frame(2, 6), frame(1.5, 99)]);
    expect(tl.length).toBe(2);
    expect(tl.sample(0)!.ball[0]).toBe(5);
    expect(tl.sample(9)!.ball[0]).toBe(6);
    expect(tl.end).toBe(2);
  });

  it('trims frames that are no longer needed', () => {
    const tl = new Timeline();
    tl.push([frame(0, 0), frame(1, 1), frame(2, 2), frame(3, 3)]);
    tl.trim(2.5);
    expect(tl.length).toBe(2);
    expect(tl.sample(2.5)!.ball[0]).toBeCloseTo(2.5);
  });
});
