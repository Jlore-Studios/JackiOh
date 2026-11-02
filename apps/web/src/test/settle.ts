// The web tests' one wait for the page to settle (#2215); helpers.test.ts fails a test file that
// defines its own `settle`.

import { act } from "@testing-library/react";
import { vi } from "vitest";

/** Default microtask turns: enough for a stubbed round trip to reach React's state. */
export const SETTLE_TICKS = 50;

export type SettleOptions = {
  /** Microtask turns flushed after the timers. */
  ticks?: number;
  /** Zero-delay `setTimeout` turns waited first, for work a real timer queued. */
  macrotasks?: number;
  /** Fake timers: ms to advance the clock; 0 runs what is due now. */
  advance?: number;
  /** Fake timers: rounds of running every pending timer, each followed by the ticks. */
  runAllTimers?: number;
  /** Wrap the wait in React's `act`; false for a test that renders nothing. */
  act?: boolean;
};

/** Let what the last step queued run: the macrotasks, then the fake clock, then the ticks. */
export async function settle({ ticks = SETTLE_TICKS, macrotasks = 0, advance, runAllTimers = 0, act: inAct = true }: SettleOptions = {}): Promise<void> {
  const wait = async (): Promise<void> => {
    for (let turn = 0; turn < macrotasks; turn += 1) {
      await new Promise<void>((resolve) => {
        setTimeout(resolve, 0);
      });
    }
    if (advance !== undefined) await vi.advanceTimersByTimeAsync(advance);
    for (let round = 0; round < Math.max(runAllTimers, 1); round += 1) {
      if (runAllTimers > 0) await vi.runAllTimersAsync();
      for (let tick = 0; tick < ticks; tick += 1) await Promise.resolve();
    }
  };
  if (inAct) await act(wait);
  else await wait();
}
