// R3403000: every entry of #3403's table (Stinkyscape #91 to #176 and their tokens, and the tokens of the early
// cards it names) drawn from a scene recipe of its own (stinkyscape91Recipes1.ts to stinkyscape91Recipes3.ts), base
// and Radiant, in Stinkyscape's deep-space look, at or above its rarity's detail floor and off the floor's short list;
// every Unit a character or a machine of its own; the cards other issues draw left out; and the kit modules #3403
// added beside the others. The pixels are e2e/cypress/component/stinkyscape91-scenes.cy.tsx's.

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
import { FIGURE_KIT, figureLookFor, type FigurePartId } from "./figureKit.ts";
import { artSeedOf } from "./pictures.ts";
import { artSpec, type ArtSpec } from "./procedural.ts";
import { ART_DETAIL_MIN, ART_DETAIL_SHORT, artElements } from "./richness.ts";
import { composeScene, SCENE_LAYERS, sceneRecipeOf, sceneUse, type KitCategory, type SceneLayer, type SceneRecipe } from "./scene.ts";
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
import { TOUCHES_KIT } from "./sceneKitTouches.ts";
import { YARD_KIT } from "./sceneKitYard.ts";
import { ALT_SCENE_RECIPES, STINKYSCAPE_91_RECIPES } from "./sceneRecipes.ts";
import { STARSHIP_PIECES } from "./stinkyscape.ts";
import { compositionFor, themeFor } from "./themes.ts";

const SVG_PREFIX = "data:image/svg+xml,";

/** #3403's table: every entry the pass draws, by id, sorted. */
const TABLE: readonly string[] = [
  ...["008-1", "010-1", "010-2", "010-3", "011-1", "011-2", "011-3", "012-1", "012-2", "012-3", "018-1", "018-2", "018-3", "084-1"],
  ...["091", "092", "094", "095", "096", "097", "099", "102", "103", "109", "110", "111", "112", "112-1", "113", "114", "115"],
  ...["116", "117", "118", "119", "120", "121", "123", "124", "125", "126", "130", "135", "135-1", "137", "137-1", "137-2"],
  ...["137-3", "137-4", "138", "139", "144", "145", "146", "147", "147-1", "147-2", "148", "148-1", "149", "150", "150-1"],
  ...["151", "152", "153", "155", "156", "157", "158", "159", "160", "161", "162", "163", "164", "165", "166", "167", "168"],
  ...["169", "170", "171", "172", "173", "174", "176", "t-drone-interceptor", "t-drone-corvette", "t-reknite", "t-korrelite"],
  ...["t-gellium", "t-axnit", "t-narcor", "t-red-narcor", "t-ai-01", "t-ai-02", "t-ai-03", "t-ai-04", "t-ai-05", "t-ai-06"],
  ...["t-ai-07", "t-ai-08", "t-ai-09"],
]
  .map((id) => `stinkyscape-${id}`)
  .sort();

/** The cards the table leaves to other issues: the Starship Pieces and what prints the keyword when Embiggened (#3333), the Drone Battleship's layers (#3336), Anomaly Found! (#3337), the Terminus tokens and Dice of Doom (#3338). */
const OWNED_ELSEWHERE: readonly string[] = [
  ...STARSHIP_PIECES,
  ...["127", "128", "129", "t-dormant-starship", "t-shield-layer", "t-propagation-layer", "t-meltdown-layer", "t-anomaly-found"].map((id) => `stinkyscape-${id}`),
  ...["004-1", "004-2", "004-3", "004-4", "004-5", "004-6", "154"].map((id) => `stinkyscape-${id}`),
];

/** The Units drawn as a creature or a machine of the scene kit, with no figure, and the part each stands as. */
const MACHINE_UNITS: Readonly<Record<string, string>> = {
  "stinkyscape-091": "space-wasp",
  "stinkyscape-092": "space-wasp",
  "stinkyscape-116": "space-station",
  "stinkyscape-135-1": "kicking-sack",
  "stinkyscape-150-1": "scrap-golem",
  "stinkyscape-169": "asteroid",
  "stinkyscape-170": "asteroid",
  "stinkyscape-171": "asteroid",
  "stinkyscape-172": "giant-asteroid",
  "stinkyscape-173": "spice-tower",
  "stinkyscape-t-drone-interceptor": "interceptor",
  "stinkyscape-t-drone-corvette": "corvette",
  "stinkyscape-t-ai-01": "escort-disc",
  "stinkyscape-t-ai-07": "combat-robot",
};

