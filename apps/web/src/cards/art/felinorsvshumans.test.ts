// R5264000: the held Felinors vs Humans set's placeholder art (felinorsVsHumansRecipes.ts). Every entry of the set, its 35
// tokens and its 82 Plagiarized reprints included, draws a scene recipe of its own through the procedural painter, base and
// Radiant, at or above its rarity's detail floor and under the ceiling; each table's rank is its card's printed rarity; the
// Legendaries and Mythics, the tokens printed either among them, use every layer and move by a recipe of their own, and
// every Mythic has its diorama; every Felinor wears the cat ears on its head; every reprint is its original's template
// photocopied and stamped; and no id outside the set reaches the new recipes.

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { artMotionOf } from "../fullArt.ts";
import { dioramaOf } from "./dioramas/index.ts";
import {
  FELINORS_VS_HUMANS_PLAGIARIZED,
  FELINORS_VS_HUMANS_PREFIX,
  FELINORS_VS_HUMANS_RECIPES,
  FELINORS_VS_HUMANS_SCENE_CARDS,
  type FvhRank,
} from "./felinorsVsHumansRecipes.ts";
import { INFINITE_ASSAULT_RECIPES, INFINITE_ASSAULT_SCENE_CARDS } from "./infiniteAssaultRecipes.ts";
import { FULL_ART_MOTION } from "./motion.ts";
import { ART_DETAIL_MAX_BYTES, ART_DETAIL_MIN, ART_DETAIL_SHORT, artElements } from "./richness.ts";
import { composeScene, SCENE_LAYERS, sceneRecipeOf, sceneUse, type SceneLayer, type SceneRecipe } from "./scene.ts";
import { SCENE_KIT } from "./sceneKit.ts";

const SVG_PREFIX = "data:image/svg+xml,";

/** Every catalog entry of the set, tokens and reprints included, in catalog order. */
const SET: readonly CardDef[] = Object.values(CATALOG).filter((card) => card.set === "Felinors vs Humans");

function recipe(id: string): SceneRecipe {
  const found = sceneRecipeOf(id);
  if (found === null) throw new Error(`no recipe for ${id}`);
  return found;
}

function svgOf(uri: string): string {
  return decodeURIComponent(uri.slice(SVG_PREFIX.length));
}

/** The rarity a card prints: a token's printed one, else its own. */
function printed(card: CardDef): string {
  return card.printedRarity ?? card.rarity;
}

/** The set piece a card's rarity asks for. */
function rankOf(card: CardDef): FvhRank {
  const rarity = printed(card);
  return rarity === "Mythic" ? "mythic" : rarity === "Legendary" ? "legendary" : "plain";
}

/** Every part a recipe places, its layers' alone (a figure's parts are the figure kit's). */
function partsOf(scene: SceneRecipe): Set<string> {
  return new Set(SCENE_LAYERS.flatMap((layer) => (scene.layers[layer] ?? []).map((placed) => placed.part)));
}

