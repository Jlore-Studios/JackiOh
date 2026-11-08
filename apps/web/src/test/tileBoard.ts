// R3342000: a fixture with 20 Legendary/Mythic permanents across all zones (issue #3342).
// Used for phone frame budget and component testing.

import type { PlayerView } from "@jackioh/shared";

import { tierOf } from "../fx/aura/tile.ts";
import { FX_CARD_TABLE, auraOfEntry } from "../fx/cards/tables.ts";
import { CATALOG } from "../wire/cards.ts";
import { baseView, emptySide, faceUpBackrow, unit } from "./fixtures.ts";

/** The face-up backrow permanents a tile aura marks, as tile.test.ts's coverage counts them. */
const BACKROW_TYPES: readonly string[] = ["Field Spell", "Field Trap", "Trap"];

export function tileBoardView(): PlayerView {
  const eligibleUnits: string[] = [];
  const eligibleFields: string[] = [];

  for (const [id, def] of Object.entries(CATALOG)) {
    if (tierOf(def) === null) continue;
    // A card whose entry wears an aura (beside hooks or, R3399001, a bespoke recipe) stands that instead.
    if (auraOfEntry(FX_CARD_TABLE.get(id)) !== undefined) continue;

    if (def.type === "Unit" && eligibleUnits.length < 10) {
      eligibleUnits.push(id);
    } else if (BACKROW_TYPES.includes(def.type) && eligibleFields.length < 10) {
      // Every Field Spell wears a table aura of its own (R2803000), so the backrow fills with face-up Traps too.
      eligibleFields.push(id);
    }
  }

  return baseView({
    you: emptySide("p1", {
      units: eligibleUnits.slice(0, 5).map((defId, i) => unit("p1", { defId, instanceId: `tile-unit-you-${i + 1}` })),
      backrow: eligibleFields.slice(0, 5).map((defId, i) =>
        faceUpBackrow("p1", { defId, instanceId: `tile-back-you-${i + 1}`, type: CATALOG[defId]?.type ?? "Field Spell" }),
      ),
    }),
    opponent: emptySide("p2", {
      units: eligibleUnits.slice(5, 10).map((defId, i) => unit("p2", { defId, instanceId: `tile-unit-opp-${i + 1}` })),
      backrow: eligibleFields.slice(5, 10).map((defId, i) =>
        faceUpBackrow("p2", { defId, instanceId: `tile-back-opp-${i + 1}`, type: CATALOG[defId]?.type ?? "Field Spell" }),
      ),
    }),
  });
}
