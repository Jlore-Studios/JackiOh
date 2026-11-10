// R3394000: every card of #3394's table, Stinkyscape's cards 1 to 90 offered Alt art and the Dormant Starship, draws an
// alt scene recipe of its own (stinkyscapeAltRecipes1.ts to stinkyscapeAltRecipes3.ts), its subject restaged in another
// place, time or moment from a brief per card, read under its Alt art seed alone, at every Alt art level (R3015002),
// base and Radiant. Here: which cards hold one and which seeds read it; that Olivian Expanse's, an early Starship
// Piece's and the Dormant Starship's normal face keeps its own painter's picture; that each keeps its card's subject and
// moves its sky or its composition off the normal picture's; its detail floor, byte ceiling, geometry and gold
// (R3331001, R2725004); its emblem; the parts the pass added to the kit; and every Alt art level's face. The pixels each
// Alt picture changes (`ALT_PICTURE_CHANGE_MIN`) are e2e/cypress/component/stinkyscape-alt-scenes.cy.tsx's, which jsdom
// cannot paint.

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
import { FULL_ART_CARD_MOTION } from "./motion.ts";
import { artSeedOf } from "./pictures.ts";
import { artSpec, type ArtSpec } from "./procedural.ts";
import { ART_DETAIL_MAX_BYTES, ART_DETAIL_MIN, artElements } from "./richness.ts";
import { composeScene, SCENE_LAYERS, sceneRecipeOf, sceneUse, type SceneLayer, type SceneRecipe } from "./scene.ts";
import { SCENE_KIT, type SceneryPartId } from "./sceneKit.ts";
import { STINKY_EARLY_ALT_KIT } from "./sceneKitStinkyEarlyAlt.ts";
import { ALT_PICTURE_CHANGE_MIN, ALT_SCENE_RECIPES, SCENE_RECIPES, STINKYSCAPE_ALT_RECIPES } from "./sceneRecipes.ts";
import { STARSHIP_PIECES, stinkyscapeSvg } from "./stinkyscape.ts";
import { STINKYSCAPE_RECIPES } from "./stinkyscapeRecipes.ts";
import { stinkyscapeArtUri } from "./svg.ts";
import { compositionFor, themeFor } from "./themes.ts";

const SVG_PREFIX = "data:image/svg+xml,";
/** The Dormant Starship: a Stinkyscape token, so one of the later cards by number, which #3394's table restages. */
const DORMANT = "stinkyscape-t-dormant-starship";
/** #3394's table: every card the pass restages, by id, sorted. */
const TABLE: readonly string[] = [
  ...[
    ...["001", "004", "005", "006", "010", "011", "014", "018", "020", "021", "025", "026", "030", "033", "034", "035"],
    ...["039", "041", "042", "043", "044", "045", "048", "050", "051", "054", "055", "056", "062", "064", "065", "066"],
    ...["068", "069", "070", "072", "073", "075", "077", "078", "079", "081", "082", "083", "086", "087", "088"],
  ].map((id) => `stinkyscape-${id}`),
  DORMANT,
];
/**
 * The cards of the table with no normal scene recipe, whose normal face their own painter draws: Olivian Expanse
 * (R3338001), the early Starship Pieces (R3333001) and the Dormant Starship (R3333000).
 */
const OWN_PAINTER: readonly string[] = [
  ...["004", "043", "048", "050", "051", "054", "055", "056", "062", "064", "065", "066", "079"].map((id) => `stinkyscape-${id}`),
  DORMANT,
];
/** What the deep space of an own painter's picture is painted in, which its Alt picture's sky holds neither of. */
const OWN_PAINTER_SPACE: readonly string[] = ["black", "slate-900"];
/** What Olivian Expanse's endless violet expanse is painted in (R3338001), which its Alt picture's olive-grey sky holds none of. */
const OLIVIAN_TONES: readonly string[] = ["violet-300", "violet-500", "violet-700", "purple-400"];
/** Olivian Expanse, whose painter draws a violet expanse rather than deep space. */
const OLIVIAN = "stinkyscape-004";
/** The parts that stand for an own painter card's piece (and what its brief sets it on), each in its Alt picture's subject layer. */
const PIECE_SUBJECT: Readonly<Record<string, readonly SceneryPartId[]>> = {
  "stinkyscape-004": ["cockpit-frame", "console"],
  "stinkyscape-043": ["pod-capsule", "hull-plate"],
  "stinkyscape-048": ["chassis-frame"],
  "stinkyscape-050": ["beam-emitter", "starship", "fighter"],
  "stinkyscape-051": ["repeater-gun", "freighter"],
  "stinkyscape-054": ["beam-emitter", "glass-case"],
  "stinkyscape-055": ["repeater-gun", "starship"],
  "stinkyscape-056": ["great-cannon", "barge"],
  "stinkyscape-062": ["mining-drill", "solar-panels"],
  "stinkyscape-064": ["mining-drill", "round-bot"],
  "stinkyscape-065": ["mining-drill", "comet"],
  "stinkyscape-066": ["mining-drill", "asteroid"],
  "stinkyscape-079": ["orca-hull"],
  [DORMANT]: ["dormant-hull", "cable"],
};
/**
 * The parts a card's subject is, beside or in place of its figure, each drawn somewhere in its normal picture and in its
 * Alt picture's subject layer: the brief's same subject, restaged.
 */
