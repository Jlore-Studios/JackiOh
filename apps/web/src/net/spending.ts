// R2378000: an account runs one spend at a time. Every spend of the tab (`net/api.ts`'s `spendRequest`)
// runs through `whileSpending`; while one is out the spend buttons wait, disabled (`economy/spend.css`),
// so a press the server would only queue or refuse is never sent and a person sees a wait, not an error.
// The server still orders every spend of the account, other tabs' too.

import { useSyncExternalStore } from "react";

let out = 0;

const listeners = new Set<() => void>();

function subscribe(onChange: () => void): () => void {
  listeners.add(onChange);
  return () => {
    listeners.delete(onChange);
  };
}

function notify(): void {
  for (const listener of [...listeners]) listener();
}

/** Runs `spend`, counted out from now until it settles, whichever way. */
export function whileSpending<T>(spend: () => Promise<T>): Promise<T> {
  out += 1;
  notify();
  return spend().finally(() => {
    out -= 1;
    notify();
  });
}

/** Whether one of the tab's spends is out now, for a handler's guard. */
export function spendOutNow(): boolean {
  return out > 0;
}

/** Whether one of the tab's spends is out, for a component to wait on. */
export function useSpendOut(): boolean {
  return useSyncExternalStore(subscribe, spendOutNow, () => false);
}
