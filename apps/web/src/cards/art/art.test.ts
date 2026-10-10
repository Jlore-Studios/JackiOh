// Polish 6, slice A: the procedural art module, tested pure (docs/polish/6-cards.md, Surface A,
// behaviours B1–B5). Nothing here renders; `CardArt.test.tsx` covers the DOM (B6).
// Where the doc names a published algorithm (FNV-1a, mulberry32) its reference outputs are pinned,
// since a wrong hash passes any test that checks no vector.

import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { GLITCH_DEF_ID } from "@jackioh/engine/config";
import type { CardDef, CardType, Tag } from "@jackioh/shared";

import { BACKDROPS, LAYOUTS, SKY_SCHEMES } from "./procedural.ts";
import {
  ART_MANIFEST,
  artDataUri,
  artSpec,
  artUrl,
  compositionFor,
  hashId,
  motifFor,
  seededRandom,
  themeFor,
  type ArtManifest,
  type ArtSpec,
  type ArtThemeId,
  type Composition,
} from "./index.ts";

/** Every card that draws a procedural picture: Glitch's window is blank, its blob drawn by glitch.css. */
const DEFS: readonly CardDef[] = Object.values(CATALOG).filter((card) => card.id !== GLITCH_DEF_ID);
const SVG_PREFIX = "data:image/svg+xml,";
/** B40: a theme-and-composition group this large draws every layout of its composition. */
const GROUP_USES_EVERY_LAYOUT = 8;

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`the catalog has no ${id}`);
  return found;
}

/** A card's two pictures as its face draws them: its theme, its composition and its name's motif (R503). */
function specsOf(card: CardDef): { base: ArtSpec; radiant: ArtSpec } {
  const theme = themeFor(card.tags, card.type);
  const composition = compositionFor(card.type);
  const motif = motifFor(card.name, theme);
  return {
    base: artSpec(card.id, theme, composition, false, motif),
    radiant: artSpec(card.id, theme, composition, true, motif),
  };
}

/** The SVG markup inside a `data:image/svg+xml,` URI, whether or not it was percent-encoded. */
function svgOf(uri: string): string {
  return decodeURIComponent(uri.slice(SVG_PREFIX.length));
}

afterEach(() => {
  vi.restoreAllMocks();
});

// B1

describe("B1: artSpec is pure and deterministic", () => {
  it("B1 equal arguments give deep-equal specs for every catalog card, and Math.random is never called", () => {
    const random = vi.spyOn(Math, "random");

    for (const card of DEFS) {
      const theme = themeFor(card.tags, card.type);
      const composition = compositionFor(card.type);
      const motif = motifFor(card.name, theme);
      for (const radiant of [false, true]) {
        const first = artSpec(card.id, theme, composition, radiant, motif);
        const second = artSpec(card.id, theme, composition, radiant, motif);
        expect(second, `${card.id} radiant=${String(radiant)}`).toEqual(first);
        expect(artDataUri(second)).toBe(artDataUri(first));
      }
    }

    expect(random).not.toHaveBeenCalled();
  });

  it("B1 a spec does not depend on what was computed before it (no hidden state)", () => {
    const card = def("core-002");
    const before = specsOf(card);
    const beforeUri = artDataUri(before.base);

    // Churn every other id through both functions, then ask again.
    for (const other of DEFS) {
      const specs = specsOf(other);
      artDataUri(specs.base);
      artDataUri(specs.radiant);
    }

    const after = specsOf(card);
    expect(after).toEqual(before);
    expect(artDataUri(after.base)).toBe(beforeUri);
  });

  it("B1 hashId is 32-bit FNV-1a: the published reference vectors", () => {
    expect(hashId("")).toBe(0x811c9dc5);
    expect(hashId("a")).toBe(0xe40c292c);
    expect(hashId("foobar")).toBe(0xbf9cf968);
  });

  it("B1 hashId is an unsigned 32-bit integer for every catalog id and for odd ids", () => {
    const ids = [...DEFS.map((card) => card.id), "", "t-1", "t-999999", "x".repeat(500)];
    for (const id of ids) {
      const hash = hashId(id);
      expect(Number.isInteger(hash), id).toBe(true);
      expect(hash, id).toBeGreaterThanOrEqual(0);
      expect(hash, id).toBeLessThan(2 ** 32);
      expect(hashId(id), id).toBe(hash);
    }
  });

  it("B1 seededRandom is mulberry32: the reference outputs, and the same seed gives the same stream", () => {
    const one = seededRandom(1);
    expect([one(), one(), one()]).toEqual([0.6270739405881613, 0.002735721180215478, 0.5274470399599522]);
    const zero = seededRandom(0);
    expect(zero()).toBe(0.26642920868471265);

    const a = seededRandom(42);
    const b = seededRandom(42);
    const xs = Array.from({ length: 200 }, () => a());
    const ys = Array.from({ length: 200 }, () => b());
    expect(ys).toEqual(xs);
  });

  it("B1 seededRandom stays in [0, 1) across the whole unsigned seed range", () => {
    for (const seed of [0, 1, 2 ** 31 - 1, 2 ** 31, 2 ** 32 - 1, hashId("core-100")]) {
      const next = seededRandom(seed);
      for (let i = 0; i < 500; i += 1) {
        const x = next();
        expect(x, `seed ${seed}`).toBeGreaterThanOrEqual(0);
        expect(x, `seed ${seed}`).toBeLessThan(1);
      }
    }
    // Two different seeds are two different streams.
    const s1 = seededRandom(7);
    const s2 = seededRandom(8);
    expect(Array.from({ length: 5 }, () => s1())).not.toEqual(Array.from({ length: 5 }, () => s2()));
  });
});