describe("R5264000 the Felinors vs Humans set's placeholder art", () => {
  it("R5264000 the tables are the set's every entry, tokens and reprints included, each drawn from a recipe of its own", () => {
    expect(SET.length).toBe(232);
    expect(SET.filter((card) => card.token)).toHaveLength(35);
    expect(SET.filter((card) => card.tags.includes("Plagiarized"))).toHaveLength(82);
    const tabled = [...FELINORS_VS_HUMANS_SCENE_CARDS.map(([id]) => id), ...FELINORS_VS_HUMANS_PLAGIARIZED.map(([id]) => id)];
    expect(tabled.sort()).toEqual(SET.map((card) => card.id).sort());
    expect(FELINORS_VS_HUMANS_PLAGIARIZED.map(([id]) => id).sort()).toEqual(SET.filter((card) => card.tags.includes("Plagiarized")).map((card) => card.id).sort());
    // Each table's rank is the rarity its card prints.
    const ranks = new Map<string, FvhRank>([
      ...FELINORS_VS_HUMANS_SCENE_CARDS.map((row) => [row[0], row[2]] as const),
      ...FELINORS_VS_HUMANS_PLAGIARIZED.map((row) => [row[0], row[3]] as const),
    ]);
    const seen = new Map<string, string>();
    for (const card of SET) {
      expect(ranks.get(card.id), card.id).toBe(rankOf(card));
      const scene = recipe(card.id);
      expect(scene, card.id).toBe(FELINORS_VS_HUMANS_RECIPES[card.id]);
      const key = JSON.stringify(scene);
      expect(seen.get(key), `${card.id} draws ${seen.get(key) ?? ""}'s recipe`).toBeUndefined();
      seen.set(key, card.id);
      for (const layer of SCENE_LAYERS) {
        expect((scene.layers[layer] ?? []).length, `${card.id} ${layer}`).toBeGreaterThan(0);
        for (const placed of scene.layers[layer] ?? []) {
          expect(placed.rotate, `${card.id} ${placed.part}`).toBeUndefined();
          expect(Object.hasOwn(SCENE_KIT, placed.part), `${card.id} ${placed.part} is a part of the kit`).toBe(true);
        }
      }
    }
  });

  it("R5264000 R3331001 every entry clears its rarity's floor, off ART_DETAIL_SHORT, and every picture stays under the ceiling", () => {
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

  it("R5264000 R3340000 the Legendaries and Mythics, tokens printed either included, use every layer, move by a recipe of their own and the Mythics have dioramas", () => {
    const rare = SET.filter((card) => rankOf(card) !== "plain").map((card) => card.id);
    expect(rare).toHaveLength(63);
    expect(SET.filter((card) => card.rarity === "Legendary" || card.rarity === "Mythic")).toHaveLength(40);
    const others = new Map(Object.entries(FULL_ART_MOTION).filter(([id]) => !rare.includes(id)).map(([id, motion]) => [JSON.stringify(motion), id]));
    for (const id of rare) {
      const layers = new Set<SceneLayer>(composeScene(recipe(id), id).map((shape) => shape.layer));
      for (const layer of SCENE_LAYERS) expect(layers, `${id} ${layer}`).toContain(layer);
      const motion = FULL_ART_MOTION[id];
      expect(motion, id).toBeDefined();
      expect(artMotionOf(id), id).toBe(motion);
      expect(others.get(JSON.stringify(motion)), `${id} moves as another card does`).toBeUndefined();
    }
    const mythics = SET.filter((card) => rankOf(card) === "mythic");
    expect(mythics).toHaveLength(26);
    for (const card of mythics) expect(dioramaOf(card.id)?.id, card.id).toBe(card.id);
  });

  it("R5264000 every Felinor wears the cat ears on its figure's head", () => {
    const felinors = SET.filter((card) => card.tags.includes("Felinor"));
    expect(felinors.length).toBeGreaterThan(20);
    for (const card of felinors) {
      const scene = recipe(card.id);
      const figure = scene.figure;
      expect(figure, card.id).toBeDefined();
      if (figure === undefined) continue;
      const ears = (scene.layers.subject ?? []).filter((placed) => placed.part === "cat-ears");
      expect(ears, card.id).toHaveLength(1);
      const [placed] = ears;
      if (placed === undefined) continue;
      // Over the figure's head: at its x, above its neck.
      expect(Math.abs(placed.x - figure.x), card.id).toBeLessThan(1);
      expect(placed.y, card.id).toBeLessThan(figure.y - 30 * figure.scale);
      expect(composeScene(scene, card.id).some((shape) => shape.part === "cat-ears" && shape.layer === "subject"), card.id).toBe(true);
    }
    // #84 𓃠, the Egyptian Felinor, keeps Bastet's black cat beside her.
    expect(partsOf(recipe("felinorsvshumans-084")).has("cat")).toBe(true);
    // No other entry of the set wears them.
    for (const card of SET.filter((entry) => !entry.tags.includes("Felinor"))) expect(partsOf(recipe(card.id)).has("cat-ears"), card.id).toBe(false);
  });

  it("R5264000 every reprint is its Infinite Assault original's template, photocopied and stamped, and still its own", () => {
    const templates = new Map(INFINITE_ASSAULT_SCENE_CARDS.map(([id, template]) => [id, template] as const));
    for (const [id, original, template] of FELINORS_VS_HUMANS_PLAGIARIZED) {
      const card = CATALOG[id];
      const source = CATALOG[original];
      expect(card?.name, id).toBe(`Plagiarized: ${source?.name ?? "?"}`);
      expect(templates.get(original), id).toBe(template);
      const scene = recipe(id);
      const theirs = INFINITE_ASSAULT_RECIPES[original];
      expect(theirs, original).toBeDefined();
      if (theirs === undefined) continue;
      expect(scene, id).not.toBe(theirs);
      expect(scene.family, id).toBe(theirs.family);
      expect(scene.sky, id).not.toEqual(theirs.sky);
      // The original's every part, the red stamp near and the toner grain over it.
      const parts = partsOf(scene);
      for (const part of partsOf(theirs)) expect(parts.has(part), `${id} ${part}`).toBe(true);
      expect((scene.layers.near ?? []).filter((placed) => placed.part === "ink-seal"), id).toHaveLength(1);
      expect((scene.layers.particles ?? []).some((placed) => placed.part === "dust"), id).toBe(true);
    }
  });

  it("R5264000 R503 no id outside the set reaches the set's recipes", () => {
    const keys = Object.keys(FELINORS_VS_HUMANS_RECIPES);
    expect(keys).toHaveLength(SET.length);
    for (const id of keys) expect(id.startsWith(FELINORS_VS_HUMANS_PREFIX), id).toBe(true);
    const ours = new Set<SceneRecipe>(Object.values(FELINORS_VS_HUMANS_RECIPES));
    for (const card of Object.values(CATALOG)) {
      if (card.id.startsWith(FELINORS_VS_HUMANS_PREFIX)) continue;
      const scene = sceneRecipeOf(card.id);
      expect(scene !== null && ours.has(scene), card.id).toBe(false);
      expect(card.set, card.id).not.toBe("Felinors vs Humans");
    }
  });
});
