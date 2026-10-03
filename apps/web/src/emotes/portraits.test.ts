// R635's portrait roster on the client (portraits.ts): every portrait id resolves, once at module
// load, to the catalog card its `PORTRAITS` entry names — BY NAME, never by number, so the test
// reads the name off the resolved def rather than asserting the lookup went anywhere in
// particular. `portraitOrDefault` is the other half: the deck column's `null` and anything the
// roster does not know both read as `vanilla` (D5).

import { describe, expect, it } from "vitest";

import catalogJson from "@jackioh/cards/catalog.json";
import { PORTRAITS, PORTRAIT_IDS, portraitOrDefault, type PortraitId } from "@jackioh/shared";
import type { CardDef } from "@jackioh/shared";

import { PORTRAIT_DEFS } from "./portraits.ts";

const catalog = catalogJson as unknown as Record<string, CardDef>;

/** The issue's roster table (R635): each portrait id, its card's name and the card it lands on. */
const ROSTER: Record<PortraitId, { cardName: string; defId: string }> = {
  vanilla: { cardName: "Mr. Vanilla", defId: "core-008" },
  gary: { cardName: "Gary the Gambler", defId: "core-004" },
  timmy: { cardName: "Tempo Timmy", defId: "core-011" },
  dfender: { cardName: "Big D-fender", defId: "core-001" },
  felinors: { cardName: "Duplicating Felinors", defId: "core-012" },
  shredder: { cardName: "Jlockeed Shredder-10", defId: "core-013" },
};

describe("R635 the portrait roster", () => {
  it("R635 all six PORTRAIT_IDS resolve to a real catalog card def, by name", () => {
    expect(Object.keys(PORTRAIT_DEFS).sort()).toEqual([...PORTRAIT_IDS].sort());

    for (const id of PORTRAIT_IDS) {
      const { defId, def } = PORTRAIT_DEFS[id];
      // The roster's cardName found the def: the name on it is the name the issue's table gives.
      expect(def.name, id).toBe(PORTRAITS[id].cardName);
      // And the resolution is a catalog entry — the same object the Almanac would draw.
      expect(catalog[defId]?.name, `${id} -> ${defId}`).toBe(def.name);
    }
  });

  it("R635 the launch roster maps to the issue's cards, each portrait its own card", () => {
    const seen = new Set<string>();
    for (const id of PORTRAIT_IDS) {
      const { defId, def } = PORTRAIT_DEFS[id];
      const expected = ROSTER[id];
      expect(def.name, id).toBe(expected.cardName);
      expect(defId, id).toBe(expected.defId);
      expect(seen.has(defId), `two portraits share ${defId}`).toBe(false);
      seen.add(defId);
    }
  });

  it("R635 portraitOrDefault reads a null column and an unknown id as vanilla", () => {
    for (const raw of [null, undefined, "", "nobody", "MR. VANILLA", "vanilla "]) {
      expect(portraitOrDefault(raw), JSON.stringify(raw)).toBe("vanilla");
    }
    for (const id of PORTRAIT_IDS) {
      expect(portraitOrDefault(id), id).toBe(id);
    }
  });
});