const KEPT_SUBJECT: Readonly<Record<string, readonly SceneryPartId[]>> = {
  "stinkyscape-001": ["ray-gun", "frost-shard"],
  "stinkyscape-010": ["flame-column", "wisp", "hex-shield"],
  "stinkyscape-011": ["frost-shard"],
  "stinkyscape-018": ["server-core"],
  "stinkyscape-021": ["berry-vine"],
  "stinkyscape-030": ["eye-drone"],
  "stinkyscape-033": ["drone-corvette", "eye-drone"],
  "stinkyscape-034": ["eye-drone", "welding-arm"],
  "stinkyscape-035": ["drone-corvette"],
  "stinkyscape-039": ["crystal"],
  "stinkyscape-041": ["helper-drone", "drone-core"],
  "stinkyscape-044": ["drone-core"],
  "stinkyscape-068": ["wedge-fighter"],
  "stinkyscape-069": ["needle-fighter"],
  "stinkyscape-072": ["arrow-fighter"],
  "stinkyscape-073": ["arrow-fighter", "hex-shield"],
  "stinkyscape-075": ["miner-craft", "crystal"],
  "stinkyscape-077": ["trader", "crate"],
  "stinkyscape-078": ["miner-craft"],
  "stinkyscape-081": ["frame-ship"],
  "stinkyscape-082": ["gunship"],
  "stinkyscape-083": ["gun-ring"],
  "stinkyscape-086": ["fused-hull"],
};
/** The cards whose figure both pictures draw: the normal picture's person, restaged. */
const KEPT_FIGURE: readonly string[] = ["006", "014", "020", "025", "026", "042", "045", "070", "087", "088"].map((id) => `stinkyscape-${id}`);
/** A card whose brief restages its subject as another part: the normal picture's subject and the Alt picture's (Masochism Unmasked's mask, whole before the hit). */
const RESTAGED: Readonly<Record<string, readonly [SceneryPartId, SceneryPartId]>> = {
  "stinkyscape-005": ["cracked-mask", "chrome-mask"],
};
/** The ship Units whose normal figure is their pilot, their character their craft (R3391000), which their Alt picture draws without the pilot. */
const PILOTED: readonly string[] = ["stinkyscape-083"];
/** The parts the pass added to the kit, each with the cards whose Alt picture places it. */
const NEW_PARTS: Readonly<Record<keyof typeof STINKY_EARLY_ALT_KIT, readonly string[]>> = {
  "chrome-mask": ["stinkyscape-005"],
  "chassis-frame": ["stinkyscape-048"],
  "beam-emitter": ["stinkyscape-050", "stinkyscape-054"],
  "repeater-gun": ["stinkyscape-051", "stinkyscape-055"],
  "orca-hull": ["stinkyscape-079"],
  "solar-panels": ["stinkyscape-062"],
  "dormant-hull": [DORMANT],
};
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
  const found = STINKYSCAPE_ALT_RECIPES[id];
  if (found === undefined) throw new Error(`no alt recipe for ${id}`);
  return found;
}

function svgOf(uri: string): string {
  return decodeURIComponent(uri.slice(SVG_PREFIX.length));
}

