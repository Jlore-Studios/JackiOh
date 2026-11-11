// R3406000: every card of #3406's table, Stinkyscape's later cards offered Alt art (the Dormant Starship aside), draws an
// alt scene recipe of its own (stinkyscape91AltRecipes1.ts to stinkyscape91AltRecipes3.ts), its subject restaged in
// another place, time or moment from a brief per card, read under its Alt art seed alone, at every Alt art level
// (R3015002), base and Radiant. Here: which cards hold one and which seeds read it; that a Starship Piece's, a Jlockeed
// weapon's and Dice of Doom's normal face keeps its own painter's picture; that each keeps its card's subject and moves
// its sky or its composition off the normal picture's; its detail floor, byte ceiling, geometry and gold (R3331001,
// R2725004); its emblem and Dice of Doom's motion (R2715001); the parts the pass added to the kit; and every Alt art
// level's face. The pixels each Alt picture changes (`ALT_PICTURE_CHANGE_MIN`) are
// e2e/cypress/component/stinkyscape91-alt-scenes.cy.tsx's, which jsdom cannot paint.

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
import { FIGURE_KIT, type FigureLook } from "./figureKit.ts";
import { ART_ROLES, FULL_ART_CARD_MOTION, FULL_ART_MOTION, type ArtMotion } from "./motion.ts";
import { artSeedOf } from "./pictures.ts";
import { artSpec, type ArtSpec } from "./procedural.ts";
import { ART_DETAIL_MAX_BYTES, ART_DETAIL_MIN, artElements } from "./richness.ts";
import { composeScene, SCENE_LAYERS, sceneRecipeOf, sceneUse, type SceneLayer, type SceneRecipe } from "./scene.ts";
import { SCENE_KIT, type SceneryPartId } from "./sceneKit.ts";
import { STINKY_ALT_KIT } from "./sceneKitStinkyAlt.ts";
import { STINKY_EARLY_ALT_KIT } from "./sceneKitStinkyEarlyAlt.ts";
import { ALT_PICTURE_CHANGE_MIN, ALT_SCENE_RECIPES, SCENE_RECIPES, STINKYSCAPE_91_ALT_RECIPES, STINKYSCAPE_91_RECIPES, STINKYSCAPE_ALT_RECIPES } from "./sceneRecipes.ts";
import { STARSHIP_PIECES, stinkyscapeSvg } from "./stinkyscape.ts";
import { stinkyscapeArtUri } from "./svg.ts";
import { compositionFor, themeFor } from "./themes.ts";

const SVG_PREFIX = "data:image/svg+xml,";
/** #3406's table: every card the pass restages, by id, sorted. */
const TABLE: readonly string[] = [
  ...["093", "094", "096", "097", "098", "099", "100", "101", "103", "104", "106", "107", "109", "110", "111", "113", "114", "115"],
  ...["116", "117", "122", "123", "124", "127", "128", "129", "132", "133", "135", "136", "137", "138", "139", "141", "142", "146"],
  ...["147", "148", "150", "151", "152", "153", "154", "155", "157", "158", "159", "160", "167", "169", "172", "173", "174", "177"],
].map((id) => `stinkyscape-${id}`);
/** The late Stinkyscape card offered Alt art that #3406's table leaves out, which #3394's table restages (R3394000). */
const LEFT_OUT = "stinkyscape-t-dormant-starship";
/** The cards of the table with no normal scene recipe, whose normal face their own painter draws: the Starship Pieces (R3333001), Jlockeed's three weapons and Dice of Doom (R3338001). */
const OWN_PAINTER: readonly string[] = [
  ...["093", "098", "100", "101", "104", "106", "107", "122", "132", "133", "136", "141", "142", "177"],
  ...["127", "128", "129", "154"],
].map((id) => `stinkyscape-${id}`);
/** What the deep space of an own painter's picture is painted in, which its Alt picture's sky holds neither of. */
const OWN_PAINTER_SPACE: readonly string[] = ["black", "slate-900"];
/** The parts that stand for an own painter card's piece (and what its brief sets it on), each in its Alt picture's subject layer. */
const PIECE_SUBJECT: Readonly<Record<string, readonly SceneryPartId[]>> = {
  "stinkyscape-093": ["nanobot-swarm"],
  "stinkyscape-098": ["hull-plate", "freighter"],
  "stinkyscape-100": ["cartridge", "hull-plate"],
  "stinkyscape-101": ["overdrive-engine", "fighter"],
  "stinkyscape-104": ["hull-plate", "fighter"],
  "stinkyscape-106": ["warp-ring", "starship"],
  "stinkyscape-107": ["rail-gun", "robot-arm"],
  "stinkyscape-122": ["cartridge"],
  "stinkyscape-127": ["tank"],
  "stinkyscape-128": ["great-cannon", "starship"],
  "stinkyscape-129": ["missile-pod", "freighter", "missile"],
  "stinkyscape-132": ["crystal"],
  "stinkyscape-133": ["capacitor-cell"],
  "stinkyscape-136": ["pod-capsule"],
  "stinkyscape-141": ["launcher-tube", "melon", "pineapple"],
  "stinkyscape-142": ["launcher-tube", "boat"],
  "stinkyscape-154": ["d100"],
  "stinkyscape-177": ["pod-capsule", "starship"],
};
/**
 * The parts a card's subject is, beside or in place of its figure, each drawn somewhere in its normal picture and in its
 * Alt picture's subject layer: the brief's same subject, restaged.
 */
