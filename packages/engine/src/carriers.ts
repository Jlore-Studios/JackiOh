// A backrow card that carries a Unit (docs/classic-sets.md B5 E21, Classic+ #33 Ivory Tower; R446).
//
// The Unit stands in the carrier's backrow zone (`PlayerState.carried`) as a Unit for every rule — every
// unit walk finds it through `zones.activeUnitsOf` — that can neither attack nor be attacked
// (`combat.ts`), with the carrier still acting beneath it: its aura and its text keep working, and
// backrow effects find the carrier, never the Unit (`zones.cardAt`). The zone takes nothing more while
// it carries one (`zones.acceptsStackCard`). This module owns the one sequence the arrangement needs:
// when the zone's acting card stops carrying — the carrier left the field, or lost its text to a
// Vanilla — the Unit steps down into a unit zone of its side without leaving the field, or is
// destroyed for want of one. §4.5's check runs it first in every pass (`stateCheck`), so a Unit is
// never left standing on nothing past the effect that took its carrier away.

import { PLAYER_IDS } from "@jackioh/shared";
import type { FieldSink } from "./animated";
import {
  cardAt,
  carriedAt,
  firstEntryZone,
  isCarrier,
  isOpen,
  slotsOf,
  stepIntoUnitZone,
  type ZoneSlot,
} from "./zones";

/**
 * R446: set in a stranded Unit's memory when it was destroyed for want of a unit zone and survived it
 * (Indestructible, R46), so it is destroyed once and not again at every check while it waits for one.
 * Memory, so R78 clears it when the card leaves the field.
 */
const STRANDED_KEY = "__stranded";

/**
 * R446: every Unit whose backrow zone no longer carries it steps down into its side's unit zone in the
 * same lane when that is open, else R64's leftmost open one — moving, not leaving the field: no Cry, no
 * R78 reset, and not summoning sick, since it entered nothing (it was on the field all along). The move
 * is reported as `animated` with `carried` set, a backrow zone to a unit zone as an animation is. With
 * no open unit zone it is destroyed (§6.3 Destroy: marked for the check that is running, no killer); an
 * Indestructible one stays standing where it is and steps down once a unit zone opens.
 */
export function settleCarried(sink: FieldSink): void {
  const state = sink.state;
  for (const player of PLAYER_IDS) {
    if (state.players[player].carried === undefined) continue;
    for (const ref of slotsOf(player, "backrow")) {
      const unit = carriedAt(state, ref);
      if (unit === null) continue;
      const top = cardAt(state, ref);
      if (top !== null && isCarrier(top)) continue;
      const same: ZoneSlot = { player, row: "units", lane: ref.lane };
      const to = isOpen(state, same) ? same : firstEntryZone(state, player, "units");
      if (to === null) {
        if (unit.memory[STRANDED_KEY] === true) continue;
        unit.memory[STRANDED_KEY] = true;
        unit.markedDestroyed = true;
        // R42: a destroy names no killer, and clears an older hit's credit as it marks.
        delete unit.lastDamagedBy;
        continue;
      }
      if (!stepIntoUnitZone(state, unit, to)) continue;
      delete unit.memory[STRANDED_KEY];
      sink.events.push({
        type: "animated",
        player,
        instanceId: unit.id,
        defId: unit.defId,
        backrowLane: ref.lane,
        unitLane: to.lane,
        carried: true,
      });
    }
  }
}
