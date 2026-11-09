// R3404000: every Meditative card offered Alt art draws an alt scene recipe of its own (meditativeAltRecipes1.ts to
// meditativeAltRecipes3.ts), its subject restaged in another place, time or moment from a brief per card, read under
// its Alt art seed alone, at every Alt art level (R3015002), base and Radiant. Here: which cards hold one and which
// seeds read it; that each keeps its card's subject and moves its sky and its composition off the normal picture's;
// its detail floor, byte ceiling, geometry and gold (R3331001, R2725004); its one red seal (R3401000), its emblem and
// props (R2715001); the three parts the briefs added; and every Alt art level's face. The pixels each Alt picture
// changes (`ALT_PICTURE_CHANGE_MIN`) are e2e/cypress/component/meditative-alt-scenes.cy.tsx's, which jsdom cannot paint.

import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef, Rarity } from "@jackioh/shared";

import { __resetSettingsForTests } from "../../settings/store.ts";
import { facePicture, wholePicture } from "../../test/artPicture.ts";
import { legacyText } from "../../test/faceData.ts";
import { CardFace } from "../CardFace.tsx";
import {
  RADIANT_GOLD_HUE_MAX_DEG,
  RADIANT_GOLD_HUE_MIN_DEG,
  RADIANT_GOLD_LIGHTNESS_MAX,
  RADIANT_GOLD_LIGHTNESS_MIN,
  RADIANT_GOLD_SATURATION_MIN,
} from "../constants.ts";
import { resetCosmeticsForTests, writeCardLevel } from "../cosmetics.ts";
import { faceModel } from "../model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../settings.ts";
import { artPictureOf, isFullBleed, ladderOf, levelsOn, offeredOn, VERSION_ALT_ART_ID } from "../versions.ts";
import { meditativeLookOf } from "./meditativeAltRecipes1.ts";
import { motifFor } from "./motifs.ts";
import { ART_ROLES, FULL_ART_CARD_MOTION, FULL_ART_MOTION } from "./motion.ts";
import { artSeedOf } from "./pictures.ts";
import { artSpec, type ArtSpec } from "./procedural.ts";
import { ART_DETAIL_MAX_BYTES, ART_DETAIL_MIN, artElements } from "./richness.ts";
import { composeScene, SCENE_LAYERS, sceneRecipeOf, sceneUse, type SceneLayer, type SceneRecipe } from "./scene.ts";
import { SCENE_KIT, type SceneryPartId } from "./sceneKit.ts";
import { INK_MACHINES_KIT } from "./sceneKitInkMachines.ts";
import { INK_OBJECTS_KIT } from "./sceneKitInkObjects.ts";
import { ALT_PICTURE_CHANGE_MIN, ALT_SCENE_RECIPES, SCENE_RECIPES } from "./sceneRecipes.ts";
import { proceduralArtUri } from "./svg.ts";
import { compositionOf, themeFor } from "./themes.ts";

const SVG_PREFIX = "data:image/svg+xml,";
/** The Meditative cards with an alt recipe, in id order. */
const ALT_IDS: readonly string[] = Object.keys(ALT_SCENE_RECIPES)
  .filter((id) => id.startsWith("meditative-"))
  .sort();
/** The rarer than Rare: a scene with a foreground, a middle and a background. */
const DEEP: readonly Rarity[] = ["Epic", "Legendary", "Mythic"];
/** R3017002's repaints, each a picture of its own under its treatment, which keep the card's normal recipe. */
const REPAINTS = ["night", "watercolor", "stained-glass", "tarot"] as const;
/** How far, in the 100-unit box, an Alt picture's subject anchor moves from the normal one's when its composition changes by place. */
const ALT_SUBJECT_SHIFT_MIN = 8;
/** Or how far its subject's scale changes, as a ratio either way, when it changes by camera. */
const ALT_SUBJECT_SCALE_RATIO_MIN = 1.25;
/** Where an emblem or the seal may stand, and how big an emblem may be: inside the band every face's window shows (meditativeScenes.test.ts's). */
const EMBLEM_BAND = { left: 10, right: 90, top: 22, bottom: 84, size: 8 } as const;
/**
 * The parts each card's subject is when no figure is (or only one picture draws one), each drawn somewhere in its
 * normal picture and in its Alt picture's subject layer, the first of them placed first: the brief's same subject,
 * restaged.
 */
