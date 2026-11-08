// R3015002 (#3015): one picture per variant. Every level of a variant draws its first level's picture (versions.ts
// `artPictureOf`, `artPartOf`; the Default's the normal one, the Sketch's full art the sketch), and a level adds only the
// full-art frame, its picture's own motion (R2715001) and a signature: a signed level is its variant's full art, moving,
// signed, with no picture of its own (R2949000's Signature look is gone). R2725004 holds at every level: a Radiant face
// keeps its marks. R3343001 (#3343): a Default level draws the card's default look, a Mythic's Full art. The ladders
// themselves are versions.test.ts's (R3015000); the pixels are e2e's cypress/component/variant-ladders.cy.tsx.

import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { CardFace } from "./CardFace.tsx";
import { resetCosmeticsForTests, writeCardLevel } from "./cosmetics.ts";
import { faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import { DEFAULT_VARIANT, artPartOf, artPictureOf, defaultLookOf, isFullBleed, ladderOf, levelsOn, lookOfLevel, variantsOn } from "./versions.ts";

/**
 * Three cards offered many variants, one of each painter: Transmogulate (procedural, a Legendary Spell), xxFreshJack
 * (Stinkyscape's, a Mythic Unit, Out of frame among its variants) and Goetzmann Game Design (Training Grounds', one of
 * the 21 full-art cards, its own motion).
 */
const CARDS = ["core-083", "stinkyscape-109", "traininggrounds-040"] as const;

/**
 * The levels of `variant` `defId` wears, lowest first: the Default's with its first level, the normal face (null).
 * R3343001: a variant whose first level is the card's default look, offered nowhere there (R3343000), starts at the
 * Default's still level (null), which draws that look.
 */
function levelsOf(defId: string, variant: string): (string | null)[] {
  const levels = levelsOn(defId, variant);
  return variant === DEFAULT_VARIANT || ladderOf(variant)[0] === defaultLookOf(defId) ? [null, ...levels] : [...levels];
}

/** The level whose picture every level of `id`'s variant draws: its variant's first, the normal face for the Default. */
function firstOf(variant: string): string | null {
  return variant === DEFAULT_VARIANT ? null : (ladderOf(variant)[0] ?? null);
}

function faceIn(id: string, radiant: boolean): HTMLElement {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  const { container } = render(<CardFace face={faceModel({ defId: id, def, radiant })} layout="full" worn />);
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error(`no face for ${id}`);
  return cf;
}

/** The art window's picture, decoded: what its CSS background draws. */
function pictureOf(cf: HTMLElement): string {
  return decodeURIComponent(cf.querySelector<HTMLElement>(".cf-art")?.style.backgroundImage ?? "");
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
});

afterEach(() => {
  cleanup();
  resetCosmeticsForTests();
  localStorage.clear();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  __resetSettingsForTests();
});

describe("R3015002 one picture per variant", () => {
  it("R3015002 every level of a variant draws its first level's picture", () => {
    // Over every card and every level of every variant it is offered: its first level's picture and part.
    const collectible = Object.values(CATALOG).filter((def) => !def.token && !def.tags.includes("Token"));
    let levels = 0;
    for (const def of collectible) {
      for (const variant of variantsOn(def.id)) {
        const first = firstOf(variant);
        for (const level of levelsOf(def.id, variant)) {
          const where = `${def.id} ${String(level)}`;
          expect(artPictureOf(level, def.set), where).toBe(artPictureOf(first, def.set));
          expect(artPartOf(level), where).toBe(artPartOf(first));
          levels += 1;
        }
      }
    }
    expect(levels).toBeGreaterThan(collectible.length * 2);
    // The Sketch's full art draws the sketch, Alt art's full arts Alt art's picture, and Full art's levels the normal one.
    for (const id of ladderOf("sketch")) expect(artPictureOf(id), id).toBe("sketch");
    for (const id of ladderOf("alt-art")) expect(artPictureOf(id), id).toBe("alt");
    for (const id of ladderOf("full-art")) expect(artPictureOf(id), id).toBe("normal");
  });

  for (const id of CARDS) {
    it(`R3015002 ${id}'s levels, base and Radiant, draw their variant's one picture and differ only in frame, motion and signature`, () => {
      const variants = variantsOn(id);
      expect(variants.length, id).toBeGreaterThan(5);
      const set = CATALOG[id]?.set ?? null;
      for (const radiant of [false, true]) {
        for (const variant of variants) {
          const first = firstOf(variant);
          const picture = artPictureOf(first, set);
          // Every still level of the variant draws one picture, and every moving one one moving picture.
          const drawn = { still: new Set<string>(), moving: new Set<string>() };
          for (const level of levelsOf(id, variant)) {
            writeCardLevel(id, level);
            const cf = faceIn(id, radiant);
            const where = `${id} ${radiant ? "Radiant" : "base"} ${String(level)}`;
            const look = lookOfLevel(level);
            const art = cf.querySelector(".cf-art");
            expect(cf.getAttribute("data-level"), where).toBe(level);
            // The variant's picture and part, at every level.
            expect(art?.getAttribute("data-art-picture") ?? "normal", where).toBe(picture);
            if (artPartOf(first) === "subject") expect(cf.querySelector(".cf-art--cutout")?.getAttribute("data-art-part"), where).toBe("subject");
            // What a level adds: the full-art frame, its picture's motion, a signature. R3343001: a Default level draws the
            // card's default look, a Mythic's Full art (the setting on), so its frame is that look's.
            const frame = level === null || variant === DEFAULT_VARIANT ? defaultLookOf(id) : level;
            expect(cf.getAttribute("data-full-art"), where).toBe(isFullBleed(frame) ? "true" : null);
            expect(cf.getAttribute("data-art-motion"), where).toBe(look.animated ? "animated" : null);
            expect(cf.querySelector(".cf-signature") !== null, where).toBe(look.signed);
            // R2725004: a Radiant face keeps its gold frame and its art's Radiant paint at every level.
            expect(cf.getAttribute("data-radiant-face"), where).toBe(radiant ? "true" : null);
            expect(art?.getAttribute("data-art-variant"), where).toBe(radiant ? "radiant" : "base");
            const background = pictureOf(cf);
            expect(background, where).not.toBe("");
            (look.animated ? drawn.moving : drawn.still).add(background);
            cleanup();
          }
          const where = `${id} ${radiant ? "Radiant" : "base"} ${variant}`;
          expect(drawn.still.size, where).toBe(1);
          expect(drawn.moving.size, where).toBe(1);
          // The moving picture is the still one with its motion: apart from it, never another picture.
          expect([...drawn.moving][0], where).not.toBe([...drawn.still][0]);
          expect([...drawn.moving][0], where).toContain("art-m-");
        }
      }
    });
  }
});