// B2

describe("B2: the radiant variant keeps the base geometry and gilds it", () => {
  it("B2 base and radiant share composition, every ridge path, and the emblem's glyph and position", () => {
    for (const card of DEFS) {
      const { base, radiant } = specsOf(card);
      expect(radiant.composition, card.id).toBe(base.composition);
      expect(radiant.ridges.map((ridge) => ridge.d), card.id).toEqual(base.ridges.map((ridge) => ridge.d));
      expect(radiant.emblem.glyph, card.id).toBe(base.emblem.glyph);
      expect(radiant.emblem.x, card.id).toBe(base.emblem.x);
      expect(radiant.emblem.y, card.id).toBe(base.emblem.y);
      // The per-card features are the card's, not the variant's.
      expect(radiant.layout, card.id).toBe(base.layout);
      expect(radiant.backdrop, card.id).toBe(base.backdrop);
      expect(radiant.skyScheme, card.id).toBe(base.skyScheme);
      expect(radiant.accent?.glyph ?? null, card.id).toBe(base.accent?.glyph ?? null);
      expect(radiant.accent?.x ?? null, card.id).toBe(base.accent?.x ?? null);
      // The motif and where its glyphs stand are the card's too (R503).
      expect(radiant.motif, card.id).toBe(base.motif);
      expect(radiant.motifGlyphs.map((placed) => [placed.glyph, placed.x, placed.y, placed.size]), card.id).toEqual(
        base.motifGlyphs.map((placed) => [placed.glyph, placed.x, placed.y, placed.size]),
      );
    }
  });

  it("B2 the specs name their variant and echo the theme and composition they were drawn from", () => {
    for (const card of DEFS) {
      const theme = themeFor(card.tags, card.type);
      const composition = compositionFor(card.type);
      const { base, radiant } = specsOf(card);
      expect(base.variant, card.id).toBe("base");
      expect(radiant.variant, card.id).toBe("radiant");
      expect(base.theme, card.id).toBe(theme);
      expect(radiant.theme, card.id).toBe(theme);
      expect(base.composition, card.id).toBe(composition);
    }
  });

  it("B2 the sky differs between the variants", () => {
    for (const card of DEFS) {
      const { base, radiant } = specsOf(card);
      expect(radiant.sky, card.id).not.toEqual(base.sky);
    }
  });

  it("B2 the base variant has no rays; the radiant variant has 7 to 11", () => {
    for (const card of DEFS) {
      const { base, radiant } = specsOf(card);
      expect(base.rays, card.id).toHaveLength(0);
      expect(radiant.rays.length, card.id).toBeGreaterThanOrEqual(7);
      expect(radiant.rays.length, card.id).toBeLessThanOrEqual(11);
    }
  });

  it("B2 the two variants of one card never draw the same SVG", () => {
    for (const card of DEFS) {
      const { base, radiant } = specsOf(card);
      expect(artDataUri(radiant), card.id).not.toBe(artDataUri(base));
    }
  });
});

