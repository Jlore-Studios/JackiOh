// R2378000: an account runs one spend at a time. Every spend of the tab (`net/api.ts`'s
// `spendRequest`) runs through `whileSpending`, so the tab knows from a press until its answer or
// its failure that one of its spends is out. While one is, the Shop's Buy and the pack panel's
// Open, Open all, sealed pack, Reveal all and Next pack wait, disabled, Buy, Open and Open all
// turning a loading wheel (`economy/spend.css`), so a press the server would only queue or refuse is
// never sent and a person sees a wait, not an error. The server still orders every spend of the
// account, other tabs' too.

import { useSyncExternalStore } from "react";

/** How many of the tab's spends are out. */
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