const KEPT_SUBJECT: Readonly<Record<string, readonly SceneryPartId[]>> = {
  "meditative-001": ["table", "ink-card-fan", "red-thread"],
  "meditative-004": ["melon"],
  "meditative-010": ["abacus"],
  "meditative-013": ["pea-vine"],
  "meditative-024": ["ink-sheep", "helmet"],
  "meditative-032": ["lion-dancer", "ink-card-fan"],
  "meditative-037": ["bottle"],
  "meditative-038": ["printer", "ink-person"],
  "meditative-040": ["rune-circle"],
  "meditative-043": ["jade-boulder", "awning"],
  "meditative-044": ["stone-well", "jade-boulder"],
  "meditative-047": ["taotie"],
  "meditative-050": ["card-ring", "playing-card"],
  "meditative-051": ["round-bot"],
  "meditative-055": ["dragon-fruit"],
  "meditative-056": ["tiled-house"],
  "meditative-060": ["parcel", "envelope-drift"],
  "meditative-063": ["horned-skull"],
  "meditative-071": ["cane", "lectern", "desk"],
  "meditative-074": ["rock-giant"],
  "meditative-075": ["great-tree", "tiled-house"],
  "meditative-078": ["gust"],
  "meditative-080": ["lantern", "closed-tome"],
  "meditative-081": ["wide-table", "deck"],
  "meditative-087": ["totem"],
  "meditative-090": ["basket"],
  "meditative-095": ["jade-die"],
  "meditative-097": ["blueprint"],
};
/** The cards whose full-art motion moves props (R2715001), so a placement outside the subject layer moves on that beat. */
const PROPS_CARDS: readonly string[] = ["meditative-021", "meditative-087"];
/** The parts the briefs added to the kit (R3404000), each with a card whose Alt picture draws it. */
const ADDED_PARTS: readonly (readonly [SceneryPartId, string])[] = [
  ["oar", "meditative-004"],
  ["sofa", "meditative-040"],
  ["tower-crane", "meditative-097"],
];

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`no ${id}`);
  return found;
}

function alt(id: string): SceneRecipe {
  const found = ALT_SCENE_RECIPES[id];
  if (found === undefined) throw new Error(`no alt recipe for ${id}`);
  return found;
}

function normal(id: string): SceneRecipe {
  const found = SCENE_RECIPES[id];
  if (found === undefined) throw new Error(`no recipe for ${id}`);
  return found;
}

function svgOf(uri: string): string {
  return decodeURIComponent(uri.slice(SVG_PREFIX.length));
}

/** The picture `seedId` draws on `card`'s face, as its window's painter draws it (CardArt). */
function uriOf(card: CardDef, seedId: string, radiant: boolean, motion: (typeof FULL_ART_MOTION)[string] | null = null): string {
  const theme = themeFor(card.tags, card.type);
  return proceduralArtUri(seedId, theme, compositionOf(card.id, card.type), radiant, motifFor(card.name, theme), motion);
}

/** `card`'s Alt art spec as its painter draws it, its recipe read from its Alt art seed. */
function altSpecOf(card: CardDef, radiant: boolean): ArtSpec {
  const theme = themeFor(card.tags, card.type);
  return artSpec(artSeedOf(card.id, "alt"), theme, compositionOf(card.id, card.type), radiant, motifFor(card.name, theme));
}

/** A hex colour's hue in degrees and saturation and lightness from 0 to 1 (classicPlusScenes.test.ts's). */
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

