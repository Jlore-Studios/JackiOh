// Log a game as it is played (SPEC R639). `Game` calls this beside `useGameAudio`: it watches the
// views the board is handed, collects what the player was shown (track.ts) and adds the game to the
// device's totals (store.ts) once, when the view says it is over.
//
// It only counts a game the caller says is the player's own: online matches and practice games, not a
// tutorial lesson, a hotseat game on one screen or a test fixture. Nothing it does reaches the board.

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
      // A new game on the same board: the last one was logged, so start a log of its own.
      if (recorded.current) {
        recorded.current = false;
        log.current = EMPTY_LOG;
      }
    }
    // `view.events` is a sliding window over the match (SPEC §10.8), so only what is new is counted,
    // as the board's animation queue does; the first view of a seat has no earlier one to compare.
    const fresh = before === null || before.viewer !== view.viewer ? [] : newEventsSince(before.events, view.events);
    log.current = observe(log.current, view, fresh);

    if (outcome !== null && !recorded.current) {
      recorded.current = true;
      recordGame(log.current, outcome);
      void syncPlayerStats();
    }
  }, [view, enabled]);
}
