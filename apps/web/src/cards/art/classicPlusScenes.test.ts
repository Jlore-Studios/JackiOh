// R3390000: every Classic+ entry, tokens included, drawn from a scene recipe of its own (classicPlusRecipes1.ts to
// classicPlusRecipes3.ts), base and Radiant, in Classic's temple-stone look, at or above its rarity's detail floor
// and off the floor's short list; every Unit a character of its own; a scene's extra figures and props; and the
// kit modules #3390 added beside the first kit. The pixels are e2e/cypress/component/classicplus-scenes.cy.tsx's.

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
import { FIGURE_KIT, figureLookFor } from "./figureKit.ts";
import { motifFor } from "./motifs.ts";
import { ART_ROLES, FULL_ART_CARD_MOTION, FULL_ART_MOTION } from "./motion.ts";
import { artSeedOf } from "./pictures.ts";
import { artSpec, type ArtSpec } from "./procedural.ts";
import { ART_DETAIL_MIN, ART_DETAIL_SHORT, artElements } from "./richness.ts";
import { composeScene, SCENE_LAYERS, sceneRecipeOf, sceneUse, type SceneLayer, type SceneRecipe } from "./scene.ts";
import { SCENE_KIT } from "./sceneKit.ts";
import { NATURE_KIT } from "./sceneKitNature.ts";
import { OBJECTS_KIT } from "./sceneKitObjects.ts";
import { PLACES_KIT } from "./sceneKitPlaces.ts";
import { ALT_SCENE_RECIPES } from "./sceneRecipes.ts";
import { proceduralArtUri } from "./svg.ts";
import { compositionOf, themeFor } from "./themes.ts";

const SVG_PREFIX = "data:image/svg+xml,";
/** Every Classic+ entry, its tokens included. */
const ENTRIES: readonly CardDef[] = Object.values(CATALOG).filter((card) => card.set === "Classic+");
/** The rarer than Rare: a scene with a foreground, a middle and a background. */
const DEEP: readonly Rarity[] = ["Epic", "Legendary", "Mythic"];
/** The Units drawn as a creature or a machine of the scene kit, with no figure: the snake, the worm, the jet, the beast. */
const CREATURE_UNITS: Readonly<Record<string, string>> = {
  "classicplus-003": "snake",
  "classicplus-039": "grub",
  "classicplus-051": "jet",
  "classicplus-064": "boar",
};
/** The parts sceneKit.ts draws itself: the first kit's 56 (#3331) and the 199 Core's scenes added beside them (R3392000). */
const FIRST_KIT_PARTS = 56 + 199;
/** Where an emblem may stand and how big it may be: inside the band every face's window shows (motifs.test.ts's). */
const EMBLEM_BAND = { left: 10, right: 90, top: 22, bottom: 84, size: 8 } as const;

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

