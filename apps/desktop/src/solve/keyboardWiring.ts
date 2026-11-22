// Keyboard wiring (select a card, press C, pick a target with the arrows or Tab, Enter):
// choosing the next target card. Pure, so it is unit-tested.

export type Placed = { id: string; x: number; y: number };
export type Direction = "up" | "down" | "left" | "right";

const DIRS: Record<Direction, { x: number; y: number }> = {
  up: { x: 0, y: -1 },
  down: { x: 0, y: 1 },
  left: { x: -1, y: 0 },
  right: { x: 1, y: 0 },
};

/** The nearest card from `at` in `dir` (centres), skipping `exclude`. Only cards inside a
 * cone around the direction count (sideways offset at most twice the distance ahead),
 * and cards straight ahead win over cards off to the side. */
export function nearestInDirection(cards: Placed[], at: Placed, dir: Direction, exclude: string[] = []): string | null {
  const d = DIRS[dir];
  let best: { id: string; score: number } | null = null;
  for (const c of cards) {
    if (c.id === at.id || exclude.includes(c.id)) continue;
    const dx = c.x - at.x;
    const dy = c.y - at.y;
    const along = dx * d.x + dy * d.y;
    const across = Math.abs(dx * d.y - dy * d.x);
    if (along <= 0 || across > 2 * along) continue;
    const score = along + 2 * across;
    if (!best || score < best.score) best = { id: c.id, score };
  }
  return best?.id ?? null;
}

/** The next (or previous) card in reading order (top to bottom, then left to right). */
export function cycle(cards: Placed[], current: string | null, step: 1 | -1, exclude: string[] = []): string | null {
  const order = cards
    .filter((c) => !exclude.includes(c.id))
    .sort((a, b) => (Math.abs(a.y - b.y) > 24 ? a.y - b.y : a.x - b.x));
  if (order.length === 0) return null;
  const i = current ? order.findIndex((c) => c.id === current) : -1;
  if (i < 0) return step === 1 ? order[0].id : order[order.length - 1].id;
  return order[(i + step + order.length) % order.length].id;
}
