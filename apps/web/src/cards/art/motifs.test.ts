// R503: v0.2.0's art. The Book, Pancake and AI families, the motif a card's name calls up, one
// distinct picture for every catalog entry and face, and Core's pictures kept where no motif moves
// them. Pure: nothing renders here (CardArt.test.tsx and CardFace.test.tsx cover the DOM).

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef, CardType } from "@jackioh/shared";

import CORE_V01 from "./fixtures/core-art-v0.1.json";
import { EMBLEMS } from "./emblems.ts";
import { hashId } from "./hash.ts";
import { MOTIF_GLYPHS } from "./motifGlyphs.ts";
import { MOTIF_WORDS, MOTIFS, THEME_OWN_MOTIFS, motifFor, nameWords, type MotifId } from "./motifs.ts";
import { artSpec, MOTIF_EMBLEM_SCALE, varietySalt, type ArtSpec } from "./procedural.ts";
import { artDataUri } from "./svg.ts";
import {
  EMBLEM_POOLS,
  HUE_DRIFT,
  TAG_THEMES,
  THEME_MOTES,
  THEME_PALETTES,
  compositionFor,
  themeFor,
  type ArtThemeId,
} from "./themes.ts";

const DEFS: readonly CardDef[] = Object.values(CATALOG);
const SVG_PREFIX = "data:image/svg+xml,";
/**
 * Where a glyph's centre may stand: a full face's art window shows about y 18 to 82 of the square
 * (a Unit's portrait 12 to 88), and a portrait's oval narrows at the sides.
 */
const SHOWN = { left: 10, right: 90, top: 22, bottom: 78, figureBottom: 84 } as const;

/** The themes whose cards have no emblem of their own, so a motif becomes the emblem. */
const PLAIN_THEMES: readonly ArtThemeId[] = ["unit", "spell", "field-spell", "trap", "field-trap", "token"];

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`the catalog has no ${id}`);
  return found;
}

/** One face's picture as CardFace draws it: the face's own type (B2.7), its theme and its motif. */
function faceSpec(card: CardDef, radiant: boolean): ArtSpec {
  const type: CardType = (radiant ? card.radiant.type : card.base.type) ?? card.type;
  const theme = themeFor(card.tags, type);
  return artSpec(card.id, theme, compositionFor(type), radiant, motifFor(card.name, theme));
}

function svgOf(spec: ArtSpec): string {
  return decodeURIComponent(artDataUri(spec).slice(SVG_PREFIX.length));
}