describe("R3390000 Classic+ drawn as scenes", () => {
  it("R3390000 every Classic+ entry, tokens included, draws from a recipe of its own in Classic's look", () => {
    expect(ENTRIES).toHaveLength(116);
    expect(ENTRIES.filter((card) => card.token)).toHaveLength(38);
    const seen = new Map<string, string>();
    for (const card of ENTRIES) {
      const scene = recipe(card.id);
      expect(scene.family, card.id).toBe("classic");
      const key = JSON.stringify(scene);
      expect(seen.get(key), `${card.id} draws ${seen.get(key) ?? ""}'s recipe`).toBeUndefined();
      seen.set(key, card.id);
      // Classic's slate ink outlines the scene's solid shapes.
      expect(composeScene(scene, card.id).some((shape) => shape.inked === true), card.id).toBe(true);
    }
  });

  it("R3390000 every Classic+ entry clears its rarity's floor and is off ART_DETAIL_SHORT", () => {
    for (const card of ENTRIES) {
      const floor = ART_DETAIL_MIN[card.rarity];
      const shapes = composeScene(recipe(card.id), card.id);
      expect(artElements(svgOf(proceduralCardArtUri(card, false))), card.id).toBeGreaterThanOrEqual(floor.elements);
      expect(sceneUse(shapes).layers, card.id).toBeGreaterThanOrEqual(floor.layers);
      const layers = new Set<SceneLayer>(shapes.map((shape) => shape.layer));
      expect(layers, card.id).toContain("subject");
      if (DEEP.includes(card.rarity)) for (const layer of ["far", "mid", "subject", "near"] as const) expect(layers, `${card.id} ${layer}`).toContain(layer);
      expect(ART_DETAIL_SHORT.has(card.id), card.id).toBe(false);
    }
    expect([...ART_DETAIL_SHORT].filter((id) => id.startsWith("classicplus-"))).toEqual([]);
  });

  it("R3390000 base and Radiant draw one geometry, the Radiant face gilded", () => {
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

  it("R3390000 every Classic+ Unit is a character of its own", () => {
    const units = ENTRIES.filter((card) => card.type === "Unit");
    expect(units).toHaveLength(34);
    const looks = new Map<string, string>();
    for (const card of units) {
      const scene = recipe(card.id);
      const creature = CREATURE_UNITS[card.id];
      if (creature !== undefined) {
        expect(scene.figure, card.id).toBeUndefined();
        expect(scene.layers.subject?.map((placed) => placed.part), card.id).toContain(creature);
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
    expect(looks.size).toBe(units.length - Object.keys(CREATURE_UNITS).length);
  });

  it("R3390000 extra figures stand in their layers and props move outside the subject layer", () => {
    let crowds = 0;
    for (const card of ENTRIES) {
      const scene = recipe(card.id);
      for (const layer of SCENE_LAYERS) {
        for (const placed of scene.layers[layer] ?? []) if (placed.props === true) expect(layer, `${card.id} ${placed.part}`).not.toBe("subject");
      }
      const extras = scene.extras ?? [];
      if (extras.length === 0) continue;
      crowds += 1;
      const shapes = composeScene(scene, card.id);
      expect(shapes.length, card.id).toBeGreaterThan(composeScene({ ...scene, extras: [] }, card.id).length);
      for (const extra of extras) {
        expect(shapes.some((shape) => shape.layer === extra.layer && Object.hasOwn(FIGURE_KIT, shape.part) && shape.part === extra.pose), `${card.id} ${extra.layer}`).toBe(true);
      }
    }
    // League of Losers stands its five Losers, and others their crowds.
    expect(recipe("classicplus-019").extras?.length ?? 0).toBeGreaterThanOrEqual(4);
    expect(crowds).toBeGreaterThan(1);
    // A Classic+ full-art card moves every layer its motion names (R2715001), its props among them.
    // The full-art cards' own motions (R2715001), not R3340000's recipes for every other Legendary and Mythic.
    const fullArt = ENTRIES.filter((card) => FULL_ART_CARD_MOTION[card.id] !== undefined);
    expect(fullArt.map((card) => card.id).sort()).toEqual(
      ["classicplus-012", "classicplus-019", "classicplus-026", "classicplus-046", "classicplus-048", "classicplus-062", "classicplus-069", "classicplus-075"].sort(),
    );
    for (const card of fullArt) {
      const motion = FULL_ART_MOTION[card.id] ?? null;
      const theme = themeFor(card.tags, card.type);
      const svg = svgOf(proceduralArtUri(card.id, theme, compositionOf(card.id, card.type), false, motifFor(card.name, theme), motion));
      for (const role of ART_ROLES) if (motion?.[role] !== undefined) expect(svg, `${card.id}: ${role}`).toContain(`class="art-m-${role}"`);
      if (motion?.props !== undefined) expect(specOf(card, false).ridges.some((ridge) => ridge.role === "props"), card.id).toBe(true);
    }
  });

  it("R3390000 the emblem stands small inside the shown band", () => {
    for (const card of ENTRIES) {
      const { emblem } = recipe(card.id);
      expect(emblem, card.id).toBeDefined();
      if (emblem === undefined) continue;
      expect(emblem.size, card.id).toBeLessThanOrEqual(EMBLEM_BAND.size);
      expect(emblem.x, card.id).toBeGreaterThanOrEqual(EMBLEM_BAND.left);
      expect(emblem.x, card.id).toBeLessThanOrEqual(EMBLEM_BAND.right);
      expect(emblem.y, card.id).toBeGreaterThanOrEqual(EMBLEM_BAND.top);
      expect(emblem.y, card.id).toBeLessThanOrEqual(EMBLEM_BAND.bottom);
      const spec = specOf(card, false);
      expect([spec.emblem.x, spec.emblem.y, spec.emblem.size], card.id).toEqual([emblem.x, emblem.y, emblem.size]);
    }
  });

  it("R3390000 Alt art grows the same parts from its own seed", () => {
    // R3396000: the 40 offered Alt art draw an alt recipe of their own instead (classicPlusAltScenes.test.tsx).
    const reseeded = ENTRIES.filter((card) => !Object.hasOwn(ALT_SCENE_RECIPES, card.id));
    // R3397000: the table holds Meditative+'s and Training Grounds' too (setAltScenes.test.tsx); R3404000: and
    // Meditative's (meditativeAltScenes.test.tsx).
    expect(reseeded).toHaveLength(ENTRIES.length - Object.keys(ALT_SCENE_RECIPES).filter((id) => CATALOG[id]?.set === "Classic+").length);
    expect(reseeded).toHaveLength(76);
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

  it("R3390000 no Classic+ base face paints in Radiant's gold window", () => {
    for (const card of ENTRIES) {
      const spec = specOf(card, false);
      const colors = [spec.sky.from, spec.sky.to, spec.glow.color, ...spec.ridges.flatMap((ridge) => [ridge.fill, ...(ridge.stroke === undefined ? [] : [ridge.stroke])])];
      for (const hex of colors) expect(inRadiantGold(hex), `${card.id}: ${hex}`).toBe(false);
    }
  });

  it("R3390000 the kit modules #3390 added each hold parts of their own, every one in SCENE_KIT", () => {
    const modules = [PLACES_KIT, OBJECTS_KIT, NATURE_KIT].map((kit) => Object.entries(kit));
    const added = modules.flat();
    expect(new Set(added.map(([id]) => id)).size, "no id in two modules").toBe(added.length);
    for (const entries of modules) expect(entries.length).toBeGreaterThan(0);
    for (const [id, part] of added) expect(SCENE_KIT[id as keyof typeof SCENE_KIT], id).toBe(part);
    // None takes the place of one of the first kit's parts: they follow its own in order (the modules #3393 and #3401
    // added follow them, and meditativeScenes.test.ts counts the whole kit).
    expect(Object.keys(SCENE_KIT).slice(FIRST_KIT_PARTS, FIRST_KIT_PARTS + added.length)).toEqual(added.map(([id]) => id));
  });
});
