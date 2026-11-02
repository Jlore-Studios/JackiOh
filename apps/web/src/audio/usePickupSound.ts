// R655: a Unit's `attack` hook, played as the viewer picks it up to attack, as in Hearthstone: `Game`
// calls it on a drag's lift (`DragLayer`'s `onLift`) and on a click choosing the attacker, both only for a
// Unit `legalActions` lets attack (`planDrag`, `onClickTarget`). Nothing plays on the drop, a cancel or
// the hit, and nothing is sent: a client cue, not a game event (CLAUDE.md rule 7). It reads only the
// viewer's own units in the view the board shows (R203). It never throws.

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
