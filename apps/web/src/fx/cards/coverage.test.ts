// R2803000: the audit's coverage test over every fx/cards table (docs/polish/9-card-animations.md,
// "The tables"): at least FX_COVERAGE_FLOOR of the catalog's non-token cards wears an entry other
// than `none`; every Spell, Field Spell, Trap and Field Trap has a `cast` (or a recipe or set piece
// of its own, or the one `none` a Starship part may wear); every Field Spell wears an `aura`.

import { describe, expect, it } from "vitest";

import { entryOf } from "../../test/setPass.ts";
import { CATALOG } from "../../wire/cards.ts";
import { FX_CARD_TABLE } from "./tables.ts";
import { FX_COVERAGE_FLOOR } from "../constants.ts";

/** The catalog's non-token cards, in catalog order. */
const CARDS = Object.keys(CATALOG).filter((id) => CATALOG[id]?.token !== true);

/** A Spell part's own moments are the Starship's, not the card's: its entry stays `none` (R2801000). */
const STARSHIP_PART_NONE = "none: Starship part";

describe("R2803000 the audit's coverage", () => {
  it("R2803000 at least half of non-token cards animate", () => {
    expect(CARDS).toHaveLength(634);
    expect(FX_CARD_TABLE.size).toBe(Object.keys(CATALOG).length);
    const drawn = CARDS.filter((id) => entryOf(id).kind !== "none");
    expect(drawn.length).toBeGreaterThanOrEqual(Math.ceil(FX_COVERAGE_FLOOR * CARDS.length));
  });

  it("R2803000 every Spell, Field Spell, Trap and Field Trap has a cast", () => {
    for (const id of CARDS) {
      const type = CATALOG[id]?.type;
      if (type !== "Spell" && type !== "Field Spell" && type !== "Trap" && type !== "Field Trap") continue;
      const entry = entryOf(id);
      if (entry.kind === "bespoke" || entry.kind === "setPiece") continue;
      if (entry.kind === "hooks" && entry.hooks.cast !== undefined) continue;
      expect(entry.kind === "none" ? `none: ${entry.reason}` : entry.kind, `${id} (${String(type)})`).toBe(STARSHIP_PART_NONE);
    }
  });

  it("R2803000 every Field Spell wears an aura", () => {
    const fields = CARDS.filter((id) => CATALOG[id]?.type === "Field Spell");
    expect(fields.length).toBeGreaterThan(0);
    for (const id of fields) {
      const entry = entryOf(id);
      expect(entry.kind === "hooks" ? entry.aura : undefined, id).toBeDefined();
    }
  });
});
