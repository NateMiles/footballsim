/** The match clock, as the core formats it. */

const BASE = [0, 45, 90, 105];
const CAP = [45, 90, 105, 120];

/**
 * A match clock string, as the core formats it: "12", "45+2", "105+1".
 * `halfStarts` are the times at which each half after the first began.
 */
export function clockString(t: number, halfStarts: number[]): string {
  const half = Math.min(3, halfStarts.filter((s) => t >= s).length);
  const start = half === 0 ? 0 : halfStarts[half - 1];
  const secs = t - start + BASE[half] * 60;
  const minute = Math.floor(Math.max(0, secs) / 60) + 1;
  const cap = CAP[half];
  return minute > cap ? `${cap}+${minute - cap}` : `${minute}`;
}
