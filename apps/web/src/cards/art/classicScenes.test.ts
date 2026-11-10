// R3352000: Classic's scenes (sceneRecipesClassic1.ts to sceneRecipesClassic3.ts, ART.md "Classic's scenes"). Every
// Classic entry but Corpse Plantation, the Glitch included, draws a scene recipe of its own in Classic's family, on
// both faces with one geometry, at or above its rarity's floor (R3331001) and under the ceiling, its median detail
// climbing rarity by rarity; a Unit's character is a figure of its own or a creature part; a scene's extra figures
// stand in their own layers and In Too Deep's props are placements marked `props`; every part this pass added to the
// kits is drawn by a Classic scene; and Corpse Plantation, #3331's proof, draws as #3331 left it. The pixels are
// e2e/cypress/component/classic-scenes.cy.tsx's.

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { GLITCH_DEF_ID } from "@jackioh/engine/config";
import type { CardDef, Rarity } from "@jackioh/shared";

import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { FIGURE_DEFAULT_LOOK, FIGURE_KIT, type FigureLook } from "./figureKit.ts";
import { hashId } from "./hash.ts";
import { motifFor } from "./motifs.ts";
import { ART_ROLES, FULL_ART_CARD_MOTION, FULL_ART_MOTION } from "./motion.ts";
import { artSeedOf } from "./pictures.ts";
import { artSpec, type ArtSpec } from "./procedural.ts";
import { ART_DETAIL_MAX_BYTES, ART_DETAIL_MIN, ART_DETAIL_SHORT, artElements } from "./richness.ts";
import { composeScene, SCENE_LAYERS, sceneRecipeOf, sceneUse, type SceneLayer, type SceneRecipe } from "./scene.ts";
import { SCENE_KIT } from "./sceneKit.ts";
import { CLASSIC_KIT } from "./sceneKitClassic.ts";
import { SCENE_RECIPES } from "./sceneRecipes.ts";
import { CLASSIC_RECIPES_1 } from "./sceneRecipesClassic1.ts";
import { CLASSIC_RECIPES_2 } from "./sceneRecipesClassic2.ts";
import { CLASSIC_RECIPES_3 } from "./sceneRecipesClassic3.ts";
import { proceduralArtUri } from "./svg.ts";
import { compositionOf, themeFor } from "./themes.ts";

const SVG_PREFIX = "data:image/svg+xml,";
/** Corpse Plantation, #3331's proof card, whose scene that issue owns. */
const PROOF = "classic-074";
/** The rarities whose scene has a foreground, a middle and a background. */
const DEEP: readonly Rarity[] = ["Epic", "Legendary", "Mythic"];
/** The ladder the scenes' median detail climbs, the Glitch apart. */
const LADDER: readonly Rarity[] = ["Common", "Rare", "Epic", "Legendary", "Mythic"];
/** The Units that are no person, each drawn as a creature part of the scene kit in its subject layer, with no figure. */
const CREATURE_UNITS: Readonly<Record<string, string>> = {
  "classic-021": "tortoise",
  "classic-027": "slime",
  "classic-048": "shrimp",
  "classic-053": "crawler",
  "classic-069": "boar",
  "classic-071": "maw",
};
/** The figure kit's parts this pass added (figureKit.ts's R3352000 line). */
const CLASSIC_FIGURE_PARTS = [
  "pose-crouch",
  "head-sheep",
  "head-ape",
  "head-nose",
  "jester-cap",
  "cat-hood",
  "beak-mask",
  "king-crown",
  "jagged-crown",
  "outfit-harlequin",
  "outfit-travel-cloak",
  "outfit-spider",
  "item-card-fan",
  "item-rake",
  "item-cleaver",
  "item-cookie",
  "item-grapple",
  "item-abacus",
] as const;
/** Where an emblem may stand and how big it may be: inside the band every face's window shows (motifs.test.ts's). */
const EMBLEM_BAND = { left: 10, right: 90, top: 22, bottom: 84, size: 8 } as const;
/** Corpse Plantation's base and Radiant picture as #3331 left it (coreScenes.test.ts's and setScenes.test.ts's pin). */
const PROOF_PIN = [1210823500, 2735167760] as const;

