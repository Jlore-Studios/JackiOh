// The browser's Back and Forward in a web test (R2127000). jsdom traverses the
// history in queued tasks, so these need real timers.

import { settle } from "./settle.ts";

/**
 * Zero-delay task turns waited after a traversal: jsdom queues two tasks for it, and a step over a
 * spent overlay entry (`net/backLayers.ts`) takes a second traversal.
 */
export const HISTORY_TURNS = 10;

export async function pressBack(): Promise<void> {
  window.history.back();
  await settle({ macrotasks: HISTORY_TURNS });
}

export async function pressForward(): Promise<void> {
  window.history.forward();
  await settle({ macrotasks: HISTORY_TURNS });
}
