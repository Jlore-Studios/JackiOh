// R3401000: every Meditative entry, tokens included, drawn from a scene recipe of its own (meditativeRecipes1.ts to
// meditativeRecipes5.ts), base and Radiant, in Meditative's ink-wash look, at or above its rarity's detail floor and
// off the floor's short list; every Unit a character of its own; one red seal on every picture; the two full-art
// motions that move props; and the eight kit modules #3401 added beside the first kit and Classic+'s. The pixels are
// e2e/cypress/component/meditative-scenes.cy.tsx's.

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef, Rarity } from "@jackioh/shared";

import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import {
  RADIANT_GOLD_HUE_MAX_DEG,
  RADIANT_GOLD_HUE_MIN_DEG,
  RADIANT_GOLD_LIGHTNESS_MAX,
  RADIANT_GOLD_LIGHTNESS_MIN,
  RADIANT_GOLD_SATURATION_MIN,
} from "../constants.ts";
import { figureLookFor } from "./figureKit.ts";
import { motifFor } from "./motifs.ts";
import { ART_ROLES, FULL_ART_CARD_MOTION, FULL_ART_MOTION } from "./motion.ts";
import { artSeedOf } from "./pictures.ts";
import { artSpec, type ArtSpec } from "./procedural.ts";
import { ART_DETAIL_MAX_BYTES, ART_DETAIL_MIN, ART_DETAIL_SHORT, artElements } from "./richness.ts";
import { composeScene, SCENE_LAYERS, sceneRecipeOf, sceneUse, type SceneLayer, type SceneRecipe } from "./scene.ts";
import { SCENE_KIT } from "./sceneKit.ts";
import { COSMOS_KIT } from "./sceneKitCosmos.ts";
import { CRAFT_KIT } from "./sceneKitCraft.ts";
import { GEAR_KIT } from "./sceneKitGear.ts";
import { INK_BUILDINGS_KIT } from "./sceneKitInkBuildings.ts";
import { INK_CHARACTERS_KIT } from "./sceneKitInkCharacters.ts";
import { INK_CREATURES_KIT } from "./sceneKitInkCreatures.ts";
import { INK_CURIOS_KIT } from "./sceneKitInkCurios.ts";
import { INK_MACHINES_KIT } from "./sceneKitInkMachines.ts";
import { INK_NATURE_KIT } from "./sceneKitInkNature.ts";
import { INK_OBJECTS_KIT } from "./sceneKitInkObjects.ts";
import { INK_PLACES_KIT } from "./sceneKitInkPlaces.ts";
import { MOUNTAIN_KIT } from "./sceneKitMountain.ts";
import { NATURE_KIT } from "./sceneKitNature.ts";
import { OBJECTS_KIT } from "./sceneKitObjects.ts";
import { PLACES_KIT } from "./sceneKitPlaces.ts";
import { SPACE_KIT } from "./sceneKitSpace.ts";
import { STINKY_ALT_KIT } from "./sceneKitStinkyAlt.ts";
import { TOUCHES_KIT } from "./sceneKitTouches.ts";
import { YARD_KIT } from "./sceneKitYard.ts";
import { ALT_SCENE_RECIPES } from "./sceneRecipes.ts";
import { proceduralArtUri } from "./svg.ts";
import { compositionOf, themeFor } from "./themes.ts";

const SVG_PREFIX = "data:image/svg+xml,";
/** Every Meditative entry, its tokens included. */
const ENTRIES: readonly CardDef[] = Object.values(CATALOG).filter((card) => card.set === "Meditative");
/** The rarer than Rare: a scene with a foreground, a middle and a background. */
const DEEP: readonly Rarity[] = ["Epic", "Legendary", "Mythic"];
/**
 * The Units that are no person, each drawn as its part of the scene kit in its subject layer, with no figure: a
 * beast, a plant, a machine, a sprite, a totem, and the nine buildings Jlockeed's Evil Blueprints raises.
 */