// A theme only sets the palette and the kind of picture: within a theme every card draws its own
// layout, backdrop, sky, emblem and name's motif (R503), and no two share all five, so neighbours
// in a tribe always differ at a glance. The layout rule reads per theme and composition, since the
// catalog mixes compositions inside a theme (a Human Field Trap, a Pancake Field Spell).

describe("per-card variety within a theme", () => {
  function signature(spec: ArtSpec): string {
    return `${spec.layout}|${spec.backdrop}|${spec.skyScheme}|${spec.emblem.glyph}|${spec.motif ?? "-"}`;
  }

  it("R503 no two catalog cards of one theme share their layout, backdrop, sky, emblem and motif", () => {
    const seen = new Map<string, string>();
    for (const card of DEFS) {
      const { base } = specsOf(card);
      const key = `${base.theme}|${signature(base)}`;
      const other = seen.get(key);
      expect(other, `${card.id} and ${other ?? "?"} draw the same ${key}`).toBeUndefined();
      seen.set(key, card.id);
    }
  });

  it("B40 each layout, backdrop and sky scheme is one of the composition's own, and every theme of four or more cards spreads over them", () => {
    const byTheme = new Map<ArtThemeId, ArtSpec[]>();
    for (const card of DEFS) {
      const { base } = specsOf(card);
      expect(LAYOUTS[base.composition], card.id).toContain(base.layout);
      expect(BACKDROPS, card.id).toContain(base.backdrop);
      expect(SKY_SCHEMES, card.id).toContain(base.skyScheme);
      byTheme.set(base.theme, [...(byTheme.get(base.theme) ?? []), base]);
    }
    for (const [theme, specs] of byTheme) {
      if (specs.length < 4) continue;
      expect(new Set(specs.map((spec) => spec.layout)).size, `${theme} layouts`).toBeGreaterThanOrEqual(2);
      expect(new Set(specs.map((spec) => spec.backdrop)).size, `${theme} backdrops`).toBeGreaterThanOrEqual(3);
      expect(new Set(specs.map((spec) => spec.skyScheme)).size, `${theme} skies`).toBeGreaterThanOrEqual(2);
    }
  });

  it("B40 R503 every theme-and-composition group of eight or more cards uses every layout of its composition", () => {
    const groups = new Map<string, ArtSpec[]>();
    for (const card of DEFS) {
      const { base } = specsOf(card);
      const key = `${base.theme}|${base.composition}`;
      groups.set(key, [...(groups.get(key) ?? []), base]);
    }
    const large = [...groups].filter(([, specs]) => specs.length >= GROUP_USES_EVERY_LAYOUT);
    // The four largest Core themes are among them, and so is the Book family's.
    expect(large.map(([key]) => key)).toEqual(
      expect.arrayContaining(["human|figure", "spell|burst", "unit|figure", "field-spell|landscape", "book|burst"]),
    );
    for (const [key, specs] of large) {
      const composition = specs[0]?.composition ?? "figure";
      expect(new Set(specs.map((spec) => spec.layout)), key).toEqual(new Set(LAYOUTS[composition]));
    }
  });

  it("the backdrop is drawn: a card with one has more layers than its composition alone draws", () => {
    for (const card of DEFS) {
      const { base } = specsOf(card);
      const svg = svgOf(artDataUri(base));
      if (base.accent !== null) expect(svg, card.id).toContain('opacity="0.8"');
      if (base.backdrop === "none") continue;
      expect(base.ridges.length, card.id).toBeGreaterThan(1);
    }
  });
});

// B3