const KEPT_SUBJECT: Readonly<Record<string, readonly SceneryPartId[]>> = {
  "stinkyscape-094": ["robot-wreck"],
  "stinkyscape-096": ["surf-wave", "crystal"],
  "stinkyscape-097": ["fighter", "ore-chunk"],
  "stinkyscape-099": ["robot-wreck"],
  "stinkyscape-111": ["light-diagram"],
  "stinkyscape-113": ["drone"],
  "stinkyscape-117": ["console"],
  "stinkyscape-123": ["crate"],
  "stinkyscape-124": ["rocket", "capsule"],
  "stinkyscape-135": ["alien", "kicking-sack"],
  "stinkyscape-139": ["wormhole"],
  "stinkyscape-146": ["ice-block"],
  "stinkyscape-147": ["spirit"],
  "stinkyscape-148": ["shuttle"],
  "stinkyscape-150": ["stink-cloud"],
  "stinkyscape-151": ["interceptor"],
  "stinkyscape-152": ["porthole"],
  "stinkyscape-153": ["credit-chip"],
  "stinkyscape-155": ["ice-shard"],
  "stinkyscape-157": ["energy-wall", "mech"],
  "stinkyscape-158": ["fighter"],
  "stinkyscape-160": ["starship", "fighter"],
  "stinkyscape-167": ["probe"],
  "stinkyscape-169": ["asteroid", "ore-chunk"],
  "stinkyscape-173": ["spice-tower"],
  "stinkyscape-174": ["starship"],
};
/** A subject the normal picture's figure holds that the Alt picture sets down: the item, and the part it stands as in the subject layer. */
const KEPT_ITEM: Readonly<Record<string, readonly [string, SceneryPartId]>> = {
  "stinkyscape-138": ["item-vial", "vial"],
};
/** The extra figures a brief keeps, by index: each the normal picture's own person, at the same index in both pictures. */
const KEPT_EXTRAS: Readonly<Record<string, readonly number[]>> = {
  "stinkyscape-115": [0],
  "stinkyscape-159": [0, 1, 2, 3, 4],
};
/** The cards whose brief goes inside their subject: the part the normal picture draws it as, and what inside it the Alt picture's subject layer draws. */
const INSIDE_VIEWS: Readonly<Record<string, readonly [SceneryPartId, SceneryPartId]>> = {
  "stinkyscape-116": ["space-station", "escalator"],
  "stinkyscape-172": ["giant-asteroid", "reactor"],
};
/** The parts the pass added to the kit, each with the cards whose Alt picture places it. */
const NEW_PARTS: Readonly<Record<keyof typeof STINKY_ALT_KIT, readonly string[]>> = {
  "nanobot-swarm": ["stinkyscape-093"],
  "hull-plate": ["stinkyscape-098", "stinkyscape-100", "stinkyscape-104", "stinkyscape-114"],
  cartridge: ["stinkyscape-100", "stinkyscape-122"],
  "overdrive-engine": ["stinkyscape-101"],
  "warp-ring": ["stinkyscape-106"],
  "rail-gun": ["stinkyscape-107"],
  "great-cannon": ["stinkyscape-128"],
  "missile-pod": ["stinkyscape-129"],
  "capacitor-cell": ["stinkyscape-133"],
  "pod-capsule": ["stinkyscape-136", "stinkyscape-177"],
  "launcher-tube": ["stinkyscape-141", "stinkyscape-142"],
  d100: ["stinkyscape-154"],
  pineapple: ["stinkyscape-141"],
  escalator: ["stinkyscape-116"],
  seashell: ["stinkyscape-148"],
};
/** R3394000: the pass's parts that #3394's early Starship Pieces reuse for their own (Healing Capsule's flank pod and hull plates, Cannon v4's cannon). */
const REUSED_BY_3394: readonly string[] = ["hull-plate", "great-cannon", "pod-capsule"];
/** What a figure both pictures draw may change: what it holds, wears over its outfit's colour, or its hair; never its head, outfit or skin. */
const LOOK_KEPT: readonly (keyof FigureLook)[] = ["head", "outfit", "skin"];
/** The rarer than Rare: a scene with a foreground, a middle and a background. */
const DEEP: readonly Rarity[] = ["Epic", "Legendary", "Mythic"];
/** R3017002's repaints, each a picture of its own under its treatment, which keep the card's normal picture. */
const REPAINTS = ["night", "watercolor", "stained-glass", "tarot"] as const;
/** How far, in the 100-unit box, an Alt picture's subject anchor moves from the normal one's when its composition changes by place (setAltScenes.test.tsx's). */
const ALT_SUBJECT_SHIFT_MIN = 8;
/** Or how far its subject's scale changes, as a ratio either way, when it changes by camera. */
const ALT_SUBJECT_SCALE_RATIO_MIN = 1.25;
/** Where an emblem may stand and how big it may be: inside the band every face's window shows (classicPlusScenes.test.ts's). */
const EMBLEM_BAND = { left: 10, right: 90, top: 22, bottom: 84, size: 8 } as const;
/** The first Stinkyscape number #3403 and #3406 call late: every card from it on, and every Stinkyscape token (`t-`). */
const LATE_FROM = 91;

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`no ${id}`);
  return found;
}

