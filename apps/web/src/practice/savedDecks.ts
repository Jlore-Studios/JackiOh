// The saved decks a deck picker offers, as both screens that read them settle them: the practice
// route's (`routes/practice.tsx`) and the Assault's (`assault/AssaultPage.tsx`). Apart from
// `PracticeSetup.tsx` so that the Assault takes no component or stylesheet of practice's setup screen.

import type { Account } from "../net/gate.ts";
import type { SavedDecks } from "./PracticeSetup.tsx";

/**
 * What the account offers the deck picker, from the account and the decks last read for `token`
 * (`loaded`, null until a read answers): the practice route's and the Assault's. `retry` reads them again.
 */
export function savedDecksFor(
  account: Account,
  token: string | null,
  loaded: { token: string; saved: SavedDecks } | null,
  retry: () => void,
): SavedDecks {
  switch (account.kind) {
    case "loading":
      return { kind: "checking" };
    case "anonymous":
      return { kind: "anonymous" };
    case "error":
      return account.retry === undefined
        ? { kind: "unavailable" }
        : { kind: "unavailable", retry: account.retry, retryAfterMs: account.retryAfterMs };
    case "ready":
      if (token === null) return { kind: "inactive" };
      if (loaded === null || loaded.token !== token) return { kind: "checking" };
      return loaded.saved.kind === "unavailable" ? { ...loaded.saved, retry } : loaded.saved;
  }
}
