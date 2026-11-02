// #258: which Units wear the "can't act yet" cue (spent.css), from the view alone.
//
// `UnitView.canAct` is false for every unit of the side whose turn it is not, and for every unit
// outside the main phase or while a prompt is open, so read bare it would grey out the whole board
// half the time. The cue is drawn only for a unit whose controller is the player acting now: there
// `canAct` false means the unit has no exertion left this turn (§4.1, R49). It is drawn state, never
// permission (CLAUDE.md rule 7): what a unit may do is `legalActions`', shown by the green glow.

import { createContext, createElement, useContext, type ReactElement, type ReactNode } from "react";

import type { PlayerId, PlayerView, UnitView } from "@jackioh/shared";

/** The player whose units may be acting now, or null when no unit's `canAct` says anything. */
export function actingSeat(view: Pick<PlayerView, "phase" | "pending" | "result" | "active">): PlayerId | null {
  if (view.result !== null || view.phase !== "main" || view.pending !== null) return null;
  return view.active;
}

const ActingContext = createContext<PlayerId | null>(null);

export function ActingProvider({ view, children }: { view: PlayerView; children?: ReactNode }): ReactElement {
  return createElement(ActingContext.Provider, { value: actingSeat(view) }, children);
}

export function isSpent(unit: Pick<UnitView, "canAct" | "controller">, acting: PlayerId | null): boolean {
  return acting !== null && unit.controller === acting && !unit.canAct;
}

export function useActingSeat(): PlayerId | null {
  return useContext(ActingContext);
}