function alt(id: string): SceneRecipe {
  const found = STINKYSCAPE_91_ALT_RECIPES[id];
  if (found === undefined) throw new Error(`no alt recipe for ${id}`);
  return found;
}

function svgOf(uri: string): string {
  return decodeURIComponent(uri.slice(SVG_PREFIX.length));
}

/** Whether `id` is one of Stinkyscape's later cards: number 91 or later (a token of one among them), or a `t-` token. */
function isLate(card: CardDef): boolean {
  if (card.set !== "Stinkyscape") return false;
  const rest = card.id.slice("stinkyscape-".length);
  return rest.startsWith("t-") || Number(rest.split("-")[0]) >= LATE_FROM;
}

/** The picture `seedId` draws on `card`'s face, as its window's painter draws it (CardArt): the Stinkyscape painter's. */
function uriOf(card: CardDef, seedId: string, radiant: boolean, motion: ArtMotion | null = null): string {
  return stinkyscapeArtUri(card.id, card.name, card.tags, card.type, radiant, motion, seedId);
}

/** `card`'s Alt art spec as the Stinkyscape painter hands it to the scene (svg.ts `sceneArtSvg`), its recipe read from its Alt art seed. */
function altSpecOf(card: CardDef, radiant: boolean): ArtSpec {
  return artSpec(artSeedOf(card.id, "alt"), themeFor(card.tags, card.type), compositionFor(card.type), radiant, null);
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

/** The parts a recipe's subject layer places. */
function subjectOf(scene: SceneRecipe): Set<string> {
  return new Set((scene.layers.subject ?? []).map((placed) => placed.part));
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

describe("R3406000 Stinkyscape's later Alt art restaged", () => {
  it("R3406000 every card of #3406's table, and no other late Stinkyscape card but the Dormant Starship, has an alt recipe, read by its Alt art seed alone", () => {
    const offered = Object.values(CATALOG)
      .filter((card) => isLate(card) && offeredOn(VERSION_ALT_ART_ID, card.id))
      .map((card) => card.id)
      .sort();
    expect(offered).toEqual([...TABLE, LEFT_OUT].sort());
    expect(TABLE).toHaveLength(54);
    expect(Object.keys(STINKYSCAPE_91_ALT_RECIPES).sort()).toEqual(TABLE);
    for (const id of TABLE) expect(ALT_SCENE_RECIPES[id], id).toBe(alt(id));
    // R3394000: #3394's table, not this one, gives it an alt recipe (stinkyscapeAltScenes.test.tsx).
    expect(Object.hasOwn(STINKYSCAPE_91_ALT_RECIPES, LEFT_OUT)).toBe(false);
    expect(ALT_SCENE_RECIPES[LEFT_OUT]).toBe(STINKYSCAPE_ALT_RECIPES[LEFT_OUT]);
    expect(ALT_PICTURE_CHANGE_MIN).toBe(0.6);
    // The own painter cards are the table's cards with no normal recipe: the Starship Pieces, Jlockeed's weapons and Dice of Doom.
    expect(TABLE.filter((id) => !Object.hasOwn(STINKYSCAPE_91_RECIPES, id)).sort()).toEqual([...OWN_PAINTER].sort());
    for (const id of OWN_PAINTER) expect(Object.hasOwn(SCENE_RECIPES, id), id).toBe(false);
    expect(OWN_PAINTER.filter((id) => STARSHIP_PIECES.has(id))).toHaveLength(14);
    expect(Object.keys(PIECE_SUBJECT).sort()).toEqual([...OWN_PAINTER].sort());
    const seen = new Map<string, string>(Object.entries(SCENE_RECIPES).map(([id, scene]) => [JSON.stringify(scene), id]));
    for (const [id, scene] of Object.entries(ALT_SCENE_RECIPES)) if (!TABLE.includes(id)) seen.set(JSON.stringify(scene), `${id}#alt`);
    for (const id of TABLE) {
      const scene = alt(id);
      const kept = STINKYSCAPE_91_RECIPES[id] ?? null;
      expect(sceneRecipeOf(artSeedOf(id, "alt")), id).toBe(scene);
      // The normal picture and every repaint keep the card's own: its recipe, or for an own painter card none.
      expect(sceneRecipeOf(id), id).toBe(kept);
      for (const picture of REPAINTS) expect(sceneRecipeOf(artSeedOf(id, picture)), `${id} ${picture}`).toBe(kept);
      expect(sceneRecipeOf(`${id}#altx`), id).toBe(kept);
      // Every Alt art level draws the Alt picture (R3015002).
      expect(levelsOn(id, "alt-art"), id).toEqual(ladderOf("alt-art"));
      for (const level of ladderOf("alt-art")) expect(artPictureOf(level, def(id).set), `${id} ${level}`).toBe("alt");
      // Each keeps Stinkyscape's family, and so its ink line.
      expect(scene.family, id).toBe("stinkyscape");
      // No recipe, normal or alt, is another's.
      const key = JSON.stringify(scene);
      expect(seen.get(key), `${id} draws ${seen.get(key) ?? ""}'s recipe`).toBeUndefined();
      seen.set(key, `${id}#alt`);
      expect(composeScene(scene, artSeedOf(id, "alt")).some((shape) => shape.inked === true), id).toBe(true);
    }
  });

  it("R3406000 each Alt picture keeps its card's subject", () => {
    for (const id of TABLE) {
      const scene = alt(id);
      const was = STINKYSCAPE_91_RECIPES[id];
      const subject = subjectOf(scene);
      expect(subject.size > 0 || scene.figure !== undefined, `${id}: a subject`).toBe(true);
      if (was === undefined) {
        // An own painter card stands its piece in the subject layer as a part of the kit.
        const pieces = PIECE_SUBJECT[id] ?? [];
        expect(pieces.length, id).toBeGreaterThan(0);
        for (const part of pieces) expect(subject.has(part), `${id}: ${part} in the Alt picture's subject layer`).toBe(true);
        continue;
      }
      // A Unit drawn as a person stays that person.
      if (def(id).type === "Unit" && was.figure !== undefined) expect(scene.figure, id).toBeDefined();
      // A figure both pictures draw is the same person: the normal look's head, outfit and skin.
      if (was.figure !== undefined && scene.figure !== undefined) {
        for (const key of LOOK_KEPT) expect(scene.figure.look[key], `${id} ${key}`).toBe(was.figure.look[key]);
      }
      // A subject that is a part stays that part.
      for (const part of KEPT_SUBJECT[id] ?? []) {
        expect(partsOf(was).has(part), `${id}: ${part} in the normal picture`).toBe(true);
        expect(subject.has(part), `${id}: ${part} in the Alt picture's subject layer`).toBe(true);
      }
      // An item the normal figure holds, set down in the Alt picture.
      const item = KEPT_ITEM[id];
      if (item !== undefined) {
        expect(was.figure?.look.item, id).toBe(item[0]);
        expect(subject.has(item[1]), `${id}: ${item[1]}`).toBe(true);
      }
      // A brief that goes inside its subject draws what is inside it, where the normal picture draws it whole.
      const inside = INSIDE_VIEWS[id];
      if (inside !== undefined) {
        expect(subjectOf(was).has(inside[0]), `${id}: ${inside[0]} in the normal picture's subject layer`).toBe(true);
        expect(subject.has(inside[1]), `${id}: ${inside[1]} in the Alt picture's subject layer`).toBe(true);
      }
      // A kept extra figure is the normal picture's person at the same index.
      for (const k of KEPT_EXTRAS[id] ?? []) {
        const before = was.extras?.[k];
        const after = scene.extras?.[k];
        expect(before, `${id} #${k} in the normal picture`).toBeDefined();
        expect(after, `${id} #${k} in the Alt picture`).toBeDefined();
        for (const key of LOOK_KEPT) expect(after?.look[key], `${id} #${k} ${key}`).toBe(before?.look[key]);
      }
      // A card with neither a figure in both nor a kept part, item, inside or person would keep nothing.
      const keeps =
        (was.figure !== undefined && scene.figure !== undefined) ||
        (KEPT_SUBJECT[id]?.length ?? 0) > 0 ||
        item !== undefined ||
        inside !== undefined ||
        (KEPT_EXTRAS[id]?.length ?? 0) > 0;
      expect(keeps, `${id} keeps its subject`).toBe(true);
    }
  });

  it("R3406000 each Alt picture's sky or composition differs from its normal one", () => {
    for (const id of TABLE) {
      const scene = alt(id);
      const was = STINKYSCAPE_91_RECIPES[id];
      if (was === undefined) {
        // An own painter's picture is its piece on deep space: the Alt picture's sky is somewhere else.
        expect(scene.sky.filter((name) => OWN_PAINTER_SPACE.includes(name)), id).toEqual([]);
        continue;
      }
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

  it("R3406000 a Starship Piece's, a Jlockeed weapon's and Dice of Doom's normal face keeps its painter", () => {
    for (const id of OWN_PAINTER) {
      const card = def(id);
      for (const radiant of [false, true]) {
        for (const seed of [id, ...REPAINTS.map((picture) => artSeedOf(id, picture)), `${id}#altx`]) {
          const painted = stinkyscapeSvg(id, card.name, card.tags, card.type, radiant, null, seed);
          expect(svgOf(uriOf(card, seed, radiant)), `${seed} ${radiant ? "radiant" : "base"}`).toBe(painted);
        }
        expect(svgOf(uriOf(card, artSeedOf(id, "alt"), radiant)), id).not.toBe(stinkyscapeSvg(id, card.name, card.tags, card.type, radiant, null, artSeedOf(id, "alt")));
      }
    }
  });

  it("R3406000 each Alt picture clears its rarity's floor and the byte ceiling, base and Radiant, and prints no text", () => {
    for (const id of TABLE) {
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

  it("R3406000 base and Radiant draw one geometry, the Radiant face gilded, and no base face paints in Radiant's gold window", () => {
    for (const id of TABLE) {
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

  it("R3406000 every Alt picture is its own", () => {
    const pictures = new Map<string, string>();
    for (const id of TABLE) {
      for (const radiant of [false, true]) {
        for (const seed of [id, artSeedOf(id, "alt")]) {
          const uri = uriOf(def(id), seed, radiant);
          const where = `${seed} ${radiant ? "radiant" : "base"}`;
          expect(pictures.get(uri), `${where} draws ${pictures.get(uri) ?? ""}'s picture`).toBeUndefined();
          pictures.set(uri, where);
        }
      }
    }
    expect(pictures.size).toBe(TABLE.length * 4);
  });

  it("R3406000 the emblem stands in the band and Dice of Doom's motion moves every layer it names", () => {
    for (const id of TABLE) {
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
    // The one full-art card of the pass moves its Alt picture's every layer its motion names: the full-art cards' own
    // motions (R2715001), not R3340000's recipes for every other Legendary and Mythic.
    const fullArt = TABLE.filter((id) => FULL_ART_CARD_MOTION[id] !== undefined);
    expect(fullArt).toEqual(["stinkyscape-154"]);
    for (const id of fullArt) {
      const motion = FULL_ART_MOTION[id] ?? null;
      expect(motion, id).not.toBeNull();
      const svg = svgOf(uriOf(def(id), artSeedOf(id, "alt"), false, motion));
      for (const role of ART_ROLES) if (motion?.[role] !== undefined) expect(svg, `${id}: ${role}`).toContain(`class="art-m-${role}"`);
      if (motion?.props !== undefined) expect(altSpecOf(def(id), false).ridges.some((ridge) => ridge.role === "props"), id).toBe(true);
    }
  });

  it("R3406000 the pass's new parts are in the kit and placed", () => {
    const added = Object.keys(STINKY_ALT_KIT);
    expect(Object.keys(NEW_PARTS).sort()).toEqual([...added].sort());
    // They join the kit last, each as itself, so none takes the place of an earlier part (an id spread twice keeps its first place);
    // R3394000: but for #3394's parts, which follow them (stinkyscapeAltScenes.test.tsx).
    const later = Object.keys(STINKY_EARLY_ALT_KIT).length;
    expect(Object.keys(SCENE_KIT).slice(-added.length - later, -later)).toEqual(added);
    for (const [id, part] of Object.entries(STINKY_ALT_KIT)) {
      expect(SCENE_KIT[id as keyof typeof SCENE_KIT], id).toBe(part);
      expect(Object.hasOwn(FIGURE_KIT, id), `${id} is a figure part`).toBe(false);
    }
    for (const [part, ids] of Object.entries(NEW_PARTS)) {
      // None is a part of a normal picture or of another set's Alt picture, and each brief that names it places it; R3394000: but
      // #3394's early Starship Pieces reuse three of them (stinkyscapeAltScenes.test.tsx).
      for (const [id, scene] of Object.entries(SCENE_RECIPES)) expect(partsOf(scene).has(part), `${id} ${part}`).toBe(false);
      for (const [id, scene] of Object.entries(ALT_SCENE_RECIPES)) {
        const reused = Object.hasOwn(STINKYSCAPE_ALT_RECIPES, id) && REUSED_BY_3394.includes(part);
        if (!TABLE.includes(id) && !reused) expect(partsOf(scene).has(part), `${id}#alt ${part}`).toBe(false);
      }
      for (const id of ids) expect(partsOf(alt(id)).has(part), `${id} ${part}`).toBe(true);
      expect(TABLE.filter((id) => partsOf(alt(id)).has(part)), part).toEqual(ids);
    }
  });

  for (const id of TABLE) {
    it(`R3406000 ${id} draws its alt recipe at every Alt art level, base and Radiant, keeping its Radiant marks`, () => {
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
