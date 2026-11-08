// R3393000, R3393001: every Meditative+ and Training Grounds entry, tokens included, drawn as a scene of its own
// from its brief in issue #3393's table (sceneRecipesMeditativePlus.ts, sceneRecipesTrainingGrounds.ts,
// sceneRecipesLegacy.ts): each draws its recipe on both faces with one geometry, in its family's look, clears its
// rarity's floor and is off `ART_DETAIL_SHORT`; every Unit is a figure-kit character; a crowd figure and a fade draw
// as scene.ts says, and the proof cards draw what they drew before. A Legacy card draws its own scene with its yard
// touch, wearing its Core card's frozen emblem. The pixels are e2e/cypress/component/set-scenes.cy.tsx's.

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef, Rarity } from "@jackioh/shared";

import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { EMBLEMS } from "./emblems.ts";
import { FIGURE_DEFAULT_LOOK, type FigureLook } from "./figureKit.ts";
import { hashId } from "./hash.ts";
import { motifFor } from "./motifs.ts";
import { artSeedOf } from "./pictures.ts";
import { artSpec } from "./procedural.ts";
import { ART_DETAIL_MAX_BYTES, ART_DETAIL_MIN, ART_DETAIL_SHORT, artElements } from "./richness.ts";
import {
  composeScene,
  SCENE_LAYERS,
  SCENE_PALETTE,
  SCENE_RADIANT_GOLD,
  SCENE_RADIANT_MIX,
  sceneRecipeOf,
  sceneUse,
  type FigureRecipe,
  type SceneLayer,
  type SceneRecipe,
} from "./scene.ts";
import { SCENE_KIT } from "./sceneKit.ts";
import { SCENE_RECIPES } from "./sceneRecipes.ts";
import { LEGACY_RECIPES, LEGACY_TOUCH_PARTS } from "./sceneRecipesLegacy.ts";
import { MEDITATIVE_PLUS_RECIPES } from "./sceneRecipesMeditativePlus.ts";
import { TRAINING_GROUNDS_RECIPES } from "./sceneRecipesTrainingGrounds.ts";
import { legacyArtUri } from "./svg.ts";
import { compositionFor, mixHex, themeFor } from "./themes.ts";
import { LEGACY_PREFIX, LEGACY_TOUCHES, legacySvg, snapshotArtOf, type LegacyTouch } from "./traininggrounds.ts";

const SVG_PREFIX = "data:image/svg+xml,";
/** The pass: every catalog entry of the two sets, tokens included. */
const PASS: readonly CardDef[] = Object.values(CATALOG).filter((card) => card.set === "Meditative+" || card.set === "Training Grounds");
const MEDITATIVE_PLUS = PASS.filter((card) => card.set === "Meditative+");
/** The Legacy cards, each a snapshot of a Core card (R2724000). */
const LEGACY = PASS.filter((card) => snapshotArtOf(card.id) !== null);
const OWN = PASS.filter((card) => snapshotArtOf(card.id) === null);
const TRAINING_GROUNDS = OWN.filter((card) => card.set === "Training Grounds");
/** The rarer than Rare: a scene with a foreground, a middle and a background. */
const DEEP: readonly Rarity[] = ["Epic", "Legendary", "Mythic"];
/** What a face may never print (art.test.ts's checks). */
const UNCLEAN = /NaN|undefined|Infinity|<text|<title|href|<image/;

/**
 * R3393000: each proof card's base and Radiant picture as `main` drew it before this pass (`hashId` of its data URI),
 * so the crowd and the fade leave every recipe that uses neither as it was.
 */
const PROOF_PINS: Readonly<Record<string, readonly [number, number]>> = {
  "classic-074": [1210823500, 2735167760],
  "core-002": [3382649194, 3484642400],
  "core-098": [344479599, 1087118204],
  "core-100": [894252645, 4246782103],
  "stinkyscape-015": [4157590552, 724339904],
};

function recipe(id: string): SceneRecipe {
  const found = sceneRecipeOf(id);
  if (found === null) throw new Error(`no recipe for ${id}`);
  return found;
}

function svgOf(uri: string): string {
  return decodeURIComponent(uri.slice(SVG_PREFIX.length));
}

function face(card: CardDef, radiant: boolean): string {
  return svgOf(proceduralCardArtUri(card, radiant));
}

