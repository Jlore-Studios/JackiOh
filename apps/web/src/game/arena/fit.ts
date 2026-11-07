// Where a corner prop may stay: a pure overlap test, so the layer can hide a prop that would cover a
// tile, a card or a control (CORNER_OBSTACLES) or leave the screen (#3348).

export type Box = { left: number; top: number; right: number; bottom: number };

/** True when the two boxes share some area; boxes that only touch do not. */
export function intersects(a: Box, b: Box): boolean {
  return Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0;
}

const hasArea = (box: Box): boolean => box.right - box.left > 0 && box.bottom - box.top > 0;

/**
 * True when `prop` has no area (no layout yet, as in jsdom), or lies inside `viewport` and meets no
 * obstacle that has area (a hidden one has none).
 */
export function propFits(prop: Box, obstacles: readonly Box[], viewport: { width: number; height: number }): boolean {
  if (!hasArea(prop)) return true;
  if (prop.left < 0 || prop.top < 0 || prop.right > viewport.width || prop.bottom > viewport.height) return false;
  return !obstacles.some((obstacle) => hasArea(obstacle) && intersects(prop, obstacle));
}
