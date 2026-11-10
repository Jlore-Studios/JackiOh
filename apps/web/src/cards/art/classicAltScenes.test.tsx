// R3388000: every Classic card offered Alt art draws an alt scene recipe of its own (classicAltRecipes1.ts to
// classicAltRecipes3.ts), its subject restaged in another place, time or moment from a brief per card (#3388's table),
// read under its Alt art seed alone, at every Alt art level (R3015002), base and Radiant. Here: which cards hold one and
// which seeds read it; that each keeps its card's subject and moves its sky and its composition off the normal
// picture's; its detail floor, byte ceiling, geometry and gold (R3331001, R2725004); its emblem and props (R3352000,
// R2715001); the two parts the pass added to the kit, a frog and a shark; and every Alt art level's face. The pixels
// each Alt picture changes (`ALT_PICTURE_CHANGE_MIN`) are e2e/cypress/component/classic-alt-scenes.cy.tsx's, which
// jsdom cannot paint.

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
import { normalClassicExtraLookOf, normalClassicLookOf } from "./classicAltRecipes1.ts";
import { FIGURE_KIT } from "./figureKit.ts";
import { motifFor } from "./motifs.ts";
import { ART_ROLES, FULL_ART_CARD_MOTION, FULL_ART_MOTION } from "./motion.ts";
import { artSeedOf } from "./pictures.ts";
import { artSpec, type ArtSpec } from "./procedural.ts";
import { ART_DETAIL_MAX_BYTES, ART_DETAIL_MIN, artElements } from "./richness.ts";
import { composeScene, SCENE_LAYERS, sceneRecipeOf, sceneUse, type SceneLayer, type SceneRecipe } from "./scene.ts";
import { SCENE_KIT, type SceneryPartId } from "./sceneKit.ts";
import { CLASSIC_ALT_KIT } from "./sceneKitClassicAlt.ts";
import { STINKY_ALT_KIT } from "./sceneKitStinkyAlt.ts";
import { ALT_PICTURE_CHANGE_MIN, ALT_SCENE_RECIPES, SCENE_RECIPES } from "./sceneRecipes.ts";
import { proceduralArtUri } from "./svg.ts";
import { compositionOf, themeFor } from "./themes.ts";

const SVG_PREFIX = "data:image/svg+xml,";
/** The Classic cards with an alt recipe, in id order. */
const ALT_IDS: readonly string[] = Object.keys(ALT_SCENE_RECIPES)
  .filter((id) => CATALOG[id]?.set === "Classic")
  .sort();
/** The rarer than Rare: a scene with a foreground, a middle and a background. */
const DEEP: readonly Rarity[] = ["Epic", "Legendary", "Mythic"];
/** R3017002's repaints, each a picture of its own under its treatment, which keep the card's normal recipe. */
const REPAINTS = ["night", "watercolor", "stained-glass", "tarot"] as const;
/** How far, in the 100-unit box, an Alt picture's subject anchor moves from the normal one's when its composition changes by place. */
const ALT_SUBJECT_SHIFT_MIN = 8;
/** Or how far its subject's scale changes, as a ratio either way, when it changes by camera. */
const ALT_SUBJECT_SCALE_RATIO_MIN = 1.25;
/** Where an emblem may stand and how big it may be: inside the band every face's window shows (classicPlusAltScenes.test.tsx's). */
const EMBLEM_BAND = { left: 10, right: 90, top: 22, bottom: 84, size: 8 } as const;
/**
 * The parts each card's subject is when no figure is (or only the normal picture draws one), each drawn somewhere in its
 * normal picture and in its Alt picture's subject layer: the brief's same subject, restaged.
 */
