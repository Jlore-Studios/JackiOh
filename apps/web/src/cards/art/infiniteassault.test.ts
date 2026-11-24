// R4577009: the held Infinite Assault set's placeholder art (infiniteAssaultRecipes.ts). Every entry of the set, its
// fifteen tokens included, draws a scene recipe of its own through the procedural painter, base and Radiant, at or
// above its rarity's detail floor and under the ceiling; the Legendaries and Mythics use every layer and move by a
// recipe of their own; and no id outside the set reaches the new recipes.

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { artMotionOf } from "../fullArt.ts";
import { INFINITE_ASSAULT_PREFIX, INFINITE_ASSAULT_RECIPES, INFINITE_ASSAULT_SCENE_CARDS } from "./infiniteAssaultRecipes.ts";
import { FULL_ART_MOTION } from "./motion.ts";
import { ART_DETAIL_MAX_BYTES, ART_DETAIL_MIN, ART_DETAIL_SHORT, artElements } from "./richness.ts";
import { composeScene, SCENE_LAYERS, sceneRecipeOf, sceneUse, type SceneLayer, type SceneRecipe } from "./scene.ts";
import { SCENE_KIT } from "./sceneKit.ts";

const SVG_PREFIX = "data:image/svg+xml,";

/** Every catalog entry of the set, tokens included, in catalog order. */
const SET: readonly CardDef[] = Object.values(CATALOG).filter((card) => card.set === "Infinite Assault");

function recipe(id: string): SceneRecipe {
  const found = sceneRecipeOf(id);
  if (found === null) throw new Error(`no recipe for ${id}`);
  return found;
}

function svgOf(uri: string): string {
  return decodeURIComponent(uri.slice(SVG_PREFIX.length));
}

describe("R4577009 the Infinite Assault set's placeholder art", () => {
  it("R4577009 the table is the set's every entry, tokens included, each drawn from a recipe of its own", () => {
    expect(SET.length).toBe(255);
    expect(SET.filter((card) => card.token)).toHaveLength(15);
    expect(INFINITE_ASSAULT_SCENE_CARDS.map(([id]) => id).sort()).toEqual(SET.map((card) => card.id).sort());
    const seen = new Map<string, string>();
    for (const card of SET) {
      const scene = recipe(card.id);
      expect(scene, card.id).toBe(INFINITE_ASSAULT_RECIPES[card.id]);
      const key = JSON.stringify(scene);
      expect(seen.get(key), `${card.id} draws ${seen.get(key) ?? ""}'s recipe`).toBeUndefined();
      seen.set(key, card.id);
      for (const layer of SCENE_LAYERS) {
        for (const placed of scene.layers[layer] ?? []) {
          expect(placed.rotate, `${card.id} ${placed.part}`).toBeUndefined();
          expect(Object.hasOwn(SCENE_KIT, placed.part), `${card.id} ${placed.part} is a part of the kit`).toBe(true);
        }
      }
    }
  });

  it("R4577009 every entry clears its rarity's floor, off ART_DETAIL_SHORT, and every picture stays under the ceiling", () => {
    for (const card of SET) {
      const floor = ART_DETAIL_MIN[card.rarity];
      expect(artElements(svgOf(proceduralCardArtUri(card, false))), card.id).toBeGreaterThanOrEqual(floor.elements);
      expect(sceneUse(composeScene(recipe(card.id), card.id)).layers, card.id).toBeGreaterThanOrEqual(floor.layers);
      expect(ART_DETAIL_SHORT.has(card.id), card.id).toBe(false);
      for (const radiant of [false, true]) {
        expect(proceduralCardArtUri(card, radiant).length, `${card.id} ${radiant ? "radiant" : "base"}`).toBeLessThanOrEqual(ART_DETAIL_MAX_BYTES);
      }
    }
  });

  it("R4577009 the Legendaries and Mythics use every layer and move by a recipe of their own", () => {
    const legendaries = SET.filter((card) => card.rarity === "Legendary" || card.rarity === "Mythic").map((card) => card.id);
    expect(legendaries.sort()).toEqual(["infiniteassault-023", "infiniteassault-024", "infiniteassault-046", "infiniteassault-047", "infiniteassault-048", "infiniteassault-070", "infiniteassault-071", "infiniteassault-072", "infiniteassault-094", "infiniteassault-095", "infiniteassault-096", "infiniteassault-118", "infiniteassault-119", "infiniteassault-120", "infiniteassault-142", "infiniteassault-143", "infiniteassault-144", "infiniteassault-166", "infiniteassault-167", "infiniteassault-168", "infiniteassault-190", "infiniteassault-191", "infiniteassault-192", "infiniteassault-214", "infiniteassault-215", "infiniteassault-216", "infiniteassault-217", "infiniteassault-219", "infiniteassault-221", "infiniteassault-223", "infiniteassault-225", "infiniteassault-227", "infiniteassault-229", "infiniteassault-230", "infiniteassault-231", "infiniteassault-232", "infiniteassault-233", "infiniteassault-234", "infiniteassault-235", "infiniteassault-236", "infiniteassault-237", "infiniteassault-238", "infiniteassault-239", "infiniteassault-240"]);
    const others = new Map(Object.entries(FULL_ART_MOTION).filter(([id]) => !legendaries.includes(id)).map(([id, motion]) => [JSON.stringify(motion), id]));
    for (const id of legendaries) {
      const layers = new Set<SceneLayer>(composeScene(recipe(id), id).map((shape) => shape.layer));
      for (const layer of SCENE_LAYERS) expect(layers, `${id} ${layer}`).toContain(layer);
      const motion = FULL_ART_MOTION[id];
      expect(motion, id).toBeDefined();
      expect(artMotionOf(id), id).toBe(motion);
      expect(others.get(JSON.stringify(motion)), `${id} moves as another card does`).toBeUndefined();
    }
  });

  it("R4577009 no id outside the set reaches the set's recipes", () => {
    const keys = Object.keys(INFINITE_ASSAULT_RECIPES);
    expect(keys).toHaveLength(SET.length);
    for (const id of keys) expect(id.startsWith(INFINITE_ASSAULT_PREFIX), id).toBe(true);
    const ours = new Set<SceneRecipe>(Object.values(INFINITE_ASSAULT_RECIPES));
    for (const card of Object.values(CATALOG)) {
      if (card.id.startsWith(INFINITE_ASSAULT_PREFIX)) continue;
      const scene = sceneRecipeOf(card.id);
      expect(scene !== null && ours.has(scene), card.id).toBe(false);
      expect(card.set, card.id).not.toBe("Infinite Assault");
    }
  });
});
