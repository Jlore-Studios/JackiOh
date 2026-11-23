// #3610 (R3351000, R3340000, R3344001): a pointer follower handles the newest pointermove once an animation frame.
// pointermove fires at the mouse's rate (often 120 to 1000 Hz), not the display's, so a layer that reads a box and writes
// styles on every one reads layout many times a frame. `followPointer` keeps the latest event and handles it in one
// frame; a leave or an unmount `cancel`s the waiting frame, so nothing is written after it. The frame source is never a
// timer (R200): where there is no `requestAnimationFrame` it never fires. Presentation only (CLAUDE.md rule 7): nothing
// here reaches the engine, an action or the log.

import type { FxFrameSource } from "../fx/types.ts";

/** `requestAnimationFrame`, or a source that never fires where there is none (no timers: R200). */
export function pointerFrames(): FxFrameSource {
  if (typeof window.requestAnimationFrame !== "function") {
    return { request: () => 0, cancel: () => undefined };
  }
  return {
    request: (callback) => window.requestAnimationFrame(callback),
    cancel: (handle) => window.cancelAnimationFrame(handle),
  };
}

/** What a follower hands each pointermove to, and how it drops the one waiting. */
export type PointerFollow = {
  /** Keeps `event` as the newest, and asks for a frame if none is waiting. */
  push(event: PointerEvent): void;
  /** Drops the waiting event and its frame; the next `push` asks for a new one. */
  cancel(): void;
};

/** #3610: `handle` runs with the newest pushed event, once in the next frame however many were pushed before it. */
export function followPointer(frames: FxFrameSource, handle: (event: PointerEvent) => void): PointerFollow {
  let latest: PointerEvent | null = null;
  // A handle of 0 is valid (a source that never fires gives it), so a waiting frame is told by `null`, never by truthiness.
  let frame: number | null = null;
  const run = (): void => {
    frame = null;
    const event = latest;
    latest = null;
    if (event !== null) handle(event);
  };
  return {
    push(event) {
      latest = event;
      if (frame === null) frame = frames.request(run);
    },
    cancel() {
      latest = null;
      if (frame === null) return;
      frames.cancel(frame);
      frame = null;
    },
  };
}