/** Whether `card` is one of Stinkyscape's later cards: number 91 or later (a token of one among them), or a `t-` token. */
function isLate(card: CardDef): boolean {
  if (card.set !== "Stinkyscape") return false;
  const rest = card.id.slice("stinkyscape-".length);
  return rest.startsWith("t-") || Number(rest.split("-")[0]) >= LATE_FROM;
}

/** Whether `card` is one of Stinkyscape's cards 1 to 90 (a token of one among them). */
function isEarly(card: CardDef): boolean {
  return card.set === "Stinkyscape" && !isLate(card);
}

/** The picture `seedId` draws on `card`'s face, as its window's painter draws it (CardArt): the Stinkyscape painter's. */
function uriOf(card: CardDef, seedId: string, radiant: boolean): string {
  return stinkyscapeArtUri(card.id, card.name, card.tags, card.type, radiant, null, seedId);
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

describe("R3394000 Stinkyscape's early Alt art restaged", () => {
  it("R3394000 every card of #3394's table, Stinkyscape's cards 1 to 90 offered Alt art and the Dormant Starship, has an alt recipe, read by its Alt art seed alone", () => {
    const offered = Object.values(CATALOG)
      .filter((card) => (isEarly(card) || card.id === DORMANT) && offeredOn(VERSION_ALT_ART_ID, card.id))
      .map((card) => card.id)
      .sort();
    expect(offered).toEqual(TABLE);
    expect(TABLE).toHaveLength(48);
    expect(Object.keys(STINKYSCAPE_ALT_RECIPES).sort()).toEqual(TABLE);
    for (const id of TABLE) expect(ALT_SCENE_RECIPES[id], id).toBe(alt(id));
    expect(ALT_PICTURE_CHANGE_MIN).toBe(0.6);
    // The own painter cards are the table's cards with no normal recipe: Olivian Expanse, the early Starship Pieces and the Dormant Starship.
    expect(TABLE.filter((id) => !Object.hasOwn(STINKYSCAPE_RECIPES, id)).sort()).toEqual([...OWN_PAINTER].sort());
    for (const id of OWN_PAINTER) expect(Object.hasOwn(SCENE_RECIPES, id), id).toBe(false);
    expect(OWN_PAINTER.filter((id) => STARSHIP_PIECES.has(id))).toHaveLength(12);
    expect(Object.keys(PIECE_SUBJECT).sort()).toEqual([...OWN_PAINTER].sort());
    const seen = new Map<string, string>(Object.entries(SCENE_RECIPES).map(([id, scene]) => [JSON.stringify(scene), id]));
    for (const [id, scene] of Object.entries(ALT_SCENE_RECIPES)) if (!TABLE.includes(id)) seen.set(JSON.stringify(scene), `${id}#alt`);
    for (const id of TABLE) {
      const scene = alt(id);
      const kept = STINKYSCAPE_RECIPES[id] ?? null;
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

  it("R3394000 each Alt picture keeps its card's subject", () => {
    // The cards whose figure both pictures draw are the ones the brief keeps a person for, and no other.
    expect(TABLE.filter((id) => STINKYSCAPE_RECIPES[id]?.figure !== undefined && alt(id).figure !== undefined)).toEqual(KEPT_FIGURE);
    for (const id of TABLE) {
      const scene = alt(id);
      const was = STINKYSCAPE_RECIPES[id];
      const subject = subjectOf(scene);
      expect(subject.size > 0 || scene.figure !== undefined, `${id}: a subject`).toBe(true);
      if (was === undefined) {
        // An own painter card stands its piece in the subject layer as a part of the kit.
        const pieces = PIECE_SUBJECT[id] ?? [];
        expect(pieces.length, id).toBeGreaterThan(0);
        for (const part of pieces) expect(subject.has(part), `${id}: ${part} in the Alt picture's subject layer`).toBe(true);
        continue;
      }
      // A Unit drawn as a person stays that person; a ship Unit whose normal figure is its pilot stays its ship.
      if (def(id).type === "Unit" && was.figure !== undefined && !PILOTED.includes(id)) expect(scene.figure, id).toBeDefined();
      // A figure both pictures draw is the same person: the normal look's head, outfit and skin.
      if (was.figure !== undefined && scene.figure !== undefined) {
        for (const key of LOOK_KEPT) expect(scene.figure.look[key], `${id} ${key}`).toBe(was.figure.look[key]);
      }
      // A subject that is a part stays that part.
      for (const part of KEPT_SUBJECT[id] ?? []) {
        expect(partsOf(was).has(part), `${id}: ${part} in the normal picture`).toBe(true);
        expect(subject.has(part), `${id}: ${part} in the Alt picture's subject layer`).toBe(true);
      }
      // A subject restaged as another part: the normal picture's in its subject layer, the brief's in the Alt picture's.
      const restaged = RESTAGED[id];
      if (restaged !== undefined) {
        expect(subjectOf(was).has(restaged[0]), `${id}: ${restaged[0]} in the normal picture's subject layer`).toBe(true);
        expect(subject.has(restaged[1]), `${id}: ${restaged[1]} in the Alt picture's subject layer`).toBe(true);
      }
      // A card with neither a figure in both nor a kept or restaged part would keep nothing.
      const keeps = KEPT_FIGURE.includes(id) || (KEPT_SUBJECT[id]?.length ?? 0) > 0 || restaged !== undefined;
      expect(keeps, `${id} keeps its subject`).toBe(true);
    }
    // Halo's pilot sits in its normal picture, and its Alt picture draws the ring alone.
    for (const id of PILOTED) {
      expect(STINKYSCAPE_RECIPES[id]?.figure, id).toBeDefined();
      expect(alt(id).figure, id).toBeUndefined();
    }
  });

  it("R3394000 each Alt picture's sky or composition differs from its normal one", () => {
    for (const id of TABLE) {
      const scene = alt(id);
      const was = STINKYSCAPE_RECIPES[id];
      if (was === undefined) {
        // An own painter's picture is its piece on deep space (Olivian Expanse's a violet expanse): the Alt picture's sky is somewhere else.
        expect(scene.sky.filter((name) => OWN_PAINTER_SPACE.includes(name)), id).toEqual([]);
        if (id === OLIVIAN) expect(scene.sky.filter((name) => OLIVIAN_TONES.includes(name)), id).toEqual([]);
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

  it("R3394000 Olivian Expanse's, a Starship Piece's and the Dormant Starship's normal face keeps its painter", () => {
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

  it("R3394000 each Alt picture clears its rarity's floor and the byte ceiling, base and Radiant, and prints no text", () => {
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

  it("R3394000 base and Radiant draw one geometry, the Radiant face gilded, and no base face paints in Radiant's gold window", () => {
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

  it("R3394000 every Alt picture is its own", () => {
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

  it("R3394000 the emblem stands in the band, no props stand in the subject layer, and no card of the pass moves as full art", () => {
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
    // None of the table has full-art motion of its own (R2715001), so no Alt picture owes one a layer per role.
    expect(TABLE.filter((id) => FULL_ART_CARD_MOTION[id] !== undefined)).toEqual([]);
  });

  it("R3394000 the pass's new parts are in the kit and placed", () => {
    const added = Object.keys(STINKY_EARLY_ALT_KIT);
    expect(Object.keys(NEW_PARTS).sort()).toEqual([...added].sort());
    // They join the kit last, each as itself, so none takes the place of an earlier part (an id spread twice keeps its first place).
    expect(Object.keys(SCENE_KIT).slice(-added.length)).toEqual(added);
    for (const [id, part] of Object.entries(STINKY_EARLY_ALT_KIT)) {
      expect(SCENE_KIT[id as keyof typeof SCENE_KIT], id).toBe(part);
      expect(Object.hasOwn(FIGURE_KIT, id), `${id} is a figure part`).toBe(false);
    }
    for (const [part, ids] of Object.entries(NEW_PARTS)) {
      // None is a part of a normal picture or of another card's Alt picture, and each brief that names it places it.
      for (const [id, scene] of Object.entries(SCENE_RECIPES)) expect(partsOf(scene).has(part), `${id} ${part}`).toBe(false);
      for (const [id, scene] of Object.entries(ALT_SCENE_RECIPES)) if (!TABLE.includes(id)) expect(partsOf(scene).has(part), `${id}#alt ${part}`).toBe(false);
      for (const id of ids) expect(partsOf(alt(id)).has(part), `${id} ${part}`).toBe(true);
      expect(TABLE.filter((id) => partsOf(alt(id)).has(part)), part).toEqual(ids);
    }
  });

  for (const id of TABLE) {
    it(`R3394000 ${id} draws its alt recipe at every Alt art level, base and Radiant, keeping its Radiant marks`, () => {
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