/** The figure parts #3403 added, each worn or held by at least one of its scenes. */
const FIGURE_PARTS_ADDED: readonly FigurePartId[] = [
  "neural-crown",
  "space-helmet",
  "hair-silver",
  "top-hat",
  "outfit-flightsuit",
  "outfit-greatcoat",
  "outfit-sleeveless",
  "outfit-plated-parka",
  "outfit-frock",
  "outfit-cloak",
  "outfit-harness",
  "item-shock-axe",
  "item-dart",
  "item-orb",
  "item-slate",
  "item-torch",
  "item-vial",
];

const SCENERY: readonly KitCategory[] = ["terrain", "architecture", "plants", "creatures", "objects", "weather"];
/** The rarer than Rare: a scene with a foreground, a middle and a background. */
const DEEP: readonly Rarity[] = ["Epic", "Legendary", "Mythic"];
/** Where an emblem may stand and how big it may be: inside the band every face's window shows (motifs.test.ts's). */
const EMBLEM_BAND = { left: 10, right: 90, top: 22, bottom: 84, size: 8 } as const;
/** #3401's modules of ink-wash parts, spread into `SCENE_KIT` before the ones #3403 added. */
const INK_MODULES = [INK_PLACES_KIT, INK_BUILDINGS_KIT, INK_OBJECTS_KIT, INK_CURIOS_KIT, INK_MACHINES_KIT, INK_NATURE_KIT, INK_CREATURES_KIT, INK_CHARACTERS_KIT];
/** The modules of parts #3403 added. */
const STINKY_KITS = [SPACE_KIT, CRAFT_KIT, GEAR_KIT, COSMOS_KIT] as const;

const ENTRIES: readonly CardDef[] = TABLE.map((id) => {
  const card = CATALOG[id];
  if (card === undefined) throw new Error(`the catalog has no ${id}`);
  return card;
});

function recipe(id: string): SceneRecipe {
  const found = sceneRecipeOf(id);
  if (found === null) throw new Error(`no recipe for ${id}`);
  return found;
}

function svgOf(uri: string): string {
  return decodeURIComponent(uri.slice(SVG_PREFIX.length));
}