const FILES: readonly Readonly<Record<string, SceneRecipe>>[] = [CLASSIC_RECIPES_1, CLASSIC_RECIPES_2, CLASSIC_RECIPES_3];
/** The pass: every Classic entry but Corpse Plantation, the Glitch among them. */
const CLASSIC: readonly CardDef[] = Object.values(CATALOG).filter((card) => card.set === "Classic" && card.id !== PROOF);
/** The entries a face's window shows a picture for: all but the Glitch, whose window draws none (glitch.css). */
const MEASURED: readonly CardDef[] = CLASSIC.filter((card) => card.id !== GLITCH_DEF_ID);

function svgOf(uri: string): string {
  return decodeURIComponent(uri.slice(SVG_PREFIX.length));
}

function recipe(id: string): SceneRecipe {
  const found = sceneRecipeOf(id);
  if (found === null) throw new Error(`no recipe for ${id}`);
  return found;
}

/** A card's spec as the procedural painter draws it, in its own composition (scene.test.ts's). */
function specOf(card: CardDef, radiant: boolean): ArtSpec {
  const theme = themeFor(card.tags, card.type);
  return artSpec(card.id, theme, compositionOf(card.id, card.type), radiant, motifFor(card.name, theme));
}

function lookParts(look: FigureLook): string[] {
  return [look.head, look.hair, look.outfit, look.item].filter((part): part is NonNullable<typeof part> => part !== null);
}

function median(values: readonly number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 1 ? (sorted[mid] ?? 0) : ((sorted[mid - 1] ?? 0) + (sorted[mid] ?? 0)) / 2;
}

