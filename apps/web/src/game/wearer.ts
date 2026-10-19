// R1295000: whose cards wear the device's card cosmetics on this board (cards/cosmetics.ts). A
// board the viewer plays (`wearCosmetics`: a match, a practice game, a hotseat game, a co-op seat's
// own duel) names the viewer, and only that seat's own cards wear them; a replay, a watched game, a
// teammate's duel and a bare board name nobody, and every card there draws the defaults. Drawn
// state only (CLAUDE.md rule 7): nothing here reaches the engine or the view.

import { createContext, createElement, useContext, type ReactElement, type ReactNode } from "react";

import type { PlayerId } from "@jackioh/shared";

const WearerContext = createContext<PlayerId | null>(null);

export function WearerProvider({ seat, children }: { seat: PlayerId | null; children?: ReactNode }): ReactElement {
  return createElement(WearerContext.Provider, { value: seat }, children);
}

/** The seat whose own cards wear the device's cosmetics, or null when no card does. */
export function useWearerSeat(): PlayerId | null {
  return useContext(WearerContext);
}