describe("B3: artDataUri draws a clean, self-contained SVG for every card", () => {
  it("B3 every catalog id, both variants: a data:image/svg+xml URI with no NaN, undefined, <text or <title", () => {
    for (const card of DEFS) {
      for (const spec of Object.values(specsOf(card))) {
        const uri = artDataUri(spec);
        const where = `${card.id} ${spec.variant}`;
        expect(uri.startsWith(SVG_PREFIX), where).toBe(true);
        for (const text of [uri, svgOf(uri)]) {
          expect(text, where).not.toContain("NaN");
          expect(text, where).not.toContain("undefined");
          expect(text, where).not.toContain("<text");
          expect(text, where).not.toContain("<title");
        }
      }
    }
  });

  it("B3 the markup is one <svg viewBox=\"0 0 100 100\"> with no external reference", () => {
    for (const card of DEFS) {
      for (const spec of Object.values(specsOf(card))) {
        const svg = svgOf(artDataUri(spec));
        const where = `${card.id} ${spec.variant}`;
        expect(svg.match(/<svg[\s>]/g) ?? [], where).toHaveLength(1);
        expect(svg, where).toMatch(/viewBox=["']0 0 100 100["']/);
        // Namespace declarations are identifiers, not fetches; anything else with a scheme is.
        const withoutNamespaces = svg.replace(/\sxmlns(?::[\w-]+)?=["'][^"']*["']/g, "");
        expect(withoutNamespaces, where).not.toMatch(/https?:|file:|\/\/|data:/);
      }
    }
  });

  it("B3 no two catalog ids produce the same base URI", () => {
    const seen = new Map<string, string>();
    for (const card of DEFS) {
      const uri = artDataUri(specsOf(card).base);
      const clash = seen.get(uri);
      expect(clash, `${card.id} draws the same base art as ${clash ?? ""}`).toBeUndefined();
      seen.set(uri, card.id);
    }
    expect(seen.size).toBe(DEFS.length);
  });

  it("B3 transient and odd ids (t-<n>, empty, very long, non-ASCII) still draw clean SVG", () => {
    const ids = ["t-1", "t-2", "t-123456", "", "x".repeat(300), "carte-été-✨"];
    const types: CardType[] = ["Unit", "Spell", "Field Spell", "Trap", "Field Trap"];
    for (const id of ids) {
      for (const type of types) {
        for (const radiant of [false, true]) {
          const uri = artDataUri(artSpec(id, themeFor([], type), compositionFor(type), radiant, motifFor(id)));
          const where = `${JSON.stringify(id)} ${type} radiant=${String(radiant)}`;
          expect(uri.startsWith(SVG_PREFIX), where).toBe(true);
          const svg = svgOf(uri);
          expect(svg, where).not.toContain("NaN");
          expect(svg, where).not.toContain("undefined");
          expect(svg, where).not.toContain("Infinity");
          expect(svg, where).not.toContain("<text");
        }
      }
    }
  });
});

// B4

/** Surface A's priority list, first match wins, with v0.2.0's three families in their places (R503). */
const PRIORITY: readonly (readonly [Tag, ArtThemeId])[] = [
  ["Call to Chaos", "chaos"],
  ["AI", "ai"],
  ["CN", "cn"],
  ["KY", "ky"],
  ["Book", "book"],
  ["Felinor", "felinor"],
  ["Pancake", "pancake"],
  ["Fruit", "fruit"],
  ["Quickdraw", "quickdraw"],
  ["Human", "human"],
];

const TYPE_THEME: readonly (readonly [CardType, ArtThemeId, Composition])[] = [
  ["Unit", "unit", "figure"],
  ["Spell", "spell", "burst"],
  ["Field Spell", "field-spell", "landscape"],
  ["Trap", "trap", "sigil"],
  ["Field Trap", "field-trap", "sigil"],
];

describe("B4: themeFor and compositionFor", () => {
  it("B4 each themed tag on its own gives its theme, whatever the type", () => {
    for (const [tag, theme] of PRIORITY) {
      for (const [type] of TYPE_THEME) {
        expect(themeFor([tag], type), `${tag} on a ${type}`).toBe(theme);
      }
    }
  });

  it("B4 the priority order decides between two themed tags, in either tag order", () => {
    PRIORITY.forEach(([winner, theme], i) => {
      for (const [loser] of PRIORITY.slice(i + 1)) {
        expect(themeFor([winner, loser], "Unit"), `${winner} + ${loser}`).toBe(theme);
        expect(themeFor([loser, winner], "Unit"), `${loser} + ${winner}`).toBe(theme);
      }
    });
  });

  it("B4 a themed tag beats Token; Token alone gives token on every type", () => {
    for (const [tag, theme] of PRIORITY) {
      expect(themeFor([tag, "Token"], "Unit"), `${tag} + Token`).toBe(theme);
      expect(themeFor(["Token", tag], "Spell"), `Token + ${tag}`).toBe(theme);
    }
    for (const [type] of TYPE_THEME) {
      expect(themeFor(["Token"], type), `Token on a ${type}`).toBe("token");
    }
  });

  it("B4 with no tags the type decides the theme", () => {
    for (const [type, theme] of TYPE_THEME) {
      expect(themeFor([], type), type).toBe(theme);
    }
  });

  it("B4 compositionFor: Unit figure, Spell burst, Field Spell landscape, Trap and Field Trap sigil", () => {
    for (const [type, , composition] of TYPE_THEME) {
      expect(compositionFor(type), type).toBe(composition);
    }
  });

  it("B4 real catalog cards land on the theme their tags and type imply", () => {
    const expected: readonly (readonly [string, ArtThemeId])[] = [
      ["core-095", "chaos"],
      ["core-090", "cn"],
      ["core-090-1", "cn"],
      ["core-031", "ky"],
      ["core-051-1", "ky"],
      ["core-012", "felinor"],
      ["core-t-felinor", "felinor"],
      ["core-047", "fruit"],
      ["core-065", "quickdraw"],
      ["core-098", "quickdraw"],
      ["core-002", "human"],
      ["core-t-rush", "token"],
      ["core-065-1", "token"],
      ["core-093-1", "token"],
      ["core-007", "unit"],
      ["core-005", "spell"],
      ["core-006", "field-spell"],
      ["core-041", "trap"],
      ["core-018", "field-trap"],
      // R503: the families, their tokens reading as the family and not as Token, and the mixes.
      ["classic-003", "book"],
      ["classicplus-054", "book"],
      ["classicplus-012", "pancake"],
      ["classicplus-012-1", "pancake"],
      ["classicplus-013", "pancake"],
      ["classicplus-t-ai-01", "ai"],
      ["classicplus-t-ai-08", "ai"],
      ["classicplus-062", "ky"],
      ["classicplus-076", "cn"],
      ["classicplus-076-1", "cn"],
      ["classicplus-019-1", "token"],
      ["classic-038", "human"],
      ["classicplus-048", "unit"],
    ];
    for (const [id, theme] of expected) {
      const card = def(id);
      expect(themeFor(card.tags, card.type), id).toBe(theme);
    }
  });
});

// B5

describe("B5: artUrl and the manifest", () => {
  const base = import.meta.env.BASE_URL;

  it("B5 an id the manifest does not list gives null for both variants", () => {
    const empty: ArtManifest = {};
    expect(artUrl("core-002", false, empty)).toBeNull();
    expect(artUrl("core-002", true, empty)).toBeNull();
  });

  it("B5 an entry for another id does not leak onto this one", () => {
    const manifest: ArtManifest = { "core-003": { base: true, radiant: true } };
    expect(artUrl("core-002", false, manifest)).toBeNull();
    expect(artUrl("core-002", true, manifest)).toBeNull();
    // Nor does a prefix: core-00 is not core-002.
    expect(artUrl("core-00", false, { "core-002": { base: true } })).toBeNull();
  });

  it("B5 a listed base gives BASE_URL + art/<id>.webp with no tint", () => {
    const manifest: ArtManifest = { "core-002": { base: true } };
    expect(artUrl("core-002", false, manifest)).toEqual({ src: `${base}art/core-002.webp`, tint: false });
  });

  it("B5 a listed radiant file gives BASE_URL + art/<id>-radiant.webp with no tint", () => {
    const both: ArtManifest = { "core-002": { base: true, radiant: true } };
    expect(artUrl("core-002", true, both)).toEqual({ src: `${base}art/core-002-radiant.webp`, tint: false });
    expect(artUrl("core-002", false, both)).toEqual({ src: `${base}art/core-002.webp`, tint: false });

    const radiantOnly: ArtManifest = { "core-002": { radiant: true } };
    expect(artUrl("core-002", true, radiantOnly)).toEqual({ src: `${base}art/core-002-radiant.webp`, tint: false });
  });

  it("B5 a radiant request with only the base listed gives the base src with tint", () => {
    const manifest: ArtManifest = { "core-002": { base: true } };
    expect(artUrl("core-002", true, manifest)).toEqual({ src: `${base}art/core-002.webp`, tint: true });
  });

  it("B5 ART_MANIFEST ships empty, so the default manifest gives null for every catalog id", () => {
    expect(Object.keys(ART_MANIFEST)).toHaveLength(0);
    for (const card of DEFS) {
      expect(artUrl(card.id, false), card.id).toBeNull();
      expect(artUrl(card.id, true), card.id).toBeNull();
    }
  });
});