const CHARACTER_UNITS: Readonly<Record<string, string>> = {
  "meditative-002": "rhino",
  "meditative-003": "clockwork-sentry",
  "meditative-013": "pea-vine",
  "meditative-028": "iris-sprite",
  "meditative-047": "taotie",
  "meditative-049-2": "glitch-critter",
  "meditative-051": "round-bot",
  "meditative-067": "cat-ears",
  "meditative-074": "rock-giant",
  "meditative-084": "jar-sprite",
  "meditative-087": "totem",
  "meditative-088": "ink-sheep",
  "meditative-097-1": "stake-plot",
  "meditative-097-2": "stone-well",
  "meditative-097-3": "schoolhouse",
  "meditative-097-4": "glass-church",
  "meditative-097-5": "bunker",
  "meditative-097-6": "campus-hall",
  "meditative-097-7": "long-wall",
  "meditative-097-8": "sea-prison",
  "meditative-097-9": "peak-fortress",
};
/** The kit modules #3401 added, which sceneKit.ts spreads into `SCENE_KIT` after Classic+'s and #3393's. */
const INK_MODULES = [INK_PLACES_KIT, INK_BUILDINGS_KIT, INK_OBJECTS_KIT, INK_CURIOS_KIT, INK_MACHINES_KIT, INK_NATURE_KIT, INK_CREATURES_KIT, INK_CHARACTERS_KIT];
/**
 * The parts sceneKit.ts draws itself: the first kit's (#3331), the 199 Core's scenes added after them (R3392000) and the
 * 81 Stinkyscape's pass added after those (R3391000, classicPlusScenes.test.ts's `STINKYSCAPE_KIT_PARTS`).
 */
const FIRST_KIT_PARTS = 56 + 199 + 81;
/** Where an emblem or the seal may stand, and how big an emblem may be: inside the band every face's window shows (motifs.test.ts's). */
const SHOWN_BAND = { left: 10, right: 90, top: 22, bottom: 84, size: 8 } as const;

function recipe(id: string): SceneRecipe {
  const found = sceneRecipeOf(id);
  if (found === null) throw new Error(`no recipe for ${id}`);
  return found;
}

function svgOf(uri: string): string {
  return decodeURIComponent(uri.slice(SVG_PREFIX.length));
}

/** A card's spec as its face's painter draws it, in its own composition, its recipe read from its id. */
function specOf(card: CardDef, radiant: boolean): ArtSpec {
  const theme = themeFor(card.tags, card.type);
  return artSpec(card.id, theme, compositionOf(card.id, card.type), radiant, motifFor(card.name, theme));
}

/** A hex colour's hue in degrees and saturation and lightness from 0 to 1 (scene.test.ts's). */
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

