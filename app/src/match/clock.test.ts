import { describe, expect, it } from 'vitest';
import { clockString } from './clock';

describe('clockString', () => {
  it('counts stoppage time in each half', () => {
    expect(clockString(0, [])).toBe('1');
    expect(clockString(46.5 * 60, [])).toBe('45+2');
    expect(clockString(47 * 60, [47 * 60])).toBe('46');
    expect(clockString(93.5 * 60, [47 * 60])).toBe('90+2');
  });

  it('runs on through extra time', () => {
    const starts = [47 * 60, 95 * 60, 111 * 60];
    expect(clockString(95 * 60, starts)).toBe('91');
    expect(clockString(110.5 * 60, starts)).toBe('105+1');
    expect(clockString(111 * 60, starts)).toBe('106');
    expect(clockString(126.5 * 60, starts)).toBe('120+1');
  });
});
