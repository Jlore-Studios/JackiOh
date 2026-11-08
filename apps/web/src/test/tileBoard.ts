// R3342000: a fixture with 20 Legendary/Mythic permanents across all zones (issue #3342).
// Used for phone frame budget and component testing.

import type { BackrowView, PlayerId, PlayerView } from "@jackioh/shared";

import { tierOf } from "../fx/aura/tile.ts";
import { entryAura, FX_CARD_TABLE } from "../fx/cards/tables.ts";
import { CATALOG } from "../wire/cards.ts";
import { baseView, emptySide, faceUpBackrow, unit } from "./fixtures.ts";

export function tileBoardView(): PlayerView {
  const eligibleUnits: string[] = [];
  const eligibleFields: string[] = [];

  for (const [id, def] of Object.entries(CATALOG)) {
    if (tierOf(def) === null) continue;
    // A card's own aura replaces its tile aura (R3400000: a bespoke entry's too).
    if (entryAura(FX_CARD_TABLE.get(id)) !== undefined) continue;

    if (def.type === "Unit" && eligibleUnits.length < 20) {
      eligibleUnits.push(id);
    } else if (def.type === "Field Spell" && eligibleFields.length < 10) {
      eligibleFields.push(id);
    }
  }

  // Most Legendary and Mythic Field Spells wear an aura of their own (R3400000), so the backrow is filled up with
  // Units drawn standing there, as a Flying Unit may (R1081000), whose tiles wear the aura as a Field Spell's do.
  const backrow = [...eligibleFields, ...eligibleUnits.slice(10)].slice(0, 10);
  const backrowCard = (owner: PlayerId, defId: string, instanceId: string): BackrowView =>
    CATALOG[defId]?.type === "Unit"
      ? faceUpBackrow(owner, { defId, instanceId, type: "Unit", unit: unit(owner, { defId, instanceId }) })
      : faceUpBackrow(owner, { defId, instanceId, type: "Field Spell" });

  return baseView({
    you: emptySide("p1", {
      units: eligibleUnits.slice(0, 5).map((defId, i) => unit("p1", { defId, instanceId: `tile-unit-you-${i + 1}` })),
      backrow: backrow.slice(0, 5).map((defId, i) => backrowCard("p1", defId, `tile-back-you-${i + 1}`)),
    }),
    opponent: emptySide("p2", {
      units: eligibleUnits.slice(5, 10).map((defId, i) => unit("p2", { defId, instanceId: `tile-unit-opp-${i + 1}` })),
      backrow: backrow.slice(5, 10).map((defId, i) => backrowCard("p2", defId, `tile-back-opp-${i + 1}`)),
    }),
  });
}
