// R2739001: the option of a card picker under a seat's pointer or keyboard focus, both ways. The chooser's
// picker (Prompt.tsx `CardChoice`) sends its index as it changes; the opponent's board outlines that back
// (OpponentChoice.tsx). Cosmetic, as R738's aim is: never an action, never in the view, and nothing here
// decides a rule (CLAUDE.md rule 7). The server coalesces the sends (`AIM_RELAY_INTERVAL_MS`) and drops an
// index past the count the opponent is shown (R2739000), so the client never asks whether a count shows.

import { useCallback, useEffect, useRef } from "react";

/** What Game and Prompt need of a route that relays choice hovers: `emit` sends this seat's, `opponent` is theirs as last relayed. */
export type ChoiceHover = { emit: (index: number | null) => void; opponent: number | null };

/**
 * R2739001: a stable sender of this seat's hovered option that sends only a change, and `null` on unmount if
 * an index is up. Does nothing when `emit` is undefined (no relay: hotseat, practice, a replay, a play's own picker).
 */
export function useChoiceHoverEmitter(emit: ((index: number | null) => void) | undefined): (index: number | null) => void {
  const sent = useRef<number | null>(null);
  const live = useRef(emit);
  live.current = emit;

  useEffect(
    () => () => {
      if (sent.current !== null) live.current?.(null);
    },
    [],
  );

  return useCallback((index: number | null) => {
    const out = live.current;
    if (out === undefined || sent.current === index) return;
    sent.current = index;
    out(index);
  }, []);
}