const KEPT_SUBJECT: Readonly<Record<string, readonly SceneryPartId[]>> = {
  "classic-004": ["orb"],
  "classic-007": ["sceptre"],
  "classic-008": ["pickle"],
  "classic-010": ["card-imp"],
  "classic-012": ["tome"],
  "classic-014": ["ghost-figure"],
  "classic-019": ["lizard"],
  "classic-020": ["scales"],
  "classic-021": ["tortoise"],
  "classic-025": ["goblin"],
  "classic-027": ["slime"],
  "classic-034": ["card"],
  "classic-035": ["rune-circle", "red-thread"],
  "classic-042": ["vat"],
  "classic-043": ["spores"],
  "classic-048": ["shrimp"],
  "classic-052": ["die"],
  "classic-055": ["tome"],
  "classic-060": ["banner"],
  "classic-062": ["orb"],
  "classic-064": ["forge", "glow-chute"],
  "classic-070": ["tome", "moth"],
  "classic-074": ["bone-scarecrow"],
  "classic-075": ["eye"],
  "classic-076": ["mule"],
  "classic-081": ["fruit-tree", "heal-fountain", "lifting-stone"],
  "classic-083": ["fireball"],
  "classic-084": ["portcullis"],
  "classic-090": ["spiral-shaft"],
};
/** The crowds a brief keeps, each figure of it the normal picture's own person: the kneeling warrior, the digger turned diver, the resting hero, the rival farmer and the two warriors of the heap. */
const KEPT_CROWDS: readonly string[] = ["classic-032", "classic-034", "classic-045", "classic-058", "classic-060"];
/** The cards whose full-art motion moves props (R2715001), so a placement outside the subject layer moves on that beat. */
const PROPS_CARDS: readonly string[] = ["classic-090"];
/** R3388000: the parts the pass added to the kit, each with the cards whose Alt pictures draw it. */
const NEW_PARTS: Readonly<Record<keyof typeof CLASSIC_ALT_KIT, readonly string[]>> = {
  frog: ["classic-042"],
  shark: ["classic-021"],
};

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

