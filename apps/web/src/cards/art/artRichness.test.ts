// R3331001: the detail floor (richness.ts). Every catalog entry's base picture, tokens included, is
// measured as its face draws it: the elements it draws, and for a scene the layers and the kit parts it
// uses. A card under its rarity's floor must be in `ART_DETAIL_SHORT`, the ratchet each set's picture pass
// shrinks, and the list may name no card that clears its floor; no picture, base or Radiant, is longer
// than `ART_DETAIL_MAX_BYTES`. The Glitch is left out: its window draws no picture (glitch.css).

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { GLITCH_DEF_ID } from "@jackioh/engine/config";
import type { CardDef, Rarity } from "@jackioh/shared";

import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import SHORT from "./fixtures/art-detail-short.json";
import {
  ART_DETAIL_MAX_BYTES,
  ART_DETAIL_MIN,
  ART_DETAIL_MIN_COMMON,
  ART_DETAIL_MIN_EPIC,
  ART_DETAIL_MIN_LEGENDARY,
  ART_DETAIL_MIN_MYTHIC,
  ART_DETAIL_MIN_RARE,
  ART_DETAIL_MIN_TOKEN,
  ART_DETAIL_SHORT,
  ART_DETAIL_SHORT_MAX,
  artElements,
} from "./richness.ts";
import { composeScene, SCENE_LAYERS, sceneRecipeOf, sceneUse } from "./scene.ts";

const SVG_PREFIX = "data:image/svg+xml,";
/** Every entry whose window draws a picture. */
const MEASURED: readonly CardDef[] = Object.values(CATALOG).filter((card) => card.id !== GLITCH_DEF_ID);

type Measure = { elements: number; layers: number; parts: number };

/** A card's base picture, as its face draws it, measured. */
function measure(card: CardDef): Measure {
  const svg = decodeURIComponent(proceduralCardArtUri(card, false).slice(SVG_PREFIX.length));
  const scene = sceneRecipeOf(card.id);
  return { elements: artElements(svg), ...(scene === null ? { layers: 0, parts: 0 } : sceneUse(composeScene(scene, card.id))) };
}

function clears(card: CardDef, measured: Measure): boolean {
  const floor = ART_DETAIL_MIN[card.rarity];
  return measured.elements >= floor.elements && measured.layers >= floor.layers;
}

const MEASURES: ReadonlyMap<string, Measure> = new Map(MEASURED.map((card) => [card.id, measure(card)]));

function measured(card: CardDef): Measure {
  const found = MEASURES.get(card.id);
  if (found === undefined) throw new Error(`${card.id} was not measured`);
  return found;
}

describe("R3331001 the detail floor", () => {
  it("R3331001 every catalog entry but the Glitch, tokens included, is measured", () => {
    expect(MEASURED.length).toBe(Object.keys(CATALOG).length - 1);
    expect(MEASURED.some((card) => card.token)).toBe(true);
    expect(MEASURED.some((card) => card.rarity === "Token")).toBe(true);
    for (const card of MEASURED) expect(measured(card).elements, card.id).toBeGreaterThan(0);
    // A scene's layers and parts are counted from the scene it composes.
    expect(measured(CATALOG["core-100"] as CardDef).layers).toBe(SCENE_LAYERS.length);
    expect(measured(CATALOG["core-100"] as CardDef).parts).toBeGreaterThan(1);
  });

  it("R3331001 every card under its floor is in ART_DETAIL_SHORT", () => {
    const missing = MEASURED.filter((card) => !clears(card, measured(card)) && !ART_DETAIL_SHORT.has(card.id)).map((card) => card.id).sort();
    expect(missing, `under their floor and not in fixtures/art-detail-short.json: ${JSON.stringify(missing)}`).toEqual([]);
  });

  it("R3331001 ART_DETAIL_SHORT names only catalog cards still under their floor, sorted, exactly ART_DETAIL_SHORT_MAX of them", () => {
    const listed: readonly string[] = SHORT;
    expect(listed, "sorted").toEqual([...listed].sort());
    expect(ART_DETAIL_SHORT.size, "each once").toBe(listed.length);
    expect(listed.length, "ART_DETAIL_SHORT_MAX is the list's length: lower it with the cards a pass takes off").toBe(ART_DETAIL_SHORT_MAX);
    const cleared = listed.filter((id) => {
      const card = CATALOG[id];
      return card === undefined || card.id === GLITCH_DEF_ID || clears(card, measured(card));
    });
    expect(cleared, `take these off fixtures/art-detail-short.json: ${JSON.stringify(cleared)}`).toEqual([]);
  });

  it("R3331001 the floors rise with rarity and Legendary and Mythic use every layer", () => {
    const ladder = [ART_DETAIL_MIN_COMMON, ART_DETAIL_MIN_RARE, ART_DETAIL_MIN_EPIC, ART_DETAIL_MIN_LEGENDARY, ART_DETAIL_MIN_MYTHIC];
    expect(ladder.map((floor) => floor.elements)).toEqual([30, 40, 60, 90, 140]);
    ladder.slice(1).forEach((floor, k) => expect(floor.elements).toBeGreaterThan(ladder[k]?.elements ?? Infinity));
    expect(ART_DETAIL_MIN_LEGENDARY.layers).toBe(SCENE_LAYERS.length);
    expect(ART_DETAIL_MIN_MYTHIC.layers).toBe(SCENE_LAYERS.length);
    expect(ART_DETAIL_MIN_TOKEN).toEqual(ART_DETAIL_MIN_COMMON);
    const rarities: readonly Rarity[] = ["Common", "Rare", "Epic", "Legendary", "Mythic", "Token"];
    expect(Object.keys(ART_DETAIL_MIN).sort()).toEqual([...rarities].sort());
  });

  it("R3331001 every picture, base and Radiant, stays under ART_DETAIL_MAX_BYTES", () => {
    let longest = 0;
    for (const card of MEASURED) {
      for (const radiant of [false, true]) {
        const length = proceduralCardArtUri(card, radiant).length;
        expect(length, `${card.id} ${radiant ? "radiant" : "base"}`).toBeLessThanOrEqual(ART_DETAIL_MAX_BYTES);
        longest = Math.max(longest, length);
      }
    }
    expect(longest).toBeGreaterThan(0);
  });
});
