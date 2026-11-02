// Log a game as it is played (SPEC R639), beside `useGameAudio` in `Game`: what the player was shown
// (track.ts) goes into the device's totals (store.ts) once, when it is over, and only for a game the caller
// says is the player's own (online and practice, not a tutorial, hotseat game or test fixture).
// Nothing it does reaches the board.

import { useEffect, useRef } from "react";
import type { PlayerView } from "@jackioh/shared";

import { newEventsSince } from "../game/animations.ts";
import { EMPTY_LOG, type GameLog } from "./model.ts";
import { recordGame } from "./store.ts";
import { syncPlayerStats } from "./sync.ts";
import { observe, outcomeOfView } from "./track.ts";

export function useGameStats(view: PlayerView, enabled: boolean): void {
  const log = useRef<GameLog>(EMPTY_LOG);
  const previous = useRef<PlayerView | null>(null);
  const recorded = useRef(false);

  useEffect(() => {
    if (!enabled || previous.current === view) return;
    const before = previous.current;
    previous.current = view;

    const outcome = outcomeOfView(view);
    if (outcome === null) {
      // A new game on the same board: the last was logged, so start a fresh log.
      if (recorded.current) {
        recorded.current = false;
        log.current = EMPTY_LOG;
      }
    }
    // `view.events` is a sliding window (SPEC §10.8), so only what is new is counted; a seat's first view
    // has no earlier one to compare.
    const fresh = before === null || before.viewer !== view.viewer ? [] : newEventsSince(before.events, view.events);
    log.current = observe(log.current, view, fresh, before);

    if (outcome !== null && !recorded.current) {
      recorded.current = true;
      recordGame(log.current, outcome);
      void syncPlayerStats();
    }
  }, [view, enabled]);
}
