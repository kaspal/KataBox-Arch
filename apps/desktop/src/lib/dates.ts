// Dates the UI owns: Rust doesn't know the user's time zone.

/** The user's local date, YYYY-MM-DD (streaks count local days). */
export function localDay(d = new Date()): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}
