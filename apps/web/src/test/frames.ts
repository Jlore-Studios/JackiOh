// #3610: a `requestAnimationFrame` a test holds until it says when a frame runs, for what handles a pointer once a frame
// (cards/pointerFrame.ts). Call `manualFrames()` in `beforeEach`, after any `vi.useFakeTimers()`, and `restore()` in
// `afterEach` before `vi.useRealTimers()`.

import { act } from "@testing-library/react";
import { vi } from "vitest";

export type ManualFrames = {
  /** Frames asked for and neither run nor cancelled yet. */
  pending(): number;
  /** Every `requestAnimationFrame` since the helper was installed. */
  requested(): number;
  /** Every `cancelAnimationFrame` since the helper was installed. */
  cancelled(): number;
  /** Runs every frame waiting now, in `act`; one asked for during the flush waits for the next. */
  flush(): void;
  /** Puts the window's own `requestAnimationFrame` and `cancelAnimationFrame` back. */
  restore(): void;
};

export function manualFrames(): ManualFrames {
  const queue = new Map<number, FrameRequestCallback>();
  let next = 1;
  let requested = 0;
  let cancelled = 0;
  const request = vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => {
    const handle = next;
    next += 1;
    requested += 1;
    queue.set(handle, callback);
    return handle;
  });
  const cancel = vi.spyOn(window, "cancelAnimationFrame").mockImplementation((handle) => {
    cancelled += 1;
    queue.delete(handle);
  });
  return {
    pending: () => queue.size,
    requested: () => requested,
    cancelled: () => cancelled,
    flush() {
      const waiting = [...queue.values()];
      queue.clear();
      act(() => {
        for (const callback of waiting) callback(0);
      });
    },
    restore() {
      request.mockRestore();
      cancel.mockRestore();
    },
  };
}
