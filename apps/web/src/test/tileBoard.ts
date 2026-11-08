// R3342000: a fixture with 20 Legendary/Mythic permanents across all zones (issue #3342).
// Used for phone frame budget and component testing.

import type { PlayerView } from "@jackioh/shared";

import { tierOf } from "../fx/aura/tile.ts";
import { FX_CARD_TABLE } from "../fx/cards/tables.ts";
import { CATALOG } from "../wire/cards.ts";
import { baseView, emptySide, faceUpBackrow, unit } from "./fixtures.ts";

export function tileBoardView(): PlayerView {
  const eligibleUnits: string[] = [];
  const eligibleFields: string[] = [];

  for (const [id, def] of Object.entries(CATALOG)) {
    if (tierOf(def) === null) continue;
    const tableEntry = FX_CARD_TABLE.get(id);
    if (tableEntry?.kind === "hooks" && tableEntry.aura !== undefined) continue;

    if (def.type === "Unit" && eligibleUnits.length < 10) {
      eligibleUnits.push(id);
    } else if (def.type === "Field Spell" && eligibleFields.length < 10) {
      eligibleFields.push(id);
    }
  }

  return baseView({
    you: emptySide("p1", {
      units: eligibleUnits.slice(0, 5).map((defId, i) => unit("p1", { defId, instanceId: `tile-unit-you-${i + 1}` })),
      backrow: eligibleFields.slice(0, 5).map((defId, i) =>
        faceUpBackrow("p1", { defId, instanceId: `tile-back-you-${i + 1}`, type: "Field Spell" }),
      ),
    }),
    opponent: emptySide("p2", {
      units: eligibleUnits.slice(5, 10).map((defId, i) => unit("p2", { defId, instanceId: `tile-unit-opp-${i + 1}` })),
      backrow: eligibleFields.slice(5, 10).map((defId, i) =>
        faceUpBackrow("p2", { defId, instanceId: `tile-back-opp-${i + 1}`, type: "Field Spell" }),
      ),
    }),
  });
}
