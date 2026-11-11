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
 * focused yet → the first item). Among the items lying in that direction,
 * the closest one counting both how far it is (facing edges) and how far it
 * is to the side (0 when in line), so a page in two columns stays in its
 * column; ties go to the most centered. Left / right stay on the same row.
 * Stays put at an edge. Hidden items (no size) are never targets.
 */
export function spatialNext(boxes: Box[], current: number, dir: Direction): number {
  if (boxes.length === 0) return -1;
  if (current < 0 || current >= boxes.length) return 0;
  const a = boxes[current];
  const vertical = dir === "up" || dir === "down";
  const center = (b: Box) => ({ x: b.left + b.width / 2, y: b.top + b.height / 2 });
  const from = center(a);
  const candidates: { i: number; score: number }[] = [];
  boxes.forEach((b, i) => {
    if (i === current || (b.width <= 0 && b.height <= 0)) return;
    const c = center(b);
    const along = dir === "left" ? from.x - c.x : dir === "right" ? c.x - from.x : dir === "up" ? from.y - c.y : c.y - from.y;
    if (along <= 1) return;
    // Distance between the facing edges, and sideways gap (0 when in line).
    const edge = dir === "down" ? b.top - (a.top + a.height) : dir === "up" ? a.top - (b.top + b.height)
      : dir === "right" ? b.left - (a.left + a.width) : a.left - (b.left + b.width);
    const gap = vertical
      ? Math.max(0, b.left - (a.left + a.width), a.left - (b.left + b.width))
      : Math.max(0, b.top - (a.top + a.height), a.top - (b.top + b.height));
    // Left / right stay on the same row.
    if (!vertical && gap > 0) return;
    candidates.push({ i, score: Math.max(0, edge) + gap + Math.abs(vertical ? c.x - from.x : c.y - from.y) / 1000 });
  });
  if (candidates.length === 0) return current;
  return candidates.reduce((m, c) => (c.score < m.score ? c : m)).i;
}

/**
 * Pages and Big Picture: the item on screen in that direction; left/right at
 * the end of a row (or without any layout) go to the next/previous item in
 * page order, so a row wraps to the next one. Up/down stay put at an edge:
 * the caller decides (top bar ↔ page).
 */
export function moveFocus(boxes: Box[], current: number, dir: Direction): number {
  if (current < 0 || current >= boxes.length) return spatialNext(boxes, current, dir);
  const next = spatialNext(boxes, current, dir);
  if (next !== current || (dir !== "right" && dir !== "left")) return next;
  const step = dir === "right" ? 1 : -1;
  const shown = (b: Box) => b.width > 0 || b.height > 0;
  // No layout at all (tests): plain page order.
  if (!boxes.some(shown)) return Math.max(0, Math.min(current + step, boxes.length - 1));
  for (let i = current + step; i >= 0 && i < boxes.length; i += step) if (shown(boxes[i])) return i;
  return current;
}

/** Where an element is on screen, for spatialNext / moveFocus. */
export function boxOf(el: Element): Box {
  const r = el.getBoundingClientRect();
  return { left: r.left, top: r.top, width: r.width, height: r.height };
}