describe("R3352000 Classic's scenes", () => {
  it("R3352000 every Classic entry but Corpse Plantation draws a scene recipe of its own, in Classic's family, off ART_DETAIL_SHORT", () => {
    expect(CLASSIC).toHaveLength(90);
    expect(CLASSIC.map((card) => card.id)).toContain(GLITCH_DEF_ID);
    const ids = FILES.flatMap((file) => Object.keys(file));
    expect(new Set(ids).size, "no id in two files").toBe(ids.length);
    expect([...ids].sort()).toEqual(CLASSIC.map((card) => card.id).sort());
    expect(ids).not.toContain(PROOF);
    const seen = new Map<string, string>();
    for (const file of FILES) {
      for (const [id, scene] of Object.entries(file)) {
        expect(SCENE_RECIPES[id], id).toBe(scene);
        expect(sceneRecipeOf(id), id).toBe(scene);
        expect(scene.family, id).toBe("classic");
        expect(scene.ownSubject, id).toBeUndefined();
        const json = JSON.stringify(scene);
        expect(seen.get(json), `${id} draws ${seen.get(json) ?? ""}'s recipe`).toBeUndefined();
        seen.set(json, id);
      }
    }
    // Off the short list, every one; the Glitch was never on it (its window draws no picture).
    for (const card of CLASSIC) expect(ART_DETAIL_SHORT.has(card.id), card.id).toBe(false);
    // The emblem stands small inside the band every face shows.
    for (const card of CLASSIC) {
      const { emblem } = recipe(card.id);
      if (emblem === undefined) continue;
      expect(emblem.size, card.id).toBeLessThanOrEqual(EMBLEM_BAND.size);
      expect(emblem.x, card.id).toBeGreaterThanOrEqual(EMBLEM_BAND.left);
      expect(emblem.x, card.id).toBeLessThanOrEqual(EMBLEM_BAND.right);
      expect(emblem.y, card.id).toBeGreaterThanOrEqual(EMBLEM_BAND.top);
      expect(emblem.y, card.id).toBeLessThanOrEqual(EMBLEM_BAND.bottom);
    }
  });

  it("R3352000 every Classic scene clears its floor under the ceiling, richer the rarer, and its Radiant face repaints every shape", () => {
    const elements = new Map<Rarity, number[]>();
    for (const card of CLASSIC) {
      const shapes = composeScene(recipe(card.id), card.id);
      const floor = ART_DETAIL_MIN[card.rarity];
      const base = proceduralCardArtUri(card, false);
      const drawn = artElements(svgOf(base));
      expect(drawn, card.id).toBeGreaterThanOrEqual(floor.elements);
      expect(sceneUse(shapes).layers, card.id).toBeGreaterThanOrEqual(floor.layers);
      const layers = new Set<SceneLayer>(shapes.map((shape) => shape.layer));
      expect(layers, card.id).toContain("subject");
      if (DEEP.includes(card.rarity)) for (const layer of ["far", "mid", "near"] as const) expect(layers, `${card.id} ${layer}`).toContain(layer);
      expect(base.length, `${card.id} base`).toBeLessThanOrEqual(ART_DETAIL_MAX_BYTES);
      expect(proceduralCardArtUri(card, true).length, `${card.id} radiant`).toBeLessThanOrEqual(ART_DETAIL_MAX_BYTES);
      // One geometry, every shape repainted: the base and Radiant geometry rule.
      const flat = specOf(card, false);
      const gilt = specOf(card, true);
      expect(gilt.ridges.map((ridge) => ridge.d), card.id).toEqual(flat.ridges.map((ridge) => ridge.d));
      expect(flat.ridges.map((ridge) => ridge.d), card.id).toEqual(shapes.map((shape) => shape.d));
      flat.ridges.forEach((ridge, k) => expect(gilt.ridges[k]?.fill, `${card.id} #${k}`).not.toBe(ridge.fill));
      if (card.rarity !== "Token") elements.set(card.rarity, [...(elements.get(card.rarity) ?? []), drawn]);
    }
    // The Glitch, a token, is held to a token's floor, Common's.
    expect(CATALOG[GLITCH_DEF_ID]?.rarity).toBe("Token");
    // Richer the rarer: the median scene of each rarity draws more than the one below it.
    const medians = LADDER.map((rarity) => median(elements.get(rarity) ?? []));
    medians.slice(1).forEach((value, k) => expect(value, `${LADDER[k + 1] ?? ""} over ${LADDER[k] ?? ""}`).toBeGreaterThan(medians[k] ?? Infinity));
  });

  it("R3352000 a Classic Unit draws its character from the kits, never the painter's silhouette, no two alike", () => {
    const units = CLASSIC.filter((card) => card.type === "Unit");
    expect(units).toHaveLength(34);
    const characters = new Map<string, string>();
    for (const card of units) {
      const scene = recipe(card.id);
      const creature = CREATURE_UNITS[card.id];
      if (creature === undefined) {
        expect(scene.figure, `${card.id} draws no figure`).toBeDefined();
        if (scene.figure === undefined) continue;
        expect(scene.figure.look, card.id).not.toEqual(FIGURE_DEFAULT_LOOK);
        const drawn = new Set<string>(composeScene(scene, card.id).map((shape) => shape.part));
        for (const part of [scene.figure.pose, ...lookParts(scene.figure.look)]) expect(drawn, `${card.id} ${part}`).toContain(part);
        const key = JSON.stringify(scene.figure.look);
        expect(characters.get(key), `${card.id} wears ${characters.get(key) ?? ""}'s look`).toBeUndefined();
        characters.set(key, card.id);
      } else {
        // A Unit that is no person: no figure, its creature in its subject layer.
        expect(scene.figure, card.id).toBeUndefined();
        expect(SCENE_KIT[creature as keyof typeof SCENE_KIT].category, creature).toBe("creatures");
        expect((scene.layers.subject ?? []).map((placed) => placed.part), card.id).toContain(creature);
      }
      // The face draws the scene alone: none of the painter's own subject.
      expect(specOf(card, false).ridges.map((ridge) => ridge.d), card.id).toEqual(composeScene(scene, card.id).map((shape) => shape.d));
    }
    expect(units.filter((card) => recipe(card.id).figure === undefined).map((card) => card.id).sort()).toEqual(Object.keys(CREATURE_UNITS).sort());
  });

  it("R3352000 a Classic scene's extra figures stand in their layers, and In Too Deep still moves its props", () => {
    let extras = 0;
    for (const card of CLASSIC) {
      const scene = recipe(card.id);
      for (const layer of SCENE_LAYERS) {
        for (const placed of scene.layers[layer] ?? []) if (placed.props === true) expect(layer, `${card.id} ${placed.part}`).not.toBe("subject");
      }
      const drawn = composeScene(scene, card.id);
      for (const extra of scene.extras ?? []) {
        expect(drawn.some((shape) => shape.layer === extra.layer && Object.hasOwn(FIGURE_KIT, shape.part) && shape.part === extra.pose), `${card.id} ${extra.pose} in ${extra.layer}`).toBe(true);
        extras += 1;
      }
    }
    expect(extras).toBeGreaterThan(0);
    // The full-art Classic card moves every layer its motion names (R2715001), its props among them: a scene draws no
    // motif glyph or accent, so its props are placements marked `props` (R3390000), never in the subject layer.
    // The full-art cards' own motions (R2715001), not R3340000's recipes for every other Legendary and Mythic.
    const fullArt = CLASSIC.filter((card) => FULL_ART_CARD_MOTION[card.id] !== undefined);
    expect(fullArt.map((card) => card.id)).toEqual(["classic-090"]);
    for (const card of fullArt) {
      const motion = FULL_ART_MOTION[card.id] ?? null;
      const theme = themeFor(card.tags, card.type);
      const svg = svgOf(proceduralArtUri(card.id, theme, compositionOf(card.id, card.type), false, motifFor(card.name, theme), motion));
      for (const role of ART_ROLES) if (motion?.[role] !== undefined) expect(svg, `${card.id}: ${role}`).toContain(`class="art-m-${role}"`);
      expect(specOf(card, false).ridges.some((ridge) => ridge.role === "props"), card.id).toBe(motion?.props !== undefined);
    }
  });

  it("R3352000 In Too Deep's scene holds a part no other scene uses, and the Glitch's picture draws its scene", () => {
    const partsOf = (id: string): Set<string> => new Set(composeScene(recipe(id), id).map((shape) => shape.part));
    const all = Object.keys(SCENE_RECIPES);
    const mythics = CLASSIC.filter((card) => card.rarity === "Mythic");
    expect(mythics.map((card) => card.id)).toEqual(["classic-090"]);
    for (const card of mythics) {
      const others = new Set(all.filter((id) => id !== card.id).flatMap((id) => [...partsOf(id)]));
      const own = [...partsOf(card.id)].filter((part) => !others.has(part));
      expect(own.length, `${card.id} draws only parts another scene draws`).toBeGreaterThan(0);
    }
    // The Glitch's window shows no picture (glitch.css), but its picture, wherever one is drawn, is its scene.
    const glitch = CATALOG[GLITCH_DEF_ID];
    if (glitch === undefined) throw new Error("the catalog has no Glitch");
    for (const radiant of [false, true]) {
      const svg = svgOf(proceduralCardArtUri(glitch, radiant));
      for (const shape of composeScene(recipe(glitch.id), glitch.id)) expect(svg.includes(`d="${shape.d}"`), `${glitch.id} ${shape.part}`).toBe(true);
    }
    expect(MEASURED).toHaveLength(89);
  });

  it("R3352000 Alt art and a repaint grow the same parts from their own seed", () => {
    for (const card of CLASSIC) {
      const scene = recipe(card.id);
      const normal = composeScene(scene, card.id);
      for (const picture of ["alt", "night"] as const) {
        const seed = artSeedOf(card.id, picture);
        expect(seed, card.id).not.toBe(card.id);
        expect(sceneRecipeOf(seed), card.id).toBe(scene);
        const own = composeScene(scene, seed);
        expect(new Set(own.map((shape) => shape.part)), `${card.id} ${picture}`).toEqual(new Set(normal.map((shape) => shape.part)));
        expect(sceneUse(own), `${card.id} ${picture}`).toEqual(sceneUse(normal));
      }
    }
  });

  it("R3352000 every part this pass added joins the kits beside the old ones and is drawn by a Classic scene", () => {
    // Classic's scenery is a module of its own, spread into SCENE_KIT after every earlier part: an id an earlier
    // part held would keep that part's place (and replace it without a word), so this module's entries stand
    // together, in its order (R3406000's Stinkyscape Alt art module follows them, stinkyscape91AltScenes.test.tsx).
    const added = Object.entries(CLASSIC_KIT);
    expect(added.length).toBeGreaterThan(0);
    const kit = Object.keys(SCENE_KIT);
    const start = kit.indexOf(added[0]?.[0] ?? "");
    expect(kit.slice(start, start + added.length)).toEqual(added.map(([id]) => id));
    for (const [id, part] of added) expect(SCENE_KIT[id as keyof typeof SCENE_KIT], id).toBe(part);
    for (const id of CLASSIC_FIGURE_PARTS) expect(Object.hasOwn(FIGURE_KIT, id), id).toBe(true);
    // Each new part is there because a brief named it: a Classic scene draws it.
    const drawn = new Set(CLASSIC.flatMap((card) => composeScene(recipe(card.id), card.id).map((shape) => shape.part)));
    for (const id of [...added.map(([part]) => part), ...CLASSIC_FIGURE_PARTS]) expect(drawn, id).toContain(id);
  });

  it("R3352000 Corpse Plantation, #3331's proof, draws as #3331 left it", () => {
    const card = CATALOG[PROOF];
    if (card === undefined) throw new Error(`the catalog has no ${PROOF}`);
    expect([hashId(proceduralCardArtUri(card, false)), hashId(proceduralCardArtUri(card, true))]).toEqual([...PROOF_PIN]);
  });
});
