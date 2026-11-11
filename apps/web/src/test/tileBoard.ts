// R3342000: a fixture with 20 Legendary/Mythic permanents across all zones (issue #3342).
// Used for phone frame budget and component testing.

import type { PlayerView } from "@jackioh/shared";

import { tierOf } from "../fx/aura/tile.ts";
import { entryAura, FX_CARD_TABLE } from "../fx/cards/tables.ts";
import { CATALOG } from "../wire/cards.ts";
import { baseView, emptySide, faceUpBackrow, unit } from "./fixtures.ts";

/** The face-up backrow permanents a tile aura marks, as tile.test.ts's coverage counts them. */
const BACKROW_TYPES: readonly string[] = ["Field Spell", "Field Trap", "Trap"];

export function tileBoardView(): PlayerView {
  const eligibleUnits: string[] = [];
  const eligibleFields: string[] = [];

  for (const [id, def] of Object.entries(CATALOG)) {
    if (tierOf(def) === null) continue;
    // A card's own aura replaces its tile aura (R3400000: a bespoke entry's too).
    if (entryAura(FX_CARD_TABLE.get(id)) !== undefined) continue;

    if (def.type === "Unit" && eligibleUnits.length < 10) {
      eligibleUnits.push(id);
    } else if (BACKROW_TYPES.includes(def.type) && eligibleFields.length < 10) {
      // Every Field Spell wears a table aura of its own (R2803000), so the backrow fills with face-up Traps too.
      eligibleFields.push(id);
    }
  }

  // R3342000: most Legendary and Mythic permanents wear an aura of their own (R3399001: since Classic+'s pass,
  // every Field Spell; R3395000: since Stinkyscape's, most of its Units), so the ones left to wear the tile aura
  // (eight Units and five face-up Traps today) take their row's five zones a side in turn: still one permanent
  // per zone.
  const fiveFrom = (eligible: readonly string[], from: number): string[] =>
    eligible.length === 0 ? [] : Array.from({ length: 5 }, (_, i) => eligible[(from + i) % eligible.length] ?? "");

  return baseView({
    you: emptySide("p1", {
      units: fiveFrom(eligibleUnits, 0).map((defId, i) => unit("p1", { defId, instanceId: `tile-unit-you-${i + 1}` })),
      backrow: fiveFrom(eligibleFields, 0).map((defId, i) => faceUpBackrow("p1", { defId, instanceId: `tile-back-you-${i + 1}`, type: CATALOG[defId]?.type ?? "Field Spell" })),
    }),
    opponent: emptySide("p2", {
      units: fiveFrom(eligibleUnits, 5).map((defId, i) => unit("p2", { defId, instanceId: `tile-unit-opp-${i + 1}` })),
      backrow: fiveFrom(eligibleFields, 5).map((defId, i) => faceUpBackrow("p2", { defId, instanceId: `tile-back-opp-${i + 1}`, type: CATALOG[defId]?.type ?? "Field Spell" })),
    }),
  });
}
