// R3338001: the nine Terminus cards' pictures of their own (#3338). Olivian Expanse, its six Terminus cards and
// Dice of Doom draw the Stinkyscape set's art, each its own kind (stinkyscape.ts `TERMINUS_KINDS`) and no longer
// the shared black hole or a planet; Ceaseless Void draws a composition of its own, a hole in the shape of a giant
// (themes.ts `CARD_COMPOSITIONS`, procedural.ts `voidGiant`), wherever its procedural picture is drawn.

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { CardArt } from "./CardArt.tsx";
import { motifFor } from "./motifs.ts";
import { artSeedOf } from "./pictures.ts";
import { artSpec, LAYOUTS } from "./procedural.ts";
import { stinkyscapeSvg, stinkySubject, TERMINUS_DISK_BACK, TERMINUS_KINDS } from "./stinkyscape.ts";
import { proceduralArtUri } from "./svg.ts";
import { CARD_COMPOSITIONS, compositionFor, compositionOf, themeFor } from "./themes.ts";

afterEach(cleanup);

const SVG_PREFIX = "data:image/svg+xml,";
const VOID_ID = "core-100";
const DICE_ID = "stinkyscape-154";
/** Olivian Expanse, its six Terminus cards and Dice of Doom, the issue's table in its order. */
const STINKY_IDS = [
  "stinkyscape-004",
  "stinkyscape-004-1",
  "stinkyscape-004-2",
  "stinkyscape-004-3",
  "stinkyscape-004-4",
  "stinkyscape-004-5",
  "stinkyscape-004-6",
  DICE_ID,
] as const;
const ALL_IDS = [...STINKY_IDS, VOID_ID] as const;

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`the catalog has no ${id}`);
  return found;
}

/** The Void's procedural picture as CardArt draws it, grown from `seedId` (the card's id, or a look's). */
function voidUri(radiant: boolean, seedId: string = VOID_ID): string {
  const card = def(VOID_ID);
  const theme = themeFor(card.tags, card.type);
  return proceduralArtUri(seedId, theme, compositionOf(card.id, card.type), radiant, motifFor(card.name, theme));
}

/** A card's picture, unencoded, as its face draws it: the set's own painter, or the procedural one. */
function pictureOf(id: string, radiant: boolean, seedId: string = id): string {
  const card = def(id);
  if (id === VOID_ID) return decodeURIComponent(voidUri(radiant, seedId).slice(SVG_PREFIX.length));
  return stinkyscapeSvg(card.id, card.name, card.tags, card.type, radiant, null, seedId);
}

describe("R3338001: the Terminus cards' pictures of their own", () => {
  it("R3338001 nine pictures of their own: 18 distinct faces, each Radiant face apart from its base, none drawing the black hole's disk", () => {
    const faces = new Map<string, string>();
    for (const id of ALL_IDS) {
      const base = pictureOf(id, false);
      const radiant = pictureOf(id, true);
      expect(radiant, id).not.toBe(base);
      for (const [face, svg] of [["base", base], ["Radiant", radiant]] as const) {
        const where = `${id} ${face}`;
        expect(svg.startsWith('<svg viewBox="0 0 100 100"'), where).toBe(true);
        expect(svg, where).not.toMatch(/NaN|undefined|Infinity|<text|<title|href|<image/);
        expect(svg, where).not.toContain(TERMINUS_DISK_BACK);
        expect(faces.get(svg), `${where} draws ${faces.get(svg) ?? ""}'s picture`).toBeUndefined();
        faces.set(svg, where);
      }
    }
    expect(faces.size).toBe(ALL_IDS.length * 2);
    // The disk is the shared black hole's, which a Terminus card the table does not list still draws.
    expect(stinkyscapeSvg("stinkyscape-999", "Unlisted End", ["Terminus"], "Spell", false)).toContain(TERMINUS_DISK_BACK);
  });

  it("R3338001 each Stinkyscape card draws a kind of its own, found by its id, and any other Terminus card the shared black hole", () => {
    const kinds = STINKY_IDS.map((id) => {
      const card = def(id);
      const subject = stinkySubject(card.id, card.name, card.tags, card.type);
      expect(subject.family, id).toBe("terminus");
      expect(subject.kind, id).toBe(TERMINUS_KINDS[id]);
      return subject.kind;
    });
    expect(kinds).toEqual(["expanse", "lamp", "exam", "edge", "crown", "destination", "crunch", "d100"]);
    expect(Object.keys(TERMINUS_KINDS).sort()).toEqual([...STINKY_IDS].sort());
    // Dice of Doom prints no Terminus tag: its id is what finds it.
    expect(def(DICE_ID).tags).not.toContain("Terminus");
    expect(stinkySubject("stinkyscape-999", "Unlisted End", ["Terminus"], "Spell")).toMatchObject({ family: "terminus", kind: "hole" });
  });

  it("R3338001 Ceaseless Void draws the void composition, a hole in the shape of a giant, wherever its picture is drawn", () => {
    expect(compositionOf(VOID_ID, "Unit")).toBe("void");
    expect(CARD_COMPOSITIONS).toEqual({ [VOID_ID]: "void" });
    // Every other card keeps its type's composition.
    for (const card of Object.values(CATALOG).filter((each) => each.id !== VOID_ID)) {
      expect(compositionOf(card.id, card.type), card.id).toBe(compositionFor(card.type));
    }
    // Its face draws it.
    const card = def(VOID_ID);
    const { container } = render(<CardArt defId={card.id} radiant={false} tags={card.tags} type={card.type} name={card.name} shape="portrait" />);
    expect(container.querySelector<HTMLElement>(".cf-art")?.style.backgroundImage).toContain(voidUri(false));
    expect(voidUri(false)).not.toBe(proceduralArtUri(card.id, themeFor(card.tags, card.type), compositionFor(card.type), false, motifFor(card.name, themeFor(card.tags, card.type))));
    // One of the giant's poses, the Radiant face gilding the same geometry.
    const theme = themeFor(card.tags, card.type);
    const base = artSpec(card.id, theme, "void", false, motifFor(card.name, theme));
    const radiant = artSpec(card.id, theme, "void", true, motifFor(card.name, theme));
    expect(LAYOUTS.void).toContain(base.layout);
    expect(radiant.ridges.map((ridge) => ridge.d)).toEqual(base.ridges.map((ridge) => ridge.d));
    expect(radiant.ridges.map((ridge) => ridge.fill)).not.toEqual(base.ridges.map((ridge) => ridge.fill));
  });

  it("R3338001 Alt art lays out each picture's subject anew", () => {
    for (const id of STINKY_IDS) {
      const card = def(id);
      const subject = (seedId: string): string => stinkyscapeSvg(card.id, card.name, card.tags, card.type, false, null, seedId, "subject");
      expect(subject(artSeedOf(id, "alt")), id).not.toBe(subject(id));
    }
    const card = def(VOID_ID);
    const theme = themeFor(card.tags, card.type);
    const giant = (seedId: string): string[] =>
      artSpec(seedId, theme, "void", false, motifFor(card.name, theme))
        .ridges.filter((ridge) => ridge.role === "subject")
        .map((ridge) => ridge.d);
    expect(giant(artSeedOf(VOID_ID, "alt"))).not.toEqual(giant(VOID_ID));
  });
});