describe("R3388000 Classic Alt art restaged", () => {
  it("R3388000 every Classic card offered Alt art, and no other, has an alt recipe, read by its Alt art seed alone", () => {
    const offeredIn = (set: string): string[] =>
      Object.values(CATALOG)
        .filter((card) => card.set === set && offeredOn(VERSION_ALT_ART_ID, card.id))
        .map((card) => card.id);
    const offered = offeredIn("Classic").sort();
    expect(offered).toHaveLength(50);
    expect(ALT_IDS).toEqual(offered);
    // With R3396000's Classic+ cards, R3397000's Meditative+ and Training Grounds cards, R3404000's Meditative cards and
    // R3398000's Core cards, the alt recipes are every card offered Alt art in the six sets, and no other; R3406000: and
    // Stinkyscape's later cards' (stinkyscape91AltScenes.test.tsx).
    const sets = ["Classic+", "Meditative+", "Training Grounds", "Meditative", "Core"];
    const others = sets.flatMap(offeredIn);
    const ofTheSix = Object.keys(ALT_SCENE_RECIPES).filter((id) => [...sets, "Classic"].includes(CATALOG[id]?.set ?? ""));
    expect(ofTheSix.sort()).toEqual([...others, ...offered].sort());
    expect(ofTheSix).toHaveLength(241);
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
      expect(scene.family, id).toBe("classic");
      // An alt picture draws its subject from the kits.
      expect(scene.ownSubject, id).toBeUndefined();
      // No recipe, normal or alt, is another's.
      const key = JSON.stringify(scene);
      expect(seen.get(key), `${id} draws ${seen.get(key) ?? ""}'s recipe`).toBeUndefined();
      seen.set(key, `${id}#alt`);
    }
  });

  it("R3388000 each Alt picture keeps its card's subject", () => {
    for (const id of ALT_IDS) {
      const scene = alt(id);
      const was = normal(id);
      const subject = new Set((scene.layers.subject ?? []).map((placed) => placed.part));
      // Every subject layer holds a part, or the scene a figure.
      expect(subject.size > 0 || scene.figure !== undefined, id).toBe(true);
      // A Unit drawn as a person stays that person.
      if (def(id).type === "Unit" && was.figure !== undefined) expect(scene.figure, id).toBeDefined();
      // A figure both pictures draw is the same person: the same head and outfit, the normal look's.
      if (was.figure !== undefined && scene.figure !== undefined) {
        expect(normalClassicLookOf(id), id).toEqual(was.figure.look);
        expect([scene.figure.look.head, scene.figure.look.outfit], id).toEqual([was.figure.look.head, was.figure.look.outfit]);
      }
      // A subject that is a part stays that part.
      for (const part of KEPT_SUBJECT[id] ?? []) {
        expect(partsOf(was).has(part), `${id}: ${part} in the normal picture`).toBe(true);
        expect(subject.has(part), `${id}: ${part} in the Alt picture's subject layer`).toBe(true);
      }
      // A card with neither a figure in both nor a kept part would keep nothing.
      expect((was.figure !== undefined && scene.figure !== undefined) || (KEPT_SUBJECT[id]?.length ?? 0) > 0, `${id} keeps its subject`).toBe(true);
    }
    // A kept crowd is the normal picture's people.
    for (const id of KEPT_CROWDS) {
      const was = normal(id).extras ?? [];
      const now = alt(id).extras ?? [];
      expect(now.length, id).toBe(was.length);
      was.forEach((extra, k) => {
        expect(normalClassicExtraLookOf(id, k), `${id} #${k}`).toEqual(extra.look);
        expect([now[k]?.look.head, now[k]?.look.outfit], `${id} #${k}`).toEqual([extra.look.head, extra.look.outfit]);
      });
    }
  });

  it("R3388000 each Alt picture's sky and composition differ from its normal one", () => {
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

  it("R3388000 each Alt picture clears its rarity's floor and the byte ceiling, base and Radiant, and prints no text", () => {
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

  it("R3388000 base and Radiant draw one geometry, the Radiant face gilded, and no base face paints in Radiant's gold window", () => {
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

  it("R3388000 every Alt picture is its own", () => {
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

  it("R3388000 the emblem stands in the band, props stay out of the subject layer and the full-art motions move every layer they name", () => {
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
    // Each Classic full-art card offered Alt art moves its Alt picture's every layer its motion names.
    // The full-art cards' own motions (R2715001), not R3340000's recipes for every other Legendary and Mythic.
    const fullArt = ALT_IDS.filter((id) => FULL_ART_CARD_MOTION[id] !== undefined);
    expect(fullArt).toEqual(PROPS_CARDS);
    for (const id of fullArt) {
      const motion = FULL_ART_MOTION[id] ?? null;
      const svg = svgOf(uriOf(def(id), artSeedOf(id, "alt"), false, motion));
      for (const role of ART_ROLES) if (motion?.[role] !== undefined) expect(svg, `${id}: ${role}`).toContain(`class="art-m-${role}"`);
      if (motion?.props !== undefined) expect(altSpecOf(def(id), false).ridges.some((ridge) => ridge.role === "props"), id).toBe(true);
    }
  });

  it("R3388000 the frog and the shark join the kit just before the Stinkyscape Alt art pass's parts, and only their scenes draw them", () => {
    const added = Object.keys(CLASSIC_ALT_KIT);
    expect(Object.keys(NEW_PARTS).sort()).toEqual([...added].sort());
    // They follow Classic's own parts and come before R3406000's, which stay last (stinkyscape91AltScenes.test.tsx),
    // each as itself, so none takes the place of an earlier part (an id spread twice keeps its first place).
    const stinky = Object.keys(STINKY_ALT_KIT).length;
    expect(Object.keys(SCENE_KIT).slice(-(stinky + added.length), -stinky)).toEqual(added);
    for (const [id, part] of Object.entries(CLASSIC_ALT_KIT)) {
      expect(SCENE_KIT[id as keyof typeof SCENE_KIT], id).toBe(part);
      expect(Object.hasOwn(FIGURE_KIT, id), `${id} is a figure part`).toBe(false);
    }
    for (const [part, ids] of Object.entries(NEW_PARTS)) {
      // None is a part of a normal picture or of another set's Alt picture, and each brief that names it places and draws it.
      for (const [id, scene] of Object.entries(SCENE_RECIPES)) expect(partsOf(scene).has(part), `${id} ${part}`).toBe(false);
      for (const [id, scene] of Object.entries(ALT_SCENE_RECIPES)) if (!ALT_IDS.includes(id)) expect(partsOf(scene).has(part), `${id}#alt ${part}`).toBe(false);
      expect(ALT_IDS.filter((id) => partsOf(alt(id)).has(part)), part).toEqual(ids);
      for (const id of ids) expect(composeScene(alt(id), artSeedOf(id, "alt")).some((shape) => shape.part === part), `${id} composes ${part}`).toBe(true);
    }
  });

  for (const id of ALT_IDS) {
    it(`R3388000 ${id} draws its alt recipe at every Alt art level, base and Radiant, keeping its Radiant marks`, () => {
      const card = def(id);
      const seed = artSeedOf(id, "alt");
      const shapes = composeScene(alt(id), seed);
      // R277's marks in the words the rules box prints: a long text's short form (Heroic Power's) can end before its first.
      const radiantFace = faceModel({ defId: id, def: card, radiant: true });
      const marked = legacyText(radiantFace.short ?? radiantFace.text).marks.length > 0;
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
