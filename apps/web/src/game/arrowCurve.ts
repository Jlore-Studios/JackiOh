// The curve both board arrows draw: the player's own drag arrow (`drag/DragLayer.tsx`) and the
// opponent's aim (`aim/OpponentAim.tsx`), a quadratic bowed upward on screen whichever way it points.
// Each passes its own bend: a `share` of its length, at most `maxPx`.

type Point = { x: number; y: number };

/** The arrow's straight length (1 when its ends meet), its control point and the unit tangent at its tip. */
export function arrowCurve(
  from: Point,
  to: Point,
  share: number,
  maxPx: number,
): { length: number; cx: number; cy: number; ux: number; uy: number } {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const length = Math.hypot(dx, dy) || 1;
  const bend = Math.min(maxPx, length * share);
  // Bow the shaft upward on screen, whichever way it points.
  let nx = -dy / length;
  let ny = dx / length;
  if (ny > 0) {
    nx = -nx;
    ny = -ny;
  }
  const cx = (from.x + to.x) / 2 + nx * bend;
  const cy = (from.y + to.y) / 2 + ny * bend;
  const tx = to.x - cx;
  const ty = to.y - cy;
  const tangent = Math.hypot(tx, ty) || 1;
  return { length, cx, cy, ux: tx / tangent, uy: ty / tangent };
}
