// The game's end reveals both hands (R434), read off the view and nothing else (CLAUDE.md rule 7).
//
// While a game runs, `SideView.hand` is the privacy boundary (§10.8): the viewer's own hand is full
// cards, the opponent's is `{ count }`. Once the game is over the engine shows the opponent's hand
// too. This is the one place the client reads that: `revealedOpponentHand` accepts the opponent's
// `hand` arriving as cards once the view says the game is over (`phase: "over"`, or a `result`).
// Everything that shows the revealed hand — the board's hand row turning its backs face up
// (Hand.tsx) and the result's "Their hand" (TheirHand.tsx) — asks here.

import type { CardView, PlayerView } from "@jackioh/shared";

/** R434: the opponent's hand as the finished game shows it, or null while it is hidden. */
export function revealedOpponentHand(view: PlayerView): CardView[] | null {
  const over = view.phase === "over" || view.result !== null;
  return over && Array.isArray(view.opponent.hand) ? view.opponent.hand : null;
}

/** The testids of what R434 shows. `e2e/support/testids.ts` may mirror them; nothing clicks them. */
export const revealTestid = {
  /** One of the opponent's cards, face up in their hand row at the game's end. */
  handCard: (instanceId: string): string => `revealed-hand-card-${instanceId}`,
  /** The result's "Their hand" section, with `data-count`. */
  theirHand: "result-their-hand",
  /** One face in it: a button that opens the card large. */
  theirHandCard: (instanceId: string): string => `result-their-hand-${instanceId}`,
  /** The chip's "Their hand" button, which opens the list in a dialog. */
  theirHandOpen: "result-their-hand-open",
} as const;
