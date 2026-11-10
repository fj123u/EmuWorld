// Gamepad focus moves between the items of a dialog by where they are on
// screen: the cover picker mixes a button row, a credit link and grids of
// different lengths, where "next index" or a fixed column count goes wrong.

export type Direction = "up" | "down" | "left" | "right";

export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/**
 * Index of the item to focus when moving `dir` from `current` (-1: nothing
 * focused yet → the first item). The nearest item lying in that direction
 * wins, a sideways offset counting double; stays put at an edge.
 */
export function spatialNext(boxes: Box[], current: number, dir: Direction): number {
  if (boxes.length === 0) return -1;
  if (current < 0 || current >= boxes.length) return 0;
  const center = (b: Box) => ({ x: b.left + b.width / 2, y: b.top + b.height / 2 });
  const from = center(boxes[current]);
  let best = current;
  let bestScore = Infinity;
  boxes.forEach((b, i) => {
    if (i === current) return;
    const c = center(b);
    const dx = c.x - from.x;
    const dy = c.y - from.y;
    const [along, across] = dir === "left" ? [-dx, dy] : dir === "right" ? [dx, dy] : dir === "up" ? [-dy, dx] : [dy, dx];
    if (along <= 1) return;
    const score = along + 2 * Math.abs(across);
    if (score < bestScore) {
      bestScore = score;
      best = i;
    }
  });
  return best;
}
