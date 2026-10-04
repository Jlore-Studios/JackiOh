// R654: a Unit's `attack` hook, played as the viewer picks it up to attack, as in Hearthstone. `Game`
// calls the function this returns when a drag lifts one of the viewer's Units to attack
// (`DragLayer`'s `onLift`) and when a click chooses one as the attacker. Both only ever lift a Unit
// that `legalActions` lets attack (`planDrag`, `onClickTarget`), the source the green glow reads.
// Nothing plays on the drop, on a cancel or when the attack lands, and nothing is sent anywhere:
// this is a client cue, not a game event (CLAUDE.md rule 7).
//
// It reads the viewer's own units in the view the board shows, and nothing else (R203). The
// opponent's Units cannot be picked up, so they never sound here. It never throws.

import { useCallback, useRef } from "react";

import type { PlayerView } from "@jackioh/shared";

import { getAudioEngine } from "./engine.ts";

export function usePickupSound(view: PlayerView): (attackerId: string) => void {
  const viewRef = useRef(view);
  viewRef.current = view;
  return useCallback((attackerId: string) => {
    try {
      const unit = viewRef.current.you.units.find((u) => u !== null && u.instanceId === attackerId);
      if (unit != null) getAudioEngine().playPickup(unit.defId);
    } catch {
      // Sound is never a rule: a failure drops the cue and keeps the drag.
    }
  }, []);
}
