// R2784000 #2799: Training Grounds' pass over its animation table (traininggrounds.json5): at least
// FX_SET_PASS_FLOOR of its 60 non-token cards wear an entry other than `none`, every Spell, Field Spell,
// Trap and Field Trap has a `cast` or a recipe, every Field Spell an aura that stands on the board, every
// hook a drawn motif that draws on its gallery fixture, three to six flourishes each with a fixture, and a
// hidden play keys none (R202). traininggrounds.md, the pass's per-card table, holds one row per card.

import { describe, expect, it } from "vitest";

import { flavorOf } from "../../cards/flavor.ts";
import { entryOf, expectPassTable, playMotif, playMotifs, setCards } from "../../test/setPass.ts";
import { fullBoardView } from "../../test/fixtures.ts";
import { CATALOG } from "../../wire/cards.ts";
import { planAuras } from "../aura/plan.ts";
import { FX_INTENSITY_SCALE, FX_SET_PASS_FLOOR } from "../constants.ts";
import { motifScale } from "../motifs/build.ts";
import { galleryCues, galleryFieldView, RECIPE_FIXTURES } from "../motifs/gallery.ts";
import { isDrawnMotif } from "../motifs/names.ts";
import { isCardRecipeKey } from "./bespoke.ts";
import rawTable from "./traininggrounds.md?raw";

const SET = setCards("Training Grounds");
const CARDS = SET.filter((id) => CATALOG[id]?.token !== true);

/** The bounds on a pass's flourishes (#2782): at least three, at most six. */
const FLOURISHES = { min: 3, max: 6 } as const;

describe("R2784000 #2799 Training Grounds' card animation pass", () => {
  it("R2784000 at least FX_SET_PASS_FLOOR of its 60 non-token cards have an entry other than none", () => {
    expect(SET).toHaveLength(61);
    expect(CARDS).toHaveLength(60);
    const drawn = CARDS.filter((id) => entryOf(id).kind !== "none");
    expect(drawn.length).toBeGreaterThanOrEqual(Math.ceil(FX_SET_PASS_FLOOR * CARDS.length));
  });

  it("R2784000 every non-Unit has a cast or a recipe, and every Field Spell an aura that stands in its zone", () => {
    const view = fullBoardView();
    const others = SET.filter((id) => CATALOG[id]?.type !== "Unit");
    expect(others).toHaveLength(28);
    const fields = others.filter((id) => CATALOG[id]?.type === "Field Spell");
    expect(fields).toHaveLength(6);
    for (const id of others) {
      const entry = entryOf(id);
      const plays = entry.kind === "bespoke" || entry.kind === "setPiece" || (entry.kind === "hooks" && entry.hooks.cast !== undefined);
      expect(plays, `${id} (${String(CATALOG[id]?.type)})`).toBe(true);
    }
    for (const id of fields) {
      const entry = entryOf(id);
      expect(entry.kind === "hooks" ? entry.aura : undefined, id).toBeDefined();
      // As the gallery stands it, face up in the viewer's first backrow zone.
      expect(
        planAuras(galleryFieldView(id, view), new Map()).some((aura) => aura.defId === id),
        id,
      ).toBe(true);
    }
  });

  it("R2784000 there are three to six flourishes, each a card recipe with a gallery fixture", () => {
    const keys = SET.flatMap((id) => {
      const entry = entryOf(id);
      return entry.kind === "bespoke" ? [entry.key] : [];
    });
    expect(keys.length).toBeGreaterThanOrEqual(FLOURISHES.min);
    expect(keys.length).toBeLessThanOrEqual(FLOURISHES.max);
    const view = fullBoardView();
    for (const key of keys) {
      expect(isCardRecipeKey(key), key).toBe(true);
      if (isCardRecipeKey(key)) expect(RECIPE_FIXTURES[key](view, "ex", "p1").length, key).toBeGreaterThan(0);
    }
  });

  it("R2784000 every hook names a drawn motif, and each draws on its gallery fixture in its palette at its card's scale", () => {
    const view = fullBoardView();
    for (const id of SET) {
      const entry = entryOf(id);
      if (entry.kind !== "hooks") continue;
      for (const [hook, spec] of Object.entries(entry.hooks)) {
        const label = `${id} ${hook} ${spec.motif}`;
        expect(isDrawnMotif(spec.motif), label).toBe(true);
        if (!isDrawnMotif(spec.motif)) continue;
        const cues = galleryCues(spec.motif, spec.palette ?? flavorOf(CATALOG[id]), motifScale(id, view, 1), view, FX_INTENSITY_SCALE.normal);
        expect(cues.length, label).toBeGreaterThan(0);
      }
    }
  });

  it("R2784000 every cast and cry motif draws on its gallery fixture as its card's play", () => {
    const motifs = playMotifs(SET);
    expect(motifs.length).toBeGreaterThan(0);
    for (const { id, motif } of motifs) {
      const result = playMotif(id, "ex", motif);
      expect(result, `${id} ${motif}`).not.toBeNull();
      expect(result?.cues.length ?? 0, `${id} ${motif}`).toBeGreaterThan(0);
      // R202: no cue names a definition.
      expect(JSON.stringify(result?.cues), id).not.toMatch(/traininggrounds-\d{3}/);
    }
  });

  it("R202 a hidden play keys no Training Grounds entry", () => {
    for (const { id, motif } of playMotifs(SET)) expect(playMotif("hidden", "hidden", motif), `${id} ${motif}`).toBeNull();
  });

  it("R2784000 traininggrounds.md has one row per card, in order, with its name, type, entry, scale, census and why", () => {
    expectPassTable(rawTable, "traininggrounds", SET);
  });
});
