// R3572001: every Legendary and Mythic of the held Stinkyscape+ set drawn from a placeholder scene recipe
// (stinkyscapePlusRecipes.ts), base and Radiant, in Stinkyscape's deep-space look, at or above its rarity's detail
// floor and off the floor's short list, with no part that turns, no gold, none of Stinkyscape's Mythic parts and
// none of an Alt art's. A later brief pass replaces the scenes (ART.md).

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import {
  RADIANT_GOLD_HUE_MAX_DEG,
  RADIANT_GOLD_HUE_MIN_DEG,
  RADIANT_GOLD_LIGHTNESS_MAX,
  RADIANT_GOLD_LIGHTNESS_MIN,
  RADIANT_GOLD_SATURATION_MIN,
} from "../constants.ts";
import { artSpec, type ArtSpec } from "./procedural.ts";
import { ART_DETAIL_MIN, ART_DETAIL_SHORT, artElements } from "./richness.ts";
import { composeScene, SCENE_LAYERS, sceneRecipeOf, sceneUse, type SceneLayer, type SceneRecipe } from "./scene.ts";
import { SCENE_KIT } from "./sceneKit.ts";
import { STINKY_PLUS_RECIPES, STINKY_PLUS_SCENE_CARDS } from "./stinkyscapePlusRecipes.ts";
import { compositionFor, themeFor } from "./themes.ts";

const SVG_PREFIX = "data:image/svg+xml,";

/** Stinkyscape's Mythic-only parts (#1883's): the placeholder scenes use none of them. */
const STINKYSCAPE_MYTHIC_PARTS = ["skiff", "ancient-rod", "drone-battleship", "wire-coil", "blade-fighter", "fused-hull"];


/** The catalog's Legendaries and Mythics of the set, tokens left out, in catalog order. */
const CATALOG_SCENES: readonly CardDef[] = Object.values(CATALOG).filter(
  (card) => card.id.startsWith("stinkyscapeplus-") && !card.token && (card.rarity === "Legendary" || card.rarity === "Mythic"),
);

function recipe(id: string): SceneRecipe {
  const found = sceneRecipeOf(id);
  if (found === null) throw new Error(`no recipe for ${id}`);
  return found;
}

function svgOf(uri: string): string {
  return decodeURIComponent(uri.slice(SVG_PREFIX.length));
}

function specOf(card: CardDef, radiant: boolean): ArtSpec {
  return artSpec(card.id, themeFor(card.tags, card.type), compositionFor(card.type), radiant, null);
}

function hsl(hex: string): { h: number; s: number; l: number } {
  const digits = hex.replace("#", "");
  const [r, g, b] = [0, 2, 4].map((at) => parseInt(digits.slice(at, at + 2), 16) / 255) as [number, number, number];
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  const d = max - min;
  if (d === 0) return { h: 0, s: 0, l };
  const s = d / (1 - Math.abs(2 * l - 1));
  const sector = max === r ? ((g - b) / d + 6) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return { h: sector * 60, s, l };
}

function inRadiantGold(hex: string): boolean {
  const { h, s, l } = hsl(hex);
  return (
    h >= RADIANT_GOLD_HUE_MIN_DEG &&
    h <= RADIANT_GOLD_HUE_MAX_DEG &&
    s >= RADIANT_GOLD_SATURATION_MIN &&
    l >= RADIANT_GOLD_LIGHTNESS_MIN &&
    l <= RADIANT_GOLD_LIGHTNESS_MAX
  );
}

describe("R3572001 Stinkyscape+'s placeholder scenes", () => {
  it("R3572001 the table equals the catalog's Legendaries and Mythics, each in a recipe of its own", () => {
    expect(STINKY_PLUS_SCENE_CARDS).toHaveLength(40);
    expect(STINKY_PLUS_SCENE_CARDS.map(([id]) => id)).toEqual(CATALOG_SCENES.map((card) => card.id));
    for (const [id, , rarity] of STINKY_PLUS_SCENE_CARDS) expect(CATALOG[id]?.rarity, id).toBe(rarity);
    expect(Object.keys(STINKY_PLUS_RECIPES)).toHaveLength(40);
    const seen = new Map<string, string>();
    for (const card of CATALOG_SCENES) {
      const scene = recipe(card.id);
      expect(scene, card.id).toBe(STINKY_PLUS_RECIPES[card.id]);
      expect(scene.family, card.id).toBe("stinkyscape");
      const key = JSON.stringify(scene);
      expect(seen.get(key), `${card.id} draws ${seen.get(key) ?? ""}'s recipe`).toBeUndefined();
      seen.set(key, card.id);
    }
  });

  it("R3572001 every entry clears its rarity's floor with all seven layers and is off ART_DETAIL_SHORT", () => {
    for (const card of CATALOG_SCENES) {
      const floor = ART_DETAIL_MIN[card.rarity];
      const shapes = composeScene(recipe(card.id), card.id);
      expect(artElements(svgOf(proceduralCardArtUri(card, false))), card.id).toBeGreaterThanOrEqual(floor.elements);
      expect(sceneUse(shapes).layers, card.id).toBeGreaterThanOrEqual(floor.layers);
      const layers = new Set<SceneLayer>(shapes.map((shape) => shape.layer));
      for (const layer of SCENE_LAYERS) expect(layers, `${card.id} ${layer}`).toContain(layer);
      expect(ART_DETAIL_SHORT.has(card.id), card.id).toBe(false);
    }
  });

  it("R3572001 base and Radiant draw one geometry, the Radiant face gilded", () => {
    for (const card of CATALOG_SCENES) {
      const shapes = composeScene(recipe(card.id), card.id);
      const base = specOf(card, false);
      const radiant = specOf(card, true);
      expect(base.ridges.map((ridge) => ridge.d), card.id).toEqual(shapes.map((shape) => shape.d));
      expect(radiant.ridges.map((ridge) => ridge.d), card.id).toEqual(base.ridges.map((ridge) => ridge.d));
      expect(radiant.sky, card.id).not.toEqual(base.sky);
    }
  });

  it("R3572001 no scene turns a part, draws a Mythic part of Stinkyscape's, or paints in Radiant's gold window", () => {
    for (const card of CATALOG_SCENES) {
      const scene = recipe(card.id);
      for (const layer of SCENE_LAYERS) {
        for (const placed of scene.layers[layer] ?? []) {
          expect(placed.rotate, `${card.id} ${placed.part}`).toBeUndefined();
          expect(STINKYSCAPE_MYTHIC_PARTS, `${card.id} ${placed.part}`).not.toContain(placed.part);
          expect(Object.hasOwn(SCENE_KIT, placed.part), `${card.id} ${placed.part} is a part of the kit`).toBe(true);
        }
      }
      const spec = specOf(card, false);
      const colors = [spec.sky.from, spec.sky.to, spec.glow.color, ...spec.ridges.flatMap((ridge) => [ridge.fill, ...(ridge.stroke === undefined ? [] : [ridge.stroke])])];
      for (const hex of colors) expect(inRadiantGold(hex), `${card.id}: ${hex}`).toBe(false);
    }
  });

  it("R3572001 a Mythic draws its near and particle layers twice a Legendary's template count", () => {
    for (const [id, , rarity] of STINKY_PLUS_SCENE_CARDS) {
      const scene = recipe(id);
      const near = scene.layers.near?.length ?? 0;
      expect(near, id).toBeGreaterThan(0);
      if (rarity === "Mythic") expect(near % 2, id).toBe(0);
    }
  });
});