/** A card's spec as the Stinkyscape painter hands it to the scene (svg.ts `sceneArtSvg`), its recipe read from its id. */
function specOf(card: CardDef, radiant: boolean): ArtSpec {
  return artSpec(card.id, themeFor(card.tags, card.type), compositionFor(card.type), radiant, null);
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

/** Every part a recipe names: its placements', its figure's and its extras'. */
function partsOf(scene: SceneRecipe): Set<string> {
  const parts = new Set<string>();
  for (const layer of SCENE_LAYERS) for (const placed of scene.layers[layer] ?? []) parts.add(placed.part);
  for (const figure of [...(scene.figure === undefined ? [] : [scene.figure]), ...(scene.extras ?? [])]) {
    const { look } = figure;
    for (const part of [figure.pose, look.head, look.hair, look.outfit, look.item]) if (part !== null) parts.add(part);
  }
  return parts;
}

describe("R3403000 Stinkyscape's later cards drawn as scenes", () => {
  it("R3403000 every entry of the table draws from a recipe of its own in Stinkyscape's look", () => {
    expect(TABLE).toHaveLength(103);
    expect(Object.keys(STINKYSCAPE_91_RECIPES).sort()).toEqual(TABLE);
    const rarities = new Map<Rarity, number>();
    for (const card of ENTRIES) rarities.set(card.rarity, (rarities.get(card.rarity) ?? 0) + 1);
    expect(Object.fromEntries(rarities)).toEqual({ Token: 41, Common: 10, Rare: 21, Epic: 16, Legendary: 11, Mythic: 4 });
    const seen = new Map<string, string>();
    for (const card of ENTRIES) {
      const scene = recipe(card.id);
      expect(scene, card.id).toBe(STINKYSCAPE_91_RECIPES[card.id]);
      expect(scene.family, card.id).toBe("stinkyscape");
      const key = JSON.stringify(scene);
      expect(seen.get(key), `${card.id} draws ${seen.get(key) ?? ""}'s recipe`).toBeUndefined();
      seen.set(key, card.id);
      // Stinkyscape's fine black line outlines the scene's solid shapes.
      expect(composeScene(scene, card.id).some((shape) => shape.inked === true), card.id).toBe(true);
    }
    // The cards other issues draw keep the pictures they have.
    expect(OWNED_ELSEWHERE.length).toBeGreaterThan(40);
    for (const id of OWNED_ELSEWHERE) {
      expect(CATALOG[id], id).toBeDefined();
      expect(sceneRecipeOf(id), id).toBeNull();
    }
  });

  it("R3403000 every entry clears its rarity's floor and is off ART_DETAIL_SHORT", () => {
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
  });

  it("R3403000 base and Radiant draw one geometry, the Radiant face gilded", () => {
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

  it("R3403000 every Unit is a character or a machine of its own", () => {
    const units = ENTRIES.filter((card) => card.type === "Unit");
    expect(units).toHaveLength(27);
    expect(Object.keys(MACHINE_UNITS)).toHaveLength(14);
    const looks = new Map<string, string>();
    for (const card of units) {
      const scene = recipe(card.id);
      const machine = MACHINE_UNITS[card.id];
      if (machine !== undefined) {
        expect(scene.figure, card.id).toBeUndefined();
        expect(scene.layers.subject?.map((placed) => placed.part), card.id).toContain(machine);
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
    expect(looks.size).toBe(13);
  });

  it("R3403000 extra figures stand in their layers", () => {
    let crowds = 0;
    for (const card of ENTRIES) {
      const scene = recipe(card.id);
      const extras = scene.extras ?? [];
      if (extras.length === 0) continue;
      crowds += 1;
      const shapes = composeScene(scene, card.id);
      expect(shapes.length, card.id).toBeGreaterThan(composeScene({ ...scene, extras: [] }, card.id).length);
      for (const extra of extras) {
        expect(shapes.some((shape) => shape.layer === extra.layer && shape.part === extra.pose), `${card.id} ${extra.layer}`).toBe(true);
      }
    }
    expect(crowds).toBeGreaterThanOrEqual(10);
  });

  it("R3403000 the emblem stands small inside the shown band", () => {
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

  it("R3403000 Alt art grows the same parts from its own seed", () => {
    // R3406000: an entry of #3406's table draws an alt recipe of its own instead (stinkyscape91AltScenes.test.tsx).
    const regrown = ENTRIES.filter((card) => !Object.hasOwn(ALT_SCENE_RECIPES, card.id));
    expect(regrown).toHaveLength(67);
    for (const card of regrown) {
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

  it("R3403000 no base face of the table paints in Radiant's gold window", () => {
    for (const card of ENTRIES) {
      const spec = specOf(card, false);
      const colors = [spec.sky.from, spec.sky.to, spec.glow.color, ...spec.ridges.flatMap((ridge) => [ridge.fill, ...(ridge.stroke === undefined ? [] : [ridge.stroke])])];
      for (const hex of colors) expect(inRadiantGold(hex), `${card.id}: ${hex}`).toBe(false);
    }
  });

  it("R3403000 the Stinkyscape kit modules are scenery of their own, every part used by the pass", () => {
    const added = STINKY_KITS.flatMap((kit) => Object.entries(kit));
    expect(added.length).toBeGreaterThan(0);
    expect(new Set(added.map(([id]) => id)).size, "no id in two modules").toBe(added.length);
    const older = new Set(
      [PLACES_KIT, OBJECTS_KIT, NATURE_KIT, MOUNTAIN_KIT, YARD_KIT, TOUCHES_KIT, ...INK_MODULES].flatMap((kit) => Object.keys(kit)),
    );
    for (const [id, part] of added) {
      expect(SCENERY, id).toContain(part.category);
      expect(older.has(id), `${id} is one of #3390's, #3393's or #3401's parts`).toBe(false);
      expect(Object.hasOwn(FIGURE_KIT, id), `${id} is a figure part`).toBe(false);
      // Each joins the kit as itself, so none takes the place of another part.
      expect(SCENE_KIT[id as keyof typeof SCENE_KIT], id).toBe(part);
    }
    const used = new Set<string>();
    for (const card of ENTRIES) for (const part of partsOf(recipe(card.id))) used.add(part);
    expect(added.map(([id]) => id).filter((id) => !used.has(id)), "parts no scene of the pass draws").toEqual([]);
    for (const part of FIGURE_PARTS_ADDED) expect(Object.hasOwn(FIGURE_KIT, part), part).toBe(true);
    expect(FIGURE_PARTS_ADDED.filter((part) => !used.has(part)), "figure parts no scene of the pass wears").toEqual([]);
  });
});