describe("R3401000 Meditative drawn as scenes", () => {
  it("R3401000 every Meditative entry, tokens included, draws from a recipe of its own in Meditative's look", () => {
    expect(ENTRIES).toHaveLength(132);
    expect(ENTRIES.filter((card) => card.token)).toHaveLength(30);
    expect(Object.keys(CHARACTER_UNITS).every((id) => CATALOG[id]?.type === "Unit")).toBe(true);
    const seen = new Map<string, string>();
    for (const card of ENTRIES) {
      const scene = recipe(card.id);
      expect(scene.family, card.id).toBe("meditative");
      expect(scene.ownSubject, card.id).toBeUndefined();
      const key = JSON.stringify(scene);
      expect(seen.get(key), `${card.id} draws ${seen.get(key) ?? ""}'s recipe`).toBeUndefined();
      seen.set(key, card.id);
      // Meditative's brush line outlines the scene's solid shapes.
      expect(composeScene(scene, card.id).some((shape) => shape.inked === true), card.id).toBe(true);
    }
  });

  it("R3401000 every Meditative entry clears its rarity's floor, stays under the byte ceiling and is off ART_DETAIL_SHORT", () => {
    for (const card of ENTRIES) {
      const floor = ART_DETAIL_MIN[card.rarity];
      const shapes = composeScene(recipe(card.id), card.id);
      expect(artElements(svgOf(proceduralCardArtUri(card, false))), card.id).toBeGreaterThanOrEqual(floor.elements);
      for (const radiant of [false, true]) expect(proceduralCardArtUri(card, radiant).length, `${card.id} ${radiant ? "radiant" : "base"}`).toBeLessThanOrEqual(ART_DETAIL_MAX_BYTES);
      expect(sceneUse(shapes).layers, card.id).toBeGreaterThanOrEqual(floor.layers);
      const layers = new Set<SceneLayer>(shapes.map((shape) => shape.layer));
      expect(layers, card.id).toContain("subject");
      if (DEEP.includes(card.rarity)) for (const layer of ["far", "mid", "subject", "near"] as const) expect(layers, `${card.id} ${layer}`).toContain(layer);
      expect(ART_DETAIL_SHORT.has(card.id), card.id).toBe(false);
    }
    expect([...ART_DETAIL_SHORT].filter((id) => id.startsWith("meditative-"))).toEqual([]);
  });

  it("R3401000 base and Radiant draw one geometry, the Radiant face gilded", () => {
    for (const card of ENTRIES) {
      const shapes = composeScene(recipe(card.id), card.id);
      const base = specOf(card, false);
      const radiant = specOf(card, true);
      expect(base.ridges.map((ridge) => ridge.d), card.id).toEqual(shapes.map((shape) => shape.d));
      expect(radiant.ridges.map((ridge) => ridge.d), card.id).toEqual(base.ridges.map((ridge) => ridge.d));
      base.ridges.forEach((ridge, k) => expect(radiant.ridges[k]?.fill, `${card.id} #${k}`).not.toBe(ridge.fill));
      expect(radiant.sky, card.id).not.toEqual(base.sky);
      expect(base.rays, card.id).toHaveLength(0);
      expect(radiant.rays.length, card.id).toBeGreaterThan(0);
      for (const face of [false, true]) {
        const svg = svgOf(proceduralCardArtUri(card, face));
        for (const shape of shapes) expect(svg.includes(`d="${shape.d}"`), `${card.id} ${face ? "radiant" : "base"} ${shape.part}`).toBe(true);
      }
    }
  });

  it("R3401000 every Meditative Unit is a character of its own", () => {
    const units = ENTRIES.filter((card) => card.type === "Unit");
    expect(units).toHaveLength(64);
    const looks = new Map<string, string>();
    for (const card of units) {
      const scene = recipe(card.id);
      const character = CHARACTER_UNITS[card.id];
      if (character !== undefined) {
        expect(scene.figure, card.id).toBeUndefined();
        expect(scene.layers.subject?.map((placed) => placed.part), card.id).toContain(character);
        continue;
      }
      expect(scene.figure, card.id).toBeDefined();
      if (scene.figure === undefined) continue;
      // Each starts from its tag's people and is dressed as who it is.
      expect(scene.figure.look, card.id).not.toEqual(figureLookFor(card.tags));
      const key = JSON.stringify(scene.figure.look);
      expect(looks.get(key), `${card.id} looks as ${looks.get(key) ?? ""} does`).toBeUndefined();
      looks.set(key, card.id);
    }
    expect(looks.size).toBe(units.length - Object.keys(CHARACTER_UNITS).length);
  });

  it("R3401000 every Meditative picture stamps one red seal, near and inside the shown band", () => {
    for (const card of ENTRIES) {
      const scene = recipe(card.id);
      const seals = SCENE_LAYERS.flatMap((layer) => (scene.layers[layer] ?? []).filter((placed) => placed.part === "ink-seal").map((placed) => ({ layer, placed })));
      expect(seals, card.id).toHaveLength(1);
      const [seal] = seals;
      if (seal === undefined) continue;
      expect(seal.layer, card.id).toBe("near");
      expect(seal.placed.count ?? 1, card.id).toBe(1);
      expect(seal.placed.paint, card.id).toBeUndefined();
      expect(seal.placed.props, card.id).toBeUndefined();
      expect(seal.placed.x, card.id).toBeGreaterThanOrEqual(SHOWN_BAND.left);
      expect(seal.placed.x, card.id).toBeLessThanOrEqual(SHOWN_BAND.right);
      expect(seal.placed.y, card.id).toBeGreaterThanOrEqual(SHOWN_BAND.top);
      expect(seal.placed.y, card.id).toBeLessThanOrEqual(SHOWN_BAND.bottom);
      const shapes = composeScene(scene, card.id).filter((shape) => shape.part === "ink-seal");
      expect(shapes.length, card.id).toBeGreaterThan(0);
      expect(shapes.some((shape) => shape.paint === "red-600"), card.id).toBe(true);
    }
  });

  it("R3401000 props move outside the subject layer, and the two full-art motions that move props still do", () => {
    for (const card of ENTRIES) {
      const scene = recipe(card.id);
      for (const layer of SCENE_LAYERS) {
        for (const placed of scene.layers[layer] ?? []) if (placed.props === true) expect(layer, `${card.id} ${placed.part}`).not.toBe("subject");
      }
    }
    // The full-art cards' own motions (R2715001), not R3340000's recipes for every other Legendary and Mythic.
    const fullArt = ENTRIES.filter((card) => FULL_ART_CARD_MOTION[card.id] !== undefined);
    expect(fullArt.map((card) => card.id).sort()).toEqual(["meditative-021", "meditative-087"]);
    for (const card of fullArt) {
      const motion = FULL_ART_MOTION[card.id] ?? null;
      const theme = themeFor(card.tags, card.type);
      const svg = svgOf(proceduralArtUri(card.id, theme, compositionOf(card.id, card.type), false, motifFor(card.name, theme), motion));
      for (const role of ART_ROLES) if (motion?.[role] !== undefined) expect(svg, `${card.id}: ${role}`).toContain(`class="art-m-${role}"`);
      expect(motion?.props, card.id).toBeDefined();
      expect(specOf(card, false).ridges.some((ridge) => ridge.role === "props"), card.id).toBe(true);
    }
  });

  it("R3401000 the emblem stands small inside the shown band", () => {
    for (const card of ENTRIES) {
      const { emblem } = recipe(card.id);
      expect(emblem, card.id).toBeDefined();
      if (emblem === undefined) continue;
      expect(emblem.size, card.id).toBeLessThanOrEqual(SHOWN_BAND.size);
      expect(emblem.x, card.id).toBeGreaterThanOrEqual(SHOWN_BAND.left);
      expect(emblem.x, card.id).toBeLessThanOrEqual(SHOWN_BAND.right);
      expect(emblem.y, card.id).toBeGreaterThanOrEqual(SHOWN_BAND.top);
      expect(emblem.y, card.id).toBeLessThanOrEqual(SHOWN_BAND.bottom);
      const spec = specOf(card, false);
      expect([spec.emblem.x, spec.emblem.y, spec.emblem.size], card.id).toEqual([emblem.x, emblem.y, emblem.size]);
    }
  });

  it("R3401000 Alt art grows the same parts from its own seed", () => {
    // R3404000: the 54 offered Alt art draw an alt recipe of their own instead (meditativeAltScenes.test.tsx).
    const reseeded = ENTRIES.filter((card) => !Object.hasOwn(ALT_SCENE_RECIPES, card.id));
    expect(reseeded).toHaveLength(78);
    for (const card of reseeded) {
      const scene = recipe(card.id);
      const normal = composeScene(scene, card.id);
      const seed = artSeedOf(card.id, "alt");
      expect(sceneRecipeOf(seed), card.id).toBe(scene);
      const own = composeScene(scene, seed);
      expect(new Set(own.map((shape) => shape.part)), card.id).toEqual(new Set(normal.map((shape) => shape.part)));
      expect(sceneUse(own), card.id).toEqual(sceneUse(normal));
      expect(own.map((shape) => shape.d), card.id).not.toEqual(normal.map((shape) => shape.d));
    }
  });

  it("R3401000 no Meditative base face paints in Radiant's gold window", () => {
    for (const card of ENTRIES) {
      const spec = specOf(card, false);
      const colors = [spec.sky.from, spec.sky.to, spec.glow.color, ...spec.ridges.flatMap((ridge) => [ridge.fill, ...(ridge.stroke === undefined ? [] : [ridge.stroke])])];
      for (const hex of colors) expect(inRadiantGold(hex), `${card.id}: ${hex}`).toBe(false);
    }
  });

  it("R3401000 the kit modules #3401 added each hold parts of their own, every one in SCENE_KIT", () => {
    const added = INK_MODULES.map((kit) => Object.entries(kit));
    for (const entries of added) expect(entries.length).toBeGreaterThan(0);
    const ink = added.flat();
    // Classic+'s modules (R3390000), then Meditative+'s, Training Grounds' and Legacy's (R3393000), then these.
    const before = [PLACES_KIT, OBJECTS_KIT, NATURE_KIT, MOUNTAIN_KIT, YARD_KIT, TOUCHES_KIT].flatMap((kit) => Object.keys(kit));
    const ids = [...before, ...ink.map(([id]) => id)];
    expect(new Set(ids).size, "no id in two modules").toBe(ids.length);
    for (const [id, part] of ink) expect(SCENE_KIT[id as keyof typeof SCENE_KIT], id).toBe(part);
    // None takes the place of one of the first kit's parts or of an earlier module's; R3403000: Stinkyscape's four
    // modules follow them (stinkyscape91Scenes.test.ts), and R3406000 the Stinkyscape Alt art pass's
    // (stinkyscape91AltScenes.test.tsx).
    const after = [SPACE_KIT, CRAFT_KIT, GEAR_KIT, COSMOS_KIT, STINKY_ALT_KIT].flatMap((kit) => Object.keys(kit));
    expect(Object.keys(SCENE_KIT)).toHaveLength(FIRST_KIT_PARTS + ids.length + after.length);
    expect(Object.keys(SCENE_KIT).slice(FIRST_KIT_PARTS + before.length, FIRST_KIT_PARTS + ids.length)).toEqual(ink.map(([id]) => id));
  });
});