describe("R503 the Book, Pancake and AI families", () => {
  it("R503 each new tag is a theme, placed in the priority order: AI after Call to Chaos, Book after KY, Pancake before Fruit", () => {
    expect(TAG_THEMES.map(([tag]) => tag)).toEqual([
      "Call to Chaos",
      "AI",
      "CN",
      "KY",
      "Book",
      "Felinor",
      "Pancake",
      "Fruit",
      "Quickdraw",
      "Human",
    ]);
    expect(themeFor(["Book"], "Spell")).toBe("book");
    expect(themeFor(["Pancake", "Token"], "Field Spell")).toBe("pancake");
    expect(themeFor(["AI", "Token"], "Trap")).toBe("ai");
    expect(themeFor(["Human", "Pancake"], "Unit")).toBe("pancake");
    expect(themeFor(["KY", "Book"], "Spell")).toBe("ky");
  });

  it("R503 every Book, Pancake and AI catalog card wears its family, tokens included", () => {
    for (const card of DEFS) {
      for (const [tag, theme] of [["Book", "book"], ["Pancake", "pancake"], ["AI", "ai"]] as const) {
        if (card.tags.includes(tag)) expect(themeFor(card.tags, card.type), card.id).toBe(theme);
      }
    }
    expect(DEFS.filter((card) => themeFor(card.tags, card.type) === "ai")).toHaveLength(10);
  });

  it("R503 each family has its palette, its own emblem first in its pool, a hue drift and its motes", () => {
    const signature: Readonly<Record<"book" | "pancake" | "ai", string>> = { book: "tome", pancake: "pancakes", ai: "chip" };
    for (const [theme, glyph] of Object.entries(signature) as [keyof typeof signature, string][]) {
      expect(THEME_PALETTES[theme].emblem.glyph, theme).toBe(glyph);
      expect(EMBLEM_POOLS[theme][0], theme).toBe(glyph);
      for (const pooled of EMBLEM_POOLS[theme]) expect(EMBLEMS[pooled], `${theme} ${pooled}`).toBeDefined();
      expect(HUE_DRIFT[theme], theme).toBeGreaterThan(0);
    }
    expect(THEME_MOTES.book).toBe("drop");
    expect(THEME_MOTES.ai).toBe("pixel");
    expect(THEME_MOTES.pancake).toBe("dot");
    // No plain theme draws a family's emblem from its pool, so an untagged card never passes for one.
    for (const theme of PLAIN_THEMES) {
      for (const glyph of ["tome", "pancakes", "chip"] as const) expect(EMBLEM_POOLS[theme], theme).not.toContain(glyph);
    }
  });

  it("R503 each family's twist: a Book card's pages and ink drops, a Pancake card's syrup, an AI card's traces and pixels", () => {
    const book = faceSpec(def("classicplus-054"), false);
    expect(book.moteShape).toBe("drop");
    expect(svgOf(book)).toMatch(/<path d="M[\d.]+ [\d.]+L[\d.]+ [\d.]+A[\d.]+ [\d.]+ 0 1 1/);

    const pancake = faceSpec(def("classicplus-012-1"), false);
    // Syrup along the top edge: a layer that starts with a band across the whole top of the box.
    expect(pancake.ridges.some((ridge) => ridge.d.startsWith("M0 0L100 0"))).toBe(true);

    const ai = faceSpec(def("classicplus-t-ai-08"), false);
    expect(ai.moteShape).toBe("pixel");
    const svg = svgOf(ai);
    expect(svg).toContain("<rect x=");
    // A plain Trap of the same composition draws fewer layers: the traces, glitch and scanlines are the family's.
    const plain = artSpec("classicplus-t-ai-08", "trap", "sigil", false, null);
    expect(ai.ridges.length).toBe(plain.ridges.length + 3);
  });

  it("R503 a family token's frame is its family's picture, never the grey Token one", () => {
    for (const id of ["classicplus-012-5", "classicplus-t-ai-03", "classicplus-065-5"]) {
      expect(faceSpec(def(id), false).theme, id).not.toBe("token");
    }
  });
});

describe("R503 a card's motif comes from its name", () => {
  it("R503 nameWords splits a name into lowercase words of letters and digits", () => {
    expect(nameWords("KY's Papaya")).toEqual(["ky", "s", "papaya"]);
    expect(nameWords("Wrong-House Attacker")).toEqual(["wrong", "house", "attacker"]);
    expect(nameWords("BOOM! Big Max")).toEqual(["boom", "big", "max"]);
    expect(nameWords("Forever&")).toEqual(["forever"]);
  });

  it("R503 a word matches whole or plural, `word*` by its start and `*word` by its end", () => {
    expect(motifFor("Burn")).toBe("flames");
    expect(motifFor("Conjure Bones")).toBe("bones");
    expect(motifFor("Flame Lance")).toBe("flames");
    expect(motifFor("Pestilent Slime")).toBe("spores");
    expect(motifFor("Doom Shroom")).toBe("mushroom");
    expect(motifFor("Anti-Softlock")).toBe("lock");
    expect(motifFor("Wardrum")).toBe("drum");
    // "Bat" is a word, never the start of one: a Battle names no bat.
    expect(motifFor("Battle Plan")).toBeNull();
  });

  it("R503 the first motif in the table wins: the thing a card is before what it does", () => {
    expect(motifFor("Plague Doctor")).toBe("spores");
    expect(motifFor("Anti-Magic Monkey")).toBe("monkey");
    expect(motifFor("Chaos Golem")).toBe("rock");
    expect(motifFor("Datacenter Fire")).toBe("flames");
    expect(motifFor("Lag in the System")).toBe("clock");
    expect(motifFor("Blade Storm")).toBe("blades");
  });

  it("R503 a family skips the motif its own picture already draws", () => {
    expect(THEME_OWN_MOTIFS).toEqual({ book: ["book"], felinor: ["cat"], pancake: ["pancake"], ai: ["circuits"] });
    expect(motifFor("Book of Flame", "book")).toBe("flames");
    expect(motifFor("Book of Books", "book")).toBeNull();
    expect(motifFor("Book Worm", "unit")).toBe("book");
    expect(motifFor("Felinor Flagbearer", "felinor")).toBe("flag");
    expect(motifFor("Felinor Fiender", "human")).toBe("cat");
    expect(motifFor("Anti-Waffle Shell", "pancake")).toBe("armor");
    expect(motifFor("Hallucination", "ai")).toBe("eyes");
    expect(motifFor("Hallucination", "spell")).toBe("circuits");
  });

  it("R503 no name, or a name about nothing drawable, has no motif", () => {
    expect(motifFor(undefined)).toBeNull();
    expect(motifFor(null)).toBeNull();
    expect(motifFor("")).toBeNull();
    expect(motifFor("core-042")).toBeNull();
    expect(motifFor("Guy Att")).toBeNull();
  });

  it("R503 the table is whole: every motif is drawable and reachable, every glyph exists and is clean", () => {
    const listed = new Set(MOTIF_WORDS.map(([motif]) => motif));
    expect(listed.size, "a motif listed twice").toBe(MOTIF_WORDS.length);
    expect([...listed].sort()).toEqual((Object.keys(MOTIFS) as MotifId[]).sort());
    for (const [motif, drawn] of Object.entries(MOTIFS)) {
      expect(EMBLEMS[drawn.glyph], motif).toBeDefined();
      expect(drawn.fill, motif).toMatch(/^#[0-9a-f]{6}$/);
      expect(drawn.stroke, motif).toMatch(/^#[0-9a-f]{6}$/);
    }
    for (const [glyph, path] of Object.entries(MOTIF_GLYPHS)) {
      expect(path.d, glyph).toMatch(/^M[-\d. MLAZ]+$/);
      expect(path.d, glyph).not.toContain("NaN");
      expect(EMBLEMS[glyph as keyof typeof MOTIF_GLYPHS], glyph).toBe(path);
    }
  });

  it("R503 the new cards' names call up fitting motifs", () => {
    const expected: readonly (readonly [string, MotifId])[] = [
      ["classic-036", "flames"], // Burn
      ["classic-055", "flames"], // Book of Wildfire
      ["classic-083", "flames"], // Flame Lance
      ["classic-043", "spores"], // Plague Nuke
      ["classic-027", "spores"], // Pestilent Slime
      ["classic-042", "spores"], // Transmutable Toxins
      ["classic-039", "spores"], // Outbreak
      ["classic-074", "spores"], // Corpse Plantation
      ["classicplus-001", "mushroom"], // Doom Shroom
      ["classicplus-049", "mushroom"], // Jay Fungus
      ["classicplus-039", "book"], // Book Worm
      ["classic-067", "cat"], // Felinor Feeler (Human)
      ["classic-082", "sheep"], // Sheeople
      ["classic-050", "wisps"], // Voidwalker
      ["classic-089", "wisps"], // Paul Allen's Ghost
      ["classicplus-025", "wisps"], // Soul Shot
      ["classic-014", "wisps"], // Shadowstep
      ["classic-045", "rock"], // Nature Titan
      ["classicplus-073-1", "rock"], // Classic Golem
      ["classic-077", "monkey"], // Anti-Magic Monkey
      ["classic-048", "shrimp"], // Hired Shrimp
      ["classic-021", "turtle"], // Turtinator
      ["classic-019", "lizard"], // Lizard's Breath
      ["classicplus-065", "grapes"], // Two Grapes
      ["classicplus-066", "grapes"], // Vine of Grapes
      ["classicplus-067", "fruit"], // Pear
      ["classicplus-059", "fruit"], // All Purpose Apple
      ["classicplus-062", "fruit"], // KY's Papaya
      ["classic-085", "crown"], // King Wagtoggle
      ["classic-079", "dice"], // Risky Die
      ["classic-052", "dice"], // Final Gambit
      ["classicplus-073", "dice"], // Call to Chaos (Classic+ Edition)
      ["classic-054", "clock"], // Rewind
      ["classicplus-035", "clock"], // Rollback
      ["classicplus-026", "clock"], // Tommy Tempo
      ["classic-084", "lock"], // Lockdown
      ["classicplus-077", "lock"], // Anti-Softlock
      ["classic-017", "broken-rune"], // Counterspell
      ["classic-072", "broken-rune"], // Grand Counterspell
      ["classicplus-009", "broken-rune"], // Silence
      ["classicplus-033", "tower"], // Ivory Tower
      ["classicplus-007", "house"], // The House
      ["classic-062", "bomb"], // Living Bomb
      ["classic-080", "bomb"], // BOOM! Big Max
      ["classic-073", "cross"], // Nurse Cleaver
      ["classicplus-060", "cross"], // Doctors Orders
      ["classic-003", "cross"], // Book of Heal
      ["classic-009", "coins"], // Income Tax
      ["classic-038", "coins"], // Jackiestan Auctioneer
      ["classicplus-048", "coins"], // Jlockheed's Lobbyist
      ["classicplus-055", "coins"], // Book of Greed
      ["classicplus-078", "circuits"], // Claude's Datacenter
      ["classicplus-043", "circuits"], // AI Slop
      ["classicplus-036", "bones"], // Conjure Bones
      ["classicplus-008", "storm"], // Withering Storm
      ["classicplus-021", "wind"], // Whirlwind
      ["classicplus-037", "drum"], // Wardrum
      ["classicplus-038", "sun"], // Solarius
      ["classicplus-003", "snake"], // Second Amendment Snake
      ["classicplus-004", "bat"], // Juhan Biggest Bat
      ["classic-033", "web"], // Joro
      ["classicplus-012-6", "frost"], // Frozen Wastes
      ["classicplus-057", "rise"], // Book of Stats
      ["classicplus-072", "fall"], // Book of Nerf
    ];
    for (const [id, motif] of expected) {
      const card = def(id);
      expect(motifFor(card.name, themeFor(card.tags, card.type)), `${id} ${card.name}`).toBe(motif);
    }
  });

  it("R503 every Classic and Classic+ entry has a motif but for the few whose names draw nothing", () => {
    const bare = DEFS.filter((card) => card.set !== "Core")
      .filter((card) => motifFor(card.name, themeFor(card.tags, card.type)) === null)
      .map((card) => card.name);
    expect(bare.sort()).toEqual(
      [
        "Boots on the Ground",
        "Mid Runner",
        "Prep",
        "State of the Game",
        "Genn",
        "Guy Att",
        "The Mother Pancake",
        "Fluffy Grip",
        "Powder Spray",
        "Mommy Barker",
        "Top Loser",
        "Mid Loser",
        "Support Loser",
        "Bot Loser",
        "KY's Constant",
        "Book of Books",
        "Brother Lar",
        "Autocomplete",
      ].sort(),
    );
  });
});

describe("R503 how a motif is drawn", () => {
  it("R503 a plain card wears its motif as its emblem, in the motif's own colours; a radiant face gilds it", () => {
    const burn = def("classic-036");
    const base = faceSpec(burn, false);
    expect(base.theme).toBe("spell");
    expect(base.motif).toBe("flames");
    expect(base.emblem.glyph).toBe(MOTIFS.flames.glyph);
    expect(base.emblem.fill).toBe(MOTIFS.flames.fill);
    expect(base.emblem.stroke).toBe(MOTIFS.flames.stroke);
    const radiant = faceSpec(burn, true);
    expect(radiant.emblem.glyph).toBe(MOTIFS.flames.glyph);
    expect(radiant.emblem.fill).not.toBe(MOTIFS.flames.fill);
  });

  it("R503 a tribe keeps its emblem and carries a hero motif in a top corner, where the accent would go", () => {
    const flagbearer = faceSpec(def("classicplus-046"), false);
    expect(flagbearer.theme).toBe("felinor");
    expect(EMBLEM_POOLS.felinor).toContain(flagbearer.emblem.glyph);
    expect(flagbearer.accent).toBeNull();
    expect(flagbearer.motifGlyphs).toHaveLength(1);
    const [hero] = flagbearer.motifGlyphs;
    expect(hero?.glyph).toBe("flag");
    // In the upper part of the band a full face's window shows (about y 18 to 82 of the box).
    expect(hero?.y).toBeGreaterThanOrEqual(22);
    expect(hero?.y).toBeLessThanOrEqual(40);
  });

  it("R503 motif glyphs stand inside the band every face's window shows, and a scatter, rise or fall keeps clear of the emblem", () => {
    for (const card of DEFS) {
      const spec = faceSpec(card, false);
      for (const placed of spec.motifGlyphs) {
        const where = `${card.id} ${placed.glyph}`;
        expect(placed.x, where).toBeGreaterThan(SHOWN.left);
        expect(placed.x, where).toBeLessThan(SHOWN.right);
        expect(placed.y, where).toBeGreaterThanOrEqual(SHOWN.top);
        expect(placed.y, where).toBeLessThanOrEqual(spec.composition === "figure" ? SHOWN.figureBottom : SHOWN.bottom);
        if (spec.motif !== null && MOTIFS[spec.motif].arrangement !== "hero") {
          const gap = Math.hypot(placed.x - spec.emblem.x, placed.y - spec.emblem.y);
          const placedAt = spec.emblem.size / (PLAIN_THEMES.includes(spec.theme) ? MOTIF_EMBLEM_SCALE : 1);
          expect(gap, where).toBeGreaterThan((placed.size + placedAt) / 2);
        }
      }
    }
  });

  it("R503 a figure wears a crown, cat ears or horns when its motif says so", () => {
    // Felinor Feeler is Human, and no Human wears ears of its own: the ears are its motif's.
    const feeler = faceSpec(def("classic-067"), false);
    const human = artSpec("classic-067", "human", "figure", false, null);
    expect(feeler.motif).toBe("cat");
    expect(feeler.ridges.map((ridge) => ridge.d)).not.toEqual(human.ridges.map((ridge) => ridge.d));
    expect(MOTIFS.cat.headgear).toBe("ears");
    expect(MOTIFS.crown.headgear).toBe("crown");
    expect(MOTIFS.horns.headgear).toBe("horns");
  });

  it("R503 the circuits and waves motifs draw a pattern as well", () => {
    const datacenter = faceSpec(def("classicplus-078"), false);
    const bare = artSpec("classicplus-078", "field-spell", "landscape", false, null);
    expect(datacenter.ridges.length).toBe(bare.ridges.length + 1);
    const deep = faceSpec(def("classic-090"), false);
    const shallow = artSpec("classic-090", "quickdraw", "landscape", false, null);
    expect(deep.motif).toBe("waves");
    expect(deep.ridges.length).toBe(shallow.ridges.length + 1);
  });
});

describe("R503 one distinct picture per catalog entry", () => {
  it("R503 every catalog entry draws a distinct picture on each face, and no face draws another's", () => {
    const seen = new Map<string, string>();
    for (const card of DEFS) {
      for (const radiant of [false, true]) {
        const uri = artDataUri(faceSpec(card, radiant));
        const where = `${card.id} ${radiant ? "radiant" : "base"}`;
        expect(seen.get(uri), `${where} draws the same art as ${seen.get(uri) ?? ""}`).toBeUndefined();
        seen.set(uri, where);
      }
    }
    expect(seen.size).toBe(DEFS.length * 2);
  });

  it("R503 a face with its own type draws that type's picture (Blood Moon's Radiant Field Trap)", () => {
    const bloodMoon = def("classicplus-022");
    expect(faceSpec(bloodMoon, false).composition).toBe(compositionFor(bloodMoon.type));
    expect(faceSpec(bloodMoon, true).composition).toBe("sigil");
  });

  it("R503 each set's cards draw their variety from their own salt; Core's is the one it always was", () => {
    expect(varietySalt("core-002")).toBe(varietySalt("t-12"));
    expect(varietySalt("classic-043")).not.toBe(varietySalt("core-043"));
    expect(varietySalt("classicplus-043")).not.toBe(varietySalt("classic-043"));
    expect(varietySalt("classicplus-012-1")).toBe(varietySalt("classicplus-012"));
  });
});

describe("R503 Core keeps the pictures v0.1 gave it", () => {
  type Recorded = readonly [ArtThemeId, string, number, number];
  const recorded = CORE_V01 as unknown as Readonly<Record<string, Recorded>>;

  it("R503 the fixture covers every Core entry", () => {
    const core = DEFS.filter((card) => card.set === "Core").map((card) => card.id);
    expect(Object.keys(recorded).sort()).toEqual(core.sort());
  });

  it("R503 every Core card keeps its theme, layout, backdrop and sky, and its emblem unless its motif became it", () => {
    for (const [id, [theme, picture]] of Object.entries(recorded)) {
      const spec = faceSpec(def(id), false);
      const [layout, backdrop, sky, emblem] = picture.split("|");
      expect(spec.theme, id).toBe(theme);
      expect(spec.layout, id).toBe(layout);
      expect(spec.backdrop, id).toBe(backdrop);
      expect(spec.skyScheme, id).toBe(sky);
      const worn = spec.motif !== null && PLAIN_THEMES.includes(spec.theme);
      expect(spec.emblem.glyph, id).toBe(worn && spec.motif !== null ? MOTIFS[spec.motif].glyph : emblem);
    }
  });

  it("R503 a Core card whose name calls up no motif draws exactly the picture it drew in v0.1, on both faces", () => {
    let unchanged = 0;
    for (const [id, [, , base, radiant]] of Object.entries(recorded)) {
      const card = def(id);
      if (faceSpec(card, false).motif !== null) continue;
      expect(hashId(artDataUri(faceSpec(card, false))), id).toBe(base);
      expect(hashId(artDataUri(faceSpec(card, true))), id).toBe(radiant);
      unchanged += 1;
    }
    expect(unchanged).toBeGreaterThan(0);
  });
});
