// The drag ghost's tilt (#3347): it leans the way it moves, by its speed, and settles flat once it stops.
// View state only: nothing here is sent or decides a rule (CLAUDE.md rule 7).

/** #3347: the most the dragged ghost leans either way, in degrees. */
export const DRAG_TILT_MAX_DEG = 14;

/** #3347: the pointer speed, in pixels per second, at which the ghost leans its whole `DRAG_TILT_MAX_DEG`. */
export const DRAG_TILT_FULL_SPEED_PX_PER_S = 1800;

/** #3347: how long the pointer may stay still, in milliseconds, before the ghost settles flat. */
export const DRAG_TILT_SETTLE_MS = 90;

/** #3347: how long a card let go over nothing takes to spring back to its place in the hand, in milliseconds. */
export const DRAG_RETURN_MS = 260;

const MS_PER_S = 1000;

/** The lean, in degrees, for a horizontal step of `dxPx` over `dtMs`: right is positive, capped at the maximum, 0 when no time passed. */
export function tiltFor(dxPx: number, dtMs: number): number {
  if (!(dtMs > 0)) return 0;
  const lean = (((dxPx / dtMs) * MS_PER_S) / DRAG_TILT_FULL_SPEED_PX_PER_S) * DRAG_TILT_MAX_DEG;
  return Math.max(-DRAG_TILT_MAX_DEG, Math.min(DRAG_TILT_MAX_DEG, lean));
}

export type TiltTracker = {
  /** The lean for the pointer now at `x`; the first move of a drag has nothing to measure from, so it is 0. */
  move(x: number): number;
};

/** A tracker for one drag, reading its time from `now` (a test's clock, `performance.now` live). */
export function createTiltTracker(now: () => number): TiltTracker {
  let last: { x: number; at: number } | null = null;
  return {
    move(x) {
      const at = now();
      const before = last;
      last = { x, at };
      return before === null ? 0 : tiltFor(x - before.x, at - before.at);
    },
  };
}
