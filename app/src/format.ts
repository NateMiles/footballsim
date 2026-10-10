/** Formatting helpers shared by screens. */

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** `2026-08-15` → `15 Aug`. */
export function shortDate(iso: string): string {
  const [, m, d] = iso.split('-').map(Number);
  return `${d} ${MONTHS[m - 1]}`;
}

/** `2026-08-15` → `15 Aug 2026`. */
export function longDate(iso: string): string {
  const [y] = iso.split('-');
  return `${shortDate(iso)} ${y}`;
}

/** `2026-08-15` → `August 2026`, for grouping. */
export function monthOf(iso: string): string {
  const [y, m] = iso.split('-').map(Number);
  const names = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
  return `${names[m - 1]} ${y}`;
}

/** A weekly wage: `£12,500 p/w`. */
export function wage(w: number): string {
  return `£${w.toLocaleString('en-GB')} p/w`;
}

/** 1st, 2nd, 3rd... */
export function ordinal(n: number): string {
  const s = ['th', 'st', 'nd', 'rd'];
  const v = n % 100;
  return n + (s[(v - 20) % 10] ?? s[v] ?? s[0]);
}