/** Every placement of a recipe, in any layer, with its layer. */
function placements(scene: SceneRecipe): { readonly layer: SceneLayer; readonly part: string; readonly count: number }[] {
  return SCENE_LAYERS.flatMap((layer) => (scene.layers[layer] ?? []).map((placed) => ({ layer, part: placed.part, count: placed.count ?? 1 })));
}

function lookParts(look: FigureLook): string[] {
  return [look.head, look.hair, look.outfit, look.item].filter((part): part is NonNullable<typeof part> => part !== null);
}

/** The Core card a Legacy card snapshots, and its name. */
function coreOf(card: CardDef): { readonly coreId: string; readonly name: string } {
  const coreId = snapshotArtOf(card.id);
  if (coreId === null) throw new Error(`${card.id} is no Legacy card`);
  return { coreId, name: card.name.replace(LEGACY_PREFIX, "") };
}

describe("R3393000 Meditative+ and Training Grounds drawn as scenes", () => {
  it("R3393000 the pass is the issue's 94 entries, each with a recipe of its own, off ART_DETAIL_SHORT", () => {
    expect(PASS).toHaveLength(94);
    expect(MEDITATIVE_PLUS).toHaveLength(33);
    expect(TRAINING_GROUNDS).toHaveLength(41);
    expect(LEGACY).toHaveLength(20);
    expect(Object.keys(MEDITATIVE_PLUS_RECIPES).sort()).toEqual(MEDITATIVE_PLUS.map((card) => card.id).sort());
    expect(Object.keys(TRAINING_GROUNDS_RECIPES).sort()).toEqual(TRAINING_GROUNDS.map((card) => card.id).sort());
    expect(Object.keys(LEGACY_RECIPES).sort()).toEqual(LEGACY.map((card) => card.id).sort());
    const seen = new Set<SceneRecipe>();
    for (const card of PASS) {
      expect(Object.hasOwn(SCENE_RECIPES, card.id), card.id).toBe(true);
      const scene = recipe(card.id);
      expect(seen.has(scene), `${card.id} shares its recipe`).toBe(false);
      seen.add(scene);
      expect(ART_DETAIL_SHORT.has(card.id), card.id).toBe(false);
    }
  });

  it("R3393000 each draws its scene on both faces with one geometry, clean", () => {
    for (const card of PASS) {
      const shapes = composeScene(recipe(card.id), card.id);
      const base = face(card, false);
      const radiant = face(card, true);
      for (const shape of shapes) {
        expect(base.includes(`d="${shape.d}" fill="${SCENE_PALETTE[shape.paint]}"`), `${card.id} base ${shape.part}`).toBe(true);
        const gilt = mixHex(SCENE_PALETTE[shape.paint], SCENE_RADIANT_GOLD[shape.layer], SCENE_RADIANT_MIX);
        expect(radiant.includes(`d="${shape.d}" fill="${gilt}"`), `${card.id} radiant ${shape.part}`).toBe(true);
      }
      for (const svg of [base, radiant]) {
        expect(svg.match(/<svg/g)?.length, card.id).toBe(1);
        expect(svg, card.id).not.toMatch(UNCLEAN);
      }
      expect(radiant, card.id).not.toBe(base);
    }
  });

  it("R3393000 Meditative+ paints in ink wash under one red seal, Training Grounds in carved wood", () => {
    for (const card of PASS) {
      const scene = recipe(card.id);
      const seals = placements(scene).filter((placed) => placed.part === "seal");
      if (card.set === "Meditative+") {
        expect(scene.family, card.id).toBe("meditative");
        expect(seals.map((placed) => [placed.layer, placed.count]), card.id).toEqual([["near", 1]]);
        expect(placements(scene).some((placed) => placed.part === "peaks"), `${card.id} peaks`).toBe(true);
      } else {
        expect(scene.family, card.id).toBe("training-grounds");
        expect(seals, card.id).toEqual([]);
      }
    }
    // The seal is the Meditative family's alone.
    for (const [id, scene] of Object.entries(SCENE_RECIPES)) if (scene.family !== "meditative") expect(placements(scene).some((placed) => placed.part === "seal"), id).toBe(false);
  });

  it("R3393000 every Unit is a character from the figure kit", () => {
    const units = PASS.filter((card) => card.type === "Unit");
    expect(units.length).toBeGreaterThan(0);
    for (const card of units) {
      const scene = recipe(card.id);
      expect(scene.figure, card.id).toBeDefined();
      expect(scene.ownSubject, card.id).toBeUndefined();
      if (scene.figure === undefined) continue;
      expect(scene.figure.look, card.id).not.toEqual(FIGURE_DEFAULT_LOOK);
      const drawn = new Set<string>(composeScene(scene, card.id).map((shape) => shape.part));
      for (const part of [scene.figure.pose, ...lookParts(scene.figure.look)]) expect(drawn, `${card.id} ${part}`).toContain(part);
    }
    // No card of the pass keeps its painter's own subject.
    for (const card of PASS) expect(recipe(card.id).ownSubject, card.id).toBeUndefined();
  });

  it("R3393000 each clears its floor; an Epic or rarer uses far, mid and near", () => {
    for (const card of PASS) {
      const floor = ART_DETAIL_MIN[card.rarity];
      const shapes = composeScene(recipe(card.id), card.id);
      expect(artElements(face(card, false)), card.id).toBeGreaterThanOrEqual(floor.elements);
      expect(sceneUse(shapes).layers, card.id).toBeGreaterThanOrEqual(floor.layers);
      const layers = new Set<SceneLayer>(shapes.map((shape) => shape.layer));
      expect(layers, card.id).toContain("subject");
      if (DEEP.includes(card.rarity)) for (const layer of ["far", "mid", "near"] as const) expect(layers, `${card.id} ${layer}`).toContain(layer);
      for (const radiant of [false, true]) expect(proceduralCardArtUri(card, radiant).length, card.id).toBeLessThanOrEqual(ART_DETAIL_MAX_BYTES);
    }
  });

  it("R3393000 no two scenes of the pass share their subject", () => {
    const subjects = new Map<string, string>();
    for (const card of PASS) {
      const scene = recipe(card.id);
      const parts = (scene.layers.subject ?? []).map((placed) => placed.part).sort();
      const figure = scene.figure === undefined ? null : [scene.figure.pose, scene.figure.look];
      const key = JSON.stringify([parts, figure, (scene.crowd ?? []).map((extra) => [extra.pose, extra.look])]);
      expect(subjects.get(key), `${card.id} draws the subject of ${subjects.get(key) ?? ""}`).toBeUndefined();
      subjects.set(key, card.id);
    }
  });

  it("R3393000 Alt art and a repaint grow the same parts from their own seed", () => {
    expect(OWN).toHaveLength(74);
    for (const card of OWN) {
      const scene = recipe(card.id);
      const normal = composeScene(scene, card.id);
      for (const picture of ["alt", "night"] as const) {
        const seed = artSeedOf(card.id, picture);
        expect(seed, card.id).not.toBe(card.id);
        expect(sceneRecipeOf(seed), card.id).toBe(scene);
        const own = composeScene(scene, seed);
        expect(new Set(own.map((shape) => shape.part)), `${card.id} ${picture}`).toEqual(new Set(normal.map((shape) => shape.part)));
        expect(sceneUse(own), `${card.id} ${picture}`).toEqual(sceneUse(normal));
        expect(own.map((shape) => shape.d), `${card.id} ${picture}`).not.toEqual(normal.map((shape) => shape.d));
      }
    }
  });

  it("R3393000 a crowd figure stands at the end of its layer, a fade scales opacity, and the proofs draw what they drew before", () => {
    const standing: FigureRecipe = { x: 30, y: 80, scale: 0.6, pose: "pose-stand", look: FIGURE_DEFAULT_LOOK };
    const extra: FigureRecipe = { ...standing, layer: "mid" };
    const plain: SceneRecipe = {
      family: "core",
      sky: ["blue-800", "sky-300"],
      light: { x: 50, y: 50, r: 30, paint: "white", opacity: 0.3 },
      crowd: [extra],
      layers: { mid: [{ part: "hills", x: 50, y: 70 }], subject: [{ part: "crate", x: 50, y: 80 }] },
    };
    const ghostly: SceneRecipe = { ...plain, crowd: [{ ...extra, fade: 0.5 }], layers: { ...plain.layers, mid: [{ part: "hills", x: 50, y: 70, fade: 0.5 }] } };
    const drawn = composeScene(plain, "crowd-proof");
    const faded = composeScene(ghostly, "crowd-proof");
    const hills = drawn.filter((shape) => shape.part === "hills").length;
    const crowd = drawn.filter((shape) => shape.part === "pose-stand" || lookParts(FIGURE_DEFAULT_LOOK).includes(shape.part)).length;
    expect(hills).toBeGreaterThan(0);
    expect(crowd).toBeGreaterThan(0);
    // The hills, then the crowd figure, both in the middle layer, then the subject layer's crate.
    expect(drawn.slice(0, hills).every((shape) => shape.part === "hills" && shape.layer === "mid")).toBe(true);
    expect(drawn.slice(hills, hills + crowd).every((shape) => shape.part !== "hills" && shape.part !== "crate" && shape.layer === "mid")).toBe(true);
    expect(drawn.slice(hills + crowd).every((shape) => shape.part === "crate" && shape.layer === "subject")).toBe(true);
    // A fade keeps every shape and halves its opacity.
    expect(faded.map((shape) => shape.d)).toEqual(drawn.map((shape) => shape.d));
    faded.forEach((shape, k) => {
      const was = drawn[k];
      if (was === undefined) throw new Error(`no shape ${k}`);
      expect(shape.opacity).toBeCloseTo(shape.part === "crate" ? was.opacity : was.opacity / 2);
    });
    // A crowd figure in the subject layer stands before the recipe's own figure.
    const both = composeScene({ ...plain, crowd: [standing], figure: { ...standing, x: 60, pose: "pose-guard" } }, "crowd-proof");
    const first = both.findIndex((shape) => shape.part === "pose-stand");
    const figure = both.findIndex((shape) => shape.part === "pose-guard");
    expect(both.slice(0, first).some((shape) => shape.part === "crate")).toBe(true);
    expect(first).toBeLessThan(figure);
    // The proofs, which use neither, draw what they drew before this pass.
    for (const [id, [base, radiant]] of Object.entries(PROOF_PINS)) {
      const card = CATALOG[id];
      if (card === undefined) throw new Error(`the catalog has no ${id}`);
      expect([hashId(proceduralCardArtUri(card, false)), hashId(proceduralCardArtUri(card, true))], id).toEqual([base, radiant]);
    }
  });
});