/** Where a picture's subject stands and how big: its figure, else its subject layer's first placement. */
function anchorOf(scene: SceneRecipe): { x: number; y: number; scale: number } | null {
  if (scene.figure !== undefined) return { x: scene.figure.x, y: scene.figure.y, scale: scene.figure.scale };
  const first = scene.layers.subject?.[0];
  return first === undefined ? null : { x: first.x, y: first.y, scale: first.scale ?? 1 };
}

/** Every part a recipe places, in any layer. */
function partsOf(scene: SceneRecipe): Set<string> {
  return new Set(SCENE_LAYERS.flatMap((layer) => (scene.layers[layer] ?? []).map((placed) => placed.part)));
}

function faceIn(id: string, radiant: boolean): HTMLElement {
  const { container } = render(<CardFace face={faceModel({ defId: id, def: def(id), radiant })} layout="full" worn />);
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error(`no face for ${id}`);
  return cf;
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

describe("R3404000 Meditative Alt art restaged", () => {
  it("R3404000 every Meditative card offered Alt art, and no other, has an alt recipe, read by its Alt art seed alone", () => {
    const offeredIn = (set: string): string[] =>
      Object.values(CATALOG)
        .filter((card) => card.set === set && offeredOn(VERSION_ALT_ART_ID, card.id))
        .map((card) => card.id);
    const offered = offeredIn("Meditative").sort();
    expect(offered).toHaveLength(54);
    expect(ALT_IDS).toEqual(offered);
    // With Classic+'s (R3396000), Meditative+'s and Training Grounds' (R3397000) and Core's (R3398000), the table holds
    // those five sets' cards offered Alt art and no other.
    const others = ["Classic+", "Meditative+", "Training Grounds", "Core"].flatMap(offeredIn);
    expect(Object.keys(ALT_SCENE_RECIPES).sort()).toEqual([...others, ...offered].sort());
    expect(Object.keys(ALT_SCENE_RECIPES)).toHaveLength(191);
    expect(ALT_PICTURE_CHANGE_MIN).toBe(0.6);
    const seen = new Map<string, string>(Object.entries(SCENE_RECIPES).map(([id, scene]) => [JSON.stringify(scene), id]));
    for (const id of ALT_IDS) {
      const scene = alt(id);
      expect(sceneRecipeOf(artSeedOf(id, "alt")), id).toBe(scene);
      // The normal picture and every repaint keep the card's recipe.
      expect(sceneRecipeOf(id), id).toBe(normal(id));
      for (const picture of REPAINTS) expect(sceneRecipeOf(artSeedOf(id, picture)), `${id} ${picture}`).toBe(normal(id));
      expect(sceneRecipeOf(`${id}#altx`), id).toBe(normal(id));
      // Every Alt art level draws the Alt picture (R3015002).
      expect(levelsOn(id, "alt-art"), id).toEqual(ladderOf("alt-art"));
      for (const level of ladderOf("alt-art")) expect(artPictureOf(level, def(id).set), `${id} ${level}`).toBe("alt");
      expect(scene.family, id).toBe("meditative");
      // No recipe, normal or alt, is another's.
      const key = JSON.stringify(scene);
      expect(seen.get(key), `${id} draws ${seen.get(key) ?? ""}'s recipe`).toBeUndefined();
      seen.set(key, `${id}#alt`);
      // Meditative's brush line outlines the scene's solid shapes.
      expect(composeScene(scene, artSeedOf(id, "alt")).some((shape) => shape.inked === true), id).toBe(true);
    }
  });

  it("R3404000 each Alt picture keeps its card's subject", () => {
    for (const id of ALT_IDS) {
      const scene = alt(id);
      const was = normal(id);
      const subject = new Set((scene.layers.subject ?? []).map((placed) => placed.part));
      // Every subject layer holds a part, or the scene a figure.
      expect(subject.size > 0 || scene.figure !== undefined, id).toBe(true);
      // A Unit drawn as a person stays that person.
      if (def(id).type === "Unit" && was.figure !== undefined) expect(scene.figure, id).toBeDefined();
      // A figure both pictures draw is the same person: the same head, outfit and skin, the normal look's.
      if (was.figure !== undefined && scene.figure !== undefined) {
        expect(meditativeLookOf(id), id).toEqual(was.figure.look);
        const { head, outfit, skin } = scene.figure.look;
        expect([head, outfit, skin], id).toEqual([was.figure.look.head, was.figure.look.outfit, was.figure.look.skin]);
      }
      // A subject that is a part stays that part, the first of them placed first.
      const kept = KEPT_SUBJECT[id] ?? [];
      for (const part of kept) {
        expect(partsOf(was).has(part), `${id}: ${part} in the normal picture`).toBe(true);
        expect(subject.has(part), `${id}: ${part} in the Alt picture's subject layer`).toBe(true);
      }
      if (kept.length > 0) expect(scene.layers.subject?.[0]?.part, id).toBe(kept[0]);
      // A card with neither a figure in both nor a kept part would keep nothing.
      expect((was.figure !== undefined && scene.figure !== undefined) || (KEPT_SUBJECT[id]?.length ?? 0) > 0, `${id} keeps its subject`).toBe(true);
    }
  });

  it("R3404000 each Alt picture's sky or composition differs from its normal one", () => {
    for (const id of ALT_IDS) {
      const scene = alt(id);
      const was = normal(id);
      // Every brief names a new palette, so the skies share no colour.
      expect(scene.sky.filter((name) => was.sky.includes(name)), id).toEqual([]);
      // And a new camera or a new place: the subject moves, or grows or shrinks, or a figure comes or goes.
      const a = anchorOf(was);
      const b = anchorOf(scene);
      const moved = a !== null && b !== null && Math.hypot(a.x - b.x, a.y - b.y) >= ALT_SUBJECT_SHIFT_MIN;
      const rescaled = a !== null && b !== null && Math.max(a.scale / b.scale, b.scale / a.scale) >= ALT_SUBJECT_SCALE_RATIO_MIN;
      const figured = (was.figure === undefined) !== (scene.figure === undefined);
      expect(moved || rescaled || figured, `${id}: ${JSON.stringify(a)} -> ${JSON.stringify(b)}`).toBe(true);
    }
  });

  it("R3404000 each Alt picture clears its rarity's floor and the byte ceiling, base and Radiant, and prints no text", () => {
    for (const id of ALT_IDS) {
      const card = def(id);
      const seed = artSeedOf(id, "alt");
      const floor = ART_DETAIL_MIN[card.rarity];
      const shapes = composeScene(alt(id), seed);
      expect(artElements(svgOf(uriOf(card, seed, false))), id).toBeGreaterThanOrEqual(floor.elements);
      expect(sceneUse(shapes).layers, id).toBeGreaterThanOrEqual(floor.layers);
      const layers = new Set<SceneLayer>(shapes.map((shape) => shape.layer));
      expect(layers, id).toContain("subject");
      if (DEEP.includes(card.rarity)) for (const layer of ["far", "mid", "subject", "near"] as const) expect(layers, `${id} ${layer}`).toContain(layer);
      for (const radiant of [false, true]) {
        const uri = uriOf(card, seed, radiant);
        const where = `${id} ${radiant ? "radiant" : "base"}`;
        expect(uri.length, where).toBeLessThanOrEqual(ART_DETAIL_MAX_BYTES);
        for (const banned of ["href", "<text", "<title", "NaN", "undefined"]) expect(svgOf(uri), `${where} ${banned}`).not.toContain(banned);
      }
    }
  });

  it("R3404000 base and Radiant draw one geometry, the Radiant face gilded, and no base face paints in Radiant's gold window", () => {
    for (const id of ALT_IDS) {
      const card = def(id);
      const shapes = composeScene(alt(id), artSeedOf(id, "alt"));
      const base = altSpecOf(card, false);
      const radiant = altSpecOf(card, true);
      expect(base.ridges.map((ridge) => ridge.d), id).toEqual(shapes.map((shape) => shape.d));
      expect(radiant.ridges.map((ridge) => ridge.d), id).toEqual(base.ridges.map((ridge) => ridge.d));
      base.ridges.forEach((ridge, k) => expect(radiant.ridges[k]?.fill, `${id} #${k}`).not.toBe(ridge.fill));
      expect(radiant.sky, id).not.toEqual(base.sky);
      expect(base.rays, id).toHaveLength(0);
      expect(radiant.rays.length, id).toBeGreaterThan(0);
      const colors = [base.sky.from, base.sky.to, base.glow.color, ...base.ridges.flatMap((ridge) => [ridge.fill, ...(ridge.stroke === undefined ? [] : [ridge.stroke])])];
      for (const hex of colors) expect(inRadiantGold(hex), `${id}: ${hex}`).toBe(false);
    }
  });

  it("R3404000 every Alt picture is its own", () => {
    const pictures = new Map<string, string>();
    for (const id of ALT_IDS) {
      for (const radiant of [false, true]) {
        for (const seed of [id, artSeedOf(id, "alt")]) {
          const uri = uriOf(def(id), seed, radiant);
          const where = `${seed} ${radiant ? "radiant" : "base"}`;
          expect(pictures.get(uri), `${where} draws ${pictures.get(uri) ?? ""}'s picture`).toBeUndefined();
          pictures.set(uri, where);
        }
      }
    }
    expect(pictures.size).toBe(ALT_IDS.length * 4);
  });

  it("R3404000 the emblem stands in the band, props stay out of the subject layer and the full-art motions move every layer they name", () => {
    for (const id of ALT_IDS) {
      const scene = alt(id);
      const { emblem } = scene;
      expect(emblem, id).toBeDefined();
      if (emblem === undefined) continue;
      expect(emblem.size, id).toBeLessThanOrEqual(EMBLEM_BAND.size);
      expect(emblem.x, id).toBeGreaterThanOrEqual(EMBLEM_BAND.left);
      expect(emblem.x, id).toBeLessThanOrEqual(EMBLEM_BAND.right);
      expect(emblem.y, id).toBeGreaterThanOrEqual(EMBLEM_BAND.top);
      expect(emblem.y, id).toBeLessThanOrEqual(EMBLEM_BAND.bottom);
      const spec = altSpecOf(def(id), false);
      expect([spec.emblem.x, spec.emblem.y, spec.emblem.size], id).toEqual([emblem.x, emblem.y, emblem.size]);
      for (const layer of SCENE_LAYERS) for (const placed of scene.layers[layer] ?? []) if (placed.props === true) expect(layer, `${id} ${placed.part}`).not.toBe("subject");
    }
    for (const id of PROPS_CARDS) {
      const moving = SCENE_LAYERS.filter((layer) => layer !== "subject").some((layer) => (alt(id).layers[layer] ?? []).some((placed) => placed.props === true));
      expect(moving, id).toBe(true);
    }
    // Each Meditative full-art card offered Alt art moves its Alt picture's every layer its motion names.
    // The full-art cards' own motions (R2715001), not R3340000's recipes for every other Legendary and Mythic.
    const fullArt = ALT_IDS.filter((id) => FULL_ART_CARD_MOTION[id] !== undefined);
    expect(fullArt).toEqual(["meditative-021", "meditative-087"]);
    for (const id of fullArt) {
      const motion = FULL_ART_MOTION[id] ?? null;
      const svg = svgOf(uriOf(def(id), artSeedOf(id, "alt"), false, motion));
      for (const role of ART_ROLES) if (motion?.[role] !== undefined) expect(svg, `${id}: ${role}`).toContain(`class="art-m-${role}"`);
      if (motion?.props !== undefined) expect(altSpecOf(def(id), false).ridges.some((ridge) => ridge.role === "props"), id).toBe(true);
    }
  });

  it("R3404000 each Alt picture stamps one red seal, near and inside the shown band", () => {
    for (const id of ALT_IDS) {
      const scene = alt(id);
      const seals = SCENE_LAYERS.flatMap((layer) => (scene.layers[layer] ?? []).filter((placed) => placed.part === "ink-seal").map((placed) => ({ layer, placed })));
      expect(seals, id).toHaveLength(1);
      const [seal] = seals;
      if (seal === undefined) continue;
      expect(seal.layer, id).toBe("near");
      expect(seal.placed.count ?? 1, id).toBe(1);
      expect(seal.placed.paint, id).toBeUndefined();
      expect(seal.placed.props, id).toBeUndefined();
      expect(seal.placed.x, id).toBeGreaterThanOrEqual(EMBLEM_BAND.left);
      expect(seal.placed.x, id).toBeLessThanOrEqual(EMBLEM_BAND.right);
      expect(seal.placed.y, id).toBeGreaterThanOrEqual(EMBLEM_BAND.top);
      expect(seal.placed.y, id).toBeLessThanOrEqual(EMBLEM_BAND.bottom);
      const shapes = composeScene(scene, artSeedOf(id, "alt")).filter((shape) => shape.part === "ink-seal");
      expect(shapes.length, id).toBeGreaterThan(0);
      expect(shapes.some((shape) => shape.paint === "red-600"), id).toBe(true);
    }
  });

  it("R3404000 the parts the briefs added draw in their pictures", () => {
    const added: Readonly<Record<string, unknown>> = { ...INK_OBJECTS_KIT, ...INK_MACHINES_KIT };
    for (const [part, id] of ADDED_PARTS) {
      expect(Object.hasOwn(added, part), part).toBe(true);
      expect(SCENE_KIT[part], part).toBe(added[part]);
      expect(partsOf(alt(id)).has(part), `${id} draws ${part}`).toBe(true);
      expect(composeScene(alt(id), artSeedOf(id, "alt")).some((shape) => shape.part === part), `${id} draws ${part}`).toBe(true);
    }
  });

  for (const id of ALT_IDS) {
    it(`R3404000 ${id} draws its alt recipe at every Alt art level, base and Radiant, keeping its Radiant marks`, () => {
      const card = def(id);
      const seed = artSeedOf(id, "alt");
      const shapes = composeScene(alt(id), seed);
      const marked = legacyText(faceModel({ defId: id, def: card, radiant: true }).text).marks.length > 0;
      const [first] = ladderOf("alt-art");
      for (const level of ladderOf("alt-art")) {
        writeCardLevel(id, level);
        for (const radiant of [false, true]) {
          const cf = faceIn(id, radiant);
          const where = `${id} ${radiant ? "Radiant" : "base"} ${level}`;
          expect(cf.getAttribute("data-level"), where).toBe(level);
          expect(cf.querySelector(".cf-art")?.getAttribute("data-art-picture"), where).toBe("alt");
          const picture = facePicture(cf);
          for (const shape of shapes) expect(picture.includes(`d="${shape.d}"`), `${where} ${shape.part}`).toBe(true);
          if (level === first) expect(picture, where).toBe(wholePicture(decodeURIComponent(uriOf(card, seed, radiant))));
          // R2725004: the gold-foil frame and the art's Radiant paint on a Radiant face alone, and R277's marks wherever its words show.
          expect(cf.getAttribute("data-radiant-face"), where).toBe(radiant ? "true" : null);
          expect(cf.querySelector(".cf-art")?.getAttribute("data-art-variant"), where).toBe(radiant ? "radiant" : "base");
          const marks = cf.querySelectorAll('.cf-mark[data-mark="radiant"]').length;
          if (!radiant || isFullBleed(level)) expect(marks, where).toBe(0);
          else expect(marks > 0, where).toBe(marked);
          cleanup();
        }
      }
    });
  }
});
