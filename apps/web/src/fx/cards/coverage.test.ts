// R2803000: the audit's coverage test over every fx/cards table (docs/polish/9-card-animations.md,
// "The tables"): at least FX_COVERAGE_FLOOR of the catalog's non-token cards wears an entry other
// than `none`; every Spell, Field Spell, Trap and Field Trap has a `cast` (or a recipe or set piece
// of its own, or the one `none` a Starship part may wear; R3332000: a Field Spell's own `entrance`, which
// draws its play's landing); every Field Spell wears an `aura`; and every frame sits on a card that may
// carry it, every `entrance` a Unit's or a Field Spell's and every `takeover` a Mythic's. R3400000,
// R3389000: a `{ bespoke }` may wear the aura beside its recipe (`entryAura`), as Classic's Field Spells' signatures do.

import { describe, expect, it } from "vitest";

import { entryOf } from "../../test/setPass.ts";
import { CATALOG } from "../../wire/cards.ts";
import { entryAura, FX_CARD_TABLE } from "./tables.ts";
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
      // R3332000: a Field Spell's play is its landing, which its own entrance draws.
      if (entry.kind === "hooks" && type === "Field Spell" && entry.entrance !== undefined) continue;
      expect(entry.kind === "none" ? `none: ${entry.reason}` : entry.kind, `${id} (${String(type)})`).toBe(STARSHIP_PART_NONE);
    }
  });

  it("R2803000 every Field Spell wears an aura", () => {
    const fields = CARDS.filter((id) => CATALOG[id]?.type === "Field Spell");
    expect(fields.length).toBeGreaterThan(0);
    for (const id of fields) {
      const entry = entryOf(id);
      // R3400000: a `{ bespoke }` may wear it (R3389000: Classic's Field Spells' signatures; R3405001: Meditative's;
      // R3395000: Stinkyscape's three).
      expect(entryAura(entry), id).toBeDefined();
    }
  });

  it("R3389000 every Field Spell whose entry is a recipe wears its aura beside it", () => {
    const recipes = CARDS.filter((id) => CATALOG[id]?.type === "Field Spell" && entryOf(id).kind === "bespoke");
    // R3407000: and Stinkyscape part two's Job Application, Alteran Reach and Sanctum.
    // R3405001: Meditative's eight signature Field Spells with a recipe (The Cane's is its entrance alone).
    // R3399000: and Classic+'s Claude's Datacenter (its three token Field Spells are no card of the census).
    // R3395000: and Stinkyscape's three, The Greatest Heist, Rod of Ages and The Narcor Project.
    expect(recipes).toEqual([
    "classic-004",
    "classic-007",
    "classic-028",
    "classic-090",
    "classicplus-078",
      "meditative-007",
      "meditative-011",
      "meditative-026",
      "meditative-040",
      "meditative-063",
      "meditative-071",
      "meditative-075",
      "meditative-080",
      "stinkyscape-007",
      "stinkyscape-016",
      "stinkyscape-039",
      "stinkyscape-112",
      "stinkyscape-148",
      "stinkyscape-152",]);
    for (const id of recipes) expect(entryAura(entryOf(id)), id).toBeDefined();
  });

  it("R3332000 every entrance is a Unit's or a Field Spell's, every takeover a Mythic's and every signature a Legendary's or a Mythic's", () => {
    for (const id of Object.keys(CATALOG)) {
      const entry = entryOf(id);
      if (entry.kind === "none") continue;
      const def = CATALOG[id];
      const rarity = def?.printedRarity ?? def?.rarity;
      if (entry.entrance !== undefined) expect(["Unit", "Field Spell"], `${id} entrance`).toContain(def?.type);
      if (entry.takeover !== undefined) expect(rarity, `${id} takeover`).toBe("Mythic");
      if (entry.signature === true) expect(["Legendary", "Mythic"], `${id} signature`).toContain(rarity);
    }
  });
});