describe("R3393001 a Legacy card draws a scene of its own", () => {
  it("R3393001 each Legacy card places its own yard touch", () => {
    const touches = Object.values(LEGACY_TOUCHES);
    expect(Object.keys(LEGACY_TOUCH_PARTS).sort()).toEqual([...touches].sort());
    const parts = Object.values(LEGACY_TOUCH_PARTS);
    expect(new Set(parts).size).toBe(20);
    for (const part of parts) expect(Object.hasOwn(SCENE_KIT, part), part).toBe(true);
    for (const card of LEGACY) {
      const touch: LegacyTouch | undefined = LEGACY_TOUCHES[card.id];
      if (touch === undefined) throw new Error(`no touch for ${card.id}`);
      const own = placements(recipe(card.id)).filter((placed) => placed.part === LEGACY_TOUCH_PARTS[touch]);
      expect(own.map((placed) => placed.count), `${card.id} ${touch}`).toEqual([1]);
    }
  });

  it("R3393001 a Legacy face draws its own scene wearing its Core card's frozen emblem, never legacySvg's or its Core card's scene", () => {
    for (const card of LEGACY) {
      const { coreId, name } = coreOf(card);
      const theme = themeFor(card.tags, card.type);
      const shapes = composeScene(recipe(card.id), card.id);
      const coreScene = sceneRecipeOf(coreId);
      for (const radiant of [false, true]) {
        const uri = legacyArtUri(card.id, coreId, name, card.tags, card.type, radiant);
        const svg = svgOf(uri);
        for (const shape of shapes) expect(svg.includes(`d="${shape.d}"`), `${card.id} ${shape.part}`).toBe(true);
        const core = artSpec(coreId, theme, compositionFor(card.type), radiant, motifFor(name, theme), null);
        expect(svg, card.id).toContain(`<path d="${EMBLEMS[core.emblem.glyph].d}" fill="${core.emblem.fill}" stroke="${core.emblem.stroke}"`);
        expect(svg, card.id).not.toBe(legacySvg(card.id, coreId, name, card.tags, card.type, radiant));
        if (coreScene !== null) for (const shape of composeScene(coreScene, coreId)) expect(svg.includes(`d="${shape.d}"`), `${card.id} ${coreId}`).toBe(false);
        expect(uri, card.id).toBe(proceduralCardArtUri(card, radiant));
      }
    }
    // Speciesist's Legacy card is the one whose Core card draws a scene of its own (R3331000).
    expect(LEGACY.filter((card) => sceneRecipeOf(coreOf(card).coreId) !== null).map((card) => card.id)).toEqual(["traininggrounds-041"]);
  });
});
