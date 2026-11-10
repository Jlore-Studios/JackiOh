// R2784000 #2793: Core's pass over its animation table (core.json5): at least FX_SET_PASS_FLOOR of its
// non-token cards wear an entry other than `none`, every Spell, Field Spell, Trap and Field Trap has a
// `cast` (or a recipe or set piece of its own), every Field Spell an aura, every hook a drawn motif, and
// every play's motif draws on its gallery fixture as that card's play, a hidden play keying none (R202).
// core.md, the pass's per-card table, holds one row per Core card that matches the table.

import { describe, expect, it } from "vitest";

import { entryOf, expectPassTable, playMotif, playMotifs, setCards } from "../../test/setPass.ts";
import { CATALOG } from "../../wire/cards.ts";
import { entryAura } from "./tables.ts";
import { FX_SET_PASS_FLOOR } from "../constants.ts";
import { isDrawnMotif } from "../motifs/names.ts";
import rawTable from "./core.md?raw";

const CORE = setCards("Core");
const CARDS = CORE.filter((id) => CATALOG[id]?.token !== true);

describe("R2784000 #2793 Core's card animation pass", () => {
  it("R2784000 at least FX_SET_PASS_FLOOR of Core's non-token cards have an entry other than none", () => {
    expect(CARDS).toHaveLength(100);
    const drawn = CARDS.filter((id) => entryOf(id).kind !== "none");
    expect(drawn.length).toBeGreaterThanOrEqual(Math.ceil(FX_SET_PASS_FLOOR * CARDS.length));
  });

  it("R2784000 every Core non-Unit has a cast, bespoke or set piece, and every Field Spell an aura", () => {
    const fields = CORE.filter((id) => CATALOG[id]?.type === "Field Spell");
    expect(fields).toHaveLength(16);
    for (const id of CORE) {
      const type = CATALOG[id]?.type;
      if (type === "Unit") continue;
      const entry = entryOf(id);
      const plays = entry.kind === "bespoke" || entry.kind === "setPiece" || (entry.kind === "hooks" && entry.hooks.cast !== undefined);
      expect(plays, `${id} (${String(type)})`).toBe(true);
      // R3402000: a `{ bespoke }` entry may wear it too (Combo-Index's, Heroic Power's).
      if (type === "Field Spell") expect(entryAura(entry), id).toBeDefined();
    }
  });

  it("R2784000 every Core hook names a drawn motif", () => {
    for (const id of CORE) {
      const entry = entryOf(id);
      if (entry.kind !== "hooks") continue;
      for (const [hook, spec] of Object.entries(entry.hooks)) expect(isDrawnMotif(spec.motif), `${id} ${hook} ${spec.motif}`).toBe(true);
    }
  });

  it("R2784000 every Core cast and cry motif draws on its gallery fixture as its card's play", () => {
    const motifs = playMotifs(CORE);
    expect(motifs.length).toBeGreaterThan(0);
    for (const { id, motif } of motifs) {
      const result = playMotif(id, "ex", motif);
      expect(result, `${id} ${motif}`).not.toBeNull();
      expect(result?.cues.length ?? 0, `${id} ${motif}`).toBeGreaterThan(0);
      // R202: no cue names a definition.
      expect(JSON.stringify(result?.cues), id).not.toMatch(/core-\d{3}/);
    }
  });

  it("R202 a hidden play keys no Core entry", () => {
    for (const { id, motif } of playMotifs(CORE)) expect(playMotif("hidden", "hidden", motif), `${id} ${motif}`).toBeNull();
  });

  it("R2784000 core.md has one row per Core card, in order, with its name, type, entry, scale, census and why", () => {
    expectPassTable(rawTable, "core", CORE);
  });
});
