// R3331000: pictures composed as scenes (scene.ts) from the scene kit (sceneKit.ts) and the figure kit
// (figureKit.ts), by the recipes of sceneRecipes.ts: the kits are whole and clean and keep out of
// Radiant's gold window, the five proof cards draw their scenes on both faces with one geometry and
// clear their floors, and every card with no recipe draws the spec it drew before. The pixels are
// e2e/cypress/component/scene-proofs.cy.tsx's.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

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
import { drawFigure, FIGURE_DEFAULT_LOOK, FIGURE_KIT, FIGURE_LOOKS, figureLookFor, POSE_ANCHORS, type FigureLook } from "./figureKit.ts";
import { fmt, seededRandom } from "./hash.ts";
import { motifFor } from "./motifs.ts";
import { artSeedOf } from "./pictures.ts";
import { artSpec, ART_BOX, type ArtSpec } from "./procedural.ts";
import { ART_DETAIL_MIN, artElements } from "./richness.ts";
import {
  composeScene,
  SCENE_BOX,
  SCENE_KIT_MIN_PARTS,
  SCENE_KIT_MIN_PER_CATEGORY,
  SCENE_LAYERS,
  SCENE_LINES,
  SCENE_PALETTE,
  sceneRecipeOf,
  sceneUse,
  type KitCategory,
  type KitPart,
  type PaletteName,
  type SceneLayer,
  type SceneRecipe,
} from "./scene.ts";
import { SCENE_KIT } from "./sceneKit.ts";
import { SCENE_RECIPES } from "./sceneRecipes.ts";
import { proceduralArtUri, stinkyscapeArtUri, trainingGroundsArtUri } from "./svg.ts";
import { compositionFor, compositionOf, themeFor } from "./themes.ts";
import { legacySvg } from "./traininggrounds.ts";

const SVG_PREFIX = "data:image/svg+xml,";
const PROOFS = ["classic-074", "core-002", "core-098", "core-100", "stinkyscape-015"] as const;
const SCENERY: readonly KitCategory[] = ["terrain", "architecture", "plants", "creatures", "objects", "weather"];
/** The seeds every part is drawn from, enough to reach each branch its stream picks. */
const SEEDS = 40;
const PATH = /^M[-\d. MLQCAZ]+$/;
/** The rarer than Rare: a scene with a foreground, a middle and a background. */
const DEEP: readonly Rarity[] = ["Epic", "Legendary", "Mythic"];

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`the catalog has no ${id}`);
  return found;
}

function recipe(id: string): SceneRecipe {
  const found = sceneRecipeOf(id);
  if (found === null) throw new Error(`no recipe for ${id}`);
  return found;
}

function svgOf(uri: string): string {
  return decodeURIComponent(uri.slice(SVG_PREFIX.length));
}

/** A card's spec as the procedural painter draws it, in its own composition (R3338001), its recipe read from its id. */
function specOf(card: CardDef, radiant: boolean, scene?: SceneRecipe | null): ArtSpec {
  const theme = themeFor(card.tags, card.type);
  const motif = card.id.startsWith("stinkyscape-") ? null : motifFor(card.name, theme);
  const composition = compositionOf(card.id, card.type);
  return scene === undefined ? artSpec(card.id, theme, composition, radiant, motif) : artSpec(card.id, theme, composition, radiant, motif, scene);
}

/** The subject ridges a recipe keeps from the painter's own picture (`ownSubject`), on that face, or none. */
function ownSubjectOf(card: CardDef, radiant: boolean): ArtSpec["ridges"] {
  if (recipe(card.id).ownSubject !== true) return [];
  const old = specOf(card, radiant, null);
  return old.ridges.slice(old.subjectRidges[0], old.subjectRidges[1]);
}

/** Every path a recipe card's face draws, back to front: the scene's, its own subject after the subject layer. */
function sceneDs(card: CardDef, radiant: boolean): string[] {
  const shapes = composeScene(recipe(card.id), card.id);
  const end = shapes.filter((shape) => SCENE_LAYERS.indexOf(shape.layer) <= SCENE_LAYERS.indexOf("subject")).length;
  const ds = shapes.map((shape) => shape.d);
  return [...ds.slice(0, end), ...ownSubjectOf(card, radiant).map((ridge) => ridge.d), ...ds.slice(end)];
}

/** A part's stream for one seed, so each part is drawn at many points of its range. */
function stream(seed: number): () => number {
  return seededRandom(seed);
}

/** Every part of both kits, by id. */
const PARTS: readonly (readonly [string, KitPart])[] = [...Object.entries(SCENE_KIT), ...Object.entries(FIGURE_KIT)];

/** A hex colour's hue in degrees and saturation and lightness from 0 to 1 (cosmetics.test.tsx's). */
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

describe("R3331000 the scene kits", () => {
  it("R3331000 the kit holds SCENE_KIT_MIN_PARTS (60) parts or more, the figure kit's included, each scenery category six or more", () => {
    expect(SCENE_KIT_MIN_PARTS).toBe(60);
    expect(Object.keys(SCENE_KIT).length + Object.keys(FIGURE_KIT).length).toBeGreaterThanOrEqual(SCENE_KIT_MIN_PARTS);
    for (const category of SCENERY) {
      const count = Object.values(SCENE_KIT).filter((part) => part.category === category).length;
      expect(count, category).toBeGreaterThanOrEqual(SCENE_KIT_MIN_PER_CATEGORY);
    }
    // The scenery kit holds scenery only, and the figure kit a figure's parts only, each kind of them.
    for (const part of Object.values(SCENE_KIT)) expect(SCENERY).toContain(part.category);
    const figure = Object.values(FIGURE_KIT).map((part) => part.category);
    for (const category of figure) expect(SCENERY).not.toContain(category);
    for (const category of ["pose", "head", "hair", "outfit", "item"] as const) expect(figure, category).toContain(category);
    expect(Object.keys(SCENE_KIT).filter((id) => Object.hasOwn(FIGURE_KIT, id))).toEqual([]);
  });

  it("R3331000 every part draws clean shapes from any seed", () => {
    for (const [id, part] of PARTS) {
      for (let seed = 1; seed <= SEEDS; seed += 1) {
        const flip = seed % 2 === 0 ? 1 : -1;
        const shapes = part.draw({ x: 50, y: 60, s: 0.5 + (seed % 5) * 0.25, flip }, stream(seed));
        expect(shapes.length, id).toBeGreaterThanOrEqual(1);
        expect(shapes.length, id).toBeLessThanOrEqual(12);
        for (const shape of shapes) {
          expect(shape.d, id).toMatch(PATH);
          expect(shape.d, id).not.toMatch(/NaN|Infinity|undefined/);
          expect(Object.keys(SCENE_PALETTE), id).toContain(shape.paint);
          expect(shape.opacity, id).toBeGreaterThan(0);
          expect(shape.opacity, id).toBeLessThanOrEqual(1);
        }
      }
    }
  });

  it("R3331000 SCENE_PALETTE is palette.css's, and no base-face scene colour is in Radiant's gold window", () => {
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "../../palette.css"), "utf8");
    const declared = Object.fromEntries([...css.matchAll(/^\s*--pal-([a-z0-9-]+):\s*(#[0-9a-f]{6});$/gm)].map((match) => [match[1] ?? "", match[2] ?? ""]));
    expect(Object.keys(declared).length).toBeGreaterThan(0);
    expect(SCENE_PALETTE).toEqual(declared);
    // The window holds palette.css's golds, so the test can fail.
    for (const gold of ["gold-400", "gold-300", "yellow-400"] as const) expect(inRadiantGold(SCENE_PALETTE[gold]), gold).toBe(true);

    const painted = new Set<PaletteName>();
    for (const [, part] of PARTS) {
      for (let seed = 1; seed <= SEEDS; seed += 1) for (const shape of part.draw({ x: 50, y: 60, s: 1, flip: 1 }, stream(seed))) painted.add(shape.paint);
    }
    for (const line of Object.values(SCENE_LINES)) if (line !== null) painted.add(line.paint);
    for (const look of [FIGURE_DEFAULT_LOOK, ...FIGURE_LOOKS.map(([, found]) => found)]) {
      painted.add(look.skin);
      painted.add(look.cloth);
    }
    for (const scene of Object.values(SCENE_RECIPES)) {
      painted.add(scene.sky[0]);
      painted.add(scene.sky[1]);
      painted.add(scene.light.paint);
      for (const layer of SCENE_LAYERS) for (const placed of scene.layers[layer] ?? []) if (placed.paint !== undefined) painted.add(placed.paint);
      if (scene.figure !== undefined) {
        painted.add(scene.figure.look.skin);
        painted.add(scene.figure.look.cloth);
      }
    }
    for (const name of painted) expect(inRadiantGold(SCENE_PALETTE[name]), name).toBe(false);
    // And what each proof card's base face prints: every fill and stroke, its sky and its glow.
    for (const id of PROOFS) {
      const spec = specOf(def(id), false);
      const colors = [spec.sky.from, spec.sky.to, spec.glow.color, ...spec.ridges.flatMap((ridge) => [ridge.fill, ...(ridge.stroke === undefined ? [] : [ridge.stroke])])];
      for (const hex of colors) expect(inRadiantGold(hex), `${id}: ${hex}`).toBe(false);
    }
  });

  it("R3331000 a figure reads as its tag", () => {
    const looks: readonly FigureLook[] = [...FIGURE_LOOKS.map(([, look]) => look), FIGURE_DEFAULT_LOOK];
    const parts = (look: FigureLook): readonly (string | null)[] => [look.head, look.hair, look.outfit, look.item];
    for (const [i, a] of looks.entries()) {
      for (const b of looks.slice(i + 1)) {
        const differ = parts(a).filter((part, k) => part !== parts(b)[k]).length;
        expect(differ, `${JSON.stringify(a)} against ${JSON.stringify(b)}`).toBeGreaterThanOrEqual(2);
      }
    }
    expect(Object.keys(POSE_ANCHORS).length).toBeGreaterThanOrEqual(2);
    // The first tag the card holds wins, and a card with none takes the default.
    expect(figureLookFor(["Felinor"]).head).toBe("head-felinor");
    expect(figureLookFor(["Human", "Jlockeed"]).outfit).toBe("outfit-suit");
    expect(figureLookFor(["Terminus"]).outfit).toBe("outfit-void");
    expect(figureLookFor([])).toBe(FIGURE_DEFAULT_LOOK);
    // A figure draws its pose and every part its look names, each shape tagged with its part.
    for (const look of looks) {
      const shapes = drawFigure({ x: 50, y: 86, scale: 1, pose: "pose-stand", look }, stream(7));
      const drawn = new Set(shapes.map((shape) => shape.part));
      for (const part of ["pose-stand", ...parts(look)]) if (part !== null) expect(drawn, part).toContain(part);
      expect(shapes.some((shape) => shape.paint === look.skin)).toBe(true);
      expect(shapes.some((shape) => shape.paint === look.cloth)).toBe(true);
    }
  });
});

describe("R3331000 the five proof cards", () => {
  it("R3331000 the five proof cards draw their scenes, base and Radiant, with one geometry", () => {
    expect(SCENE_BOX).toBe(ART_BOX);
    expect(Object.keys(SCENE_RECIPES).sort()).toEqual([...PROOFS].sort());
    for (const id of PROOFS) {
      const card = def(id);
      const shapes = composeScene(recipe(id), id);
      const base = specOf(card, false);
      const radiant = specOf(card, true);
      expect(base.ridges.map((ridge) => ridge.d), id).toEqual(sceneDs(card, false));
      expect(radiant.ridges.map((ridge) => ridge.d), id).toEqual(base.ridges.map((ridge) => ridge.d));
      // A kept subject is the painter's own, painted as the painter paints that face.
      for (const face of [base, radiant]) {
        const own = ownSubjectOf(card, face === radiant);
        for (const ridge of own) expect(face.ridges, id).toContainEqual(ridge);
      }
      base.ridges.forEach((ridge, k) => expect(radiant.ridges[k]?.fill, `${id} #${k}`).not.toBe(ridge.fill));
      expect(radiant.sky, id).not.toEqual(base.sky);
      expect(base.rays, id).toHaveLength(0);
      expect(radiant.rays.length, id).toBeGreaterThan(0);
      // Each face's shapes differ from the picture it drew with no recipe (Classic keeps no fixture of its own).
      for (const face of [false, true]) expect(specOf(card, face).ridges, id).not.toEqual(specOf(card, face, null).ridges);
      // What the face shows draws every shape of the scene, on both faces.
      for (const face of [false, true]) {
        const svg = svgOf(proceduralCardArtUri(card, face));
        for (const shape of shapes) expect(svg.includes(`d="${shape.d}"`), `${id} ${face ? "radiant" : "base"}`).toBe(true);
        for (const ridge of ownSubjectOf(card, face)) expect(svg.includes(`d="${ridge.d}"`), `${id} ${face ? "radiant" : "base"} own`).toBe(true);
      }
    }
  });

  it("R3331000 Ceaseless Void's scene keeps its own composition's giant as its subject, its emblem on the giant's chest", () => {
    // R3338001 drew Ceaseless Void as a hole in the shape of a giant (`CARD_COMPOSITIONS`); its scene stands it
    // in front of the void's swirl in place of a figure. It is the only recipe that keeps its painter's subject.
    expect(PROOFS.filter((id) => recipe(id).ownSubject === true)).toEqual(["core-100"]);
    const card = def("core-100");
    const scene = recipe(card.id);
    expect(scene.figure).toBeUndefined();
    expect(scene.emblem).toBeUndefined();
    for (const radiant of [false, true]) {
      const spec = specOf(card, radiant);
      const old = specOf(card, radiant, null);
      expect(spec.composition).toBe("void");
      expect(ownSubjectOf(card, radiant).length).toBeGreaterThan(0);
      expect(spec.emblem).toEqual(old.emblem);
      const [from, to] = spec.subjectRidges;
      expect(spec.ridges.slice(to - ownSubjectOf(card, radiant).length, to)).toEqual(ownSubjectOf(card, radiant));
      expect(spec.ridges.slice(from, to).every((ridge) => ridge.role === "subject")).toBe(true);
    }
  });

  it("R3331000 each proof card clears its rarity's floor; an Epic or rarer uses far, mid and near", () => {
    for (const id of PROOFS) {
      const card = def(id);
      const floor = ART_DETAIL_MIN[card.rarity];
      const shapes = composeScene(recipe(id), id);
      expect(artElements(svgOf(proceduralCardArtUri(card, false))), id).toBeGreaterThanOrEqual(floor.elements);
      expect(sceneUse(shapes).layers, id).toBeGreaterThanOrEqual(floor.layers);
      const layers = new Set<SceneLayer>(shapes.map((shape) => shape.layer));
      if (DEEP.includes(card.rarity)) for (const layer of ["far", "mid", "near"] as const) expect(layers, `${id} ${layer}`).toContain(layer);
      expect(layers, id).toContain("subject");
    }
    // The proofs span the ladder: Common, Epic and Mythic.
    expect(new Set(PROOFS.map((id) => def(id).rarity))).toEqual(new Set(["Common", "Epic", "Mythic"]));
  });

  it("R3331000 Alt art and a repaint grow the same parts from their own seed", () => {
    for (const id of PROOFS) {
      const scene = recipe(id);
      const normal = composeScene(scene, id);
      for (const picture of ["alt", "night"] as const) {
        const seed = artSeedOf(id, picture);
        expect(seed, id).not.toBe(id);
        expect(sceneRecipeOf(seed), id).toBe(scene);
        const own = composeScene(scene, seed);
        expect(new Set(own.map((shape) => shape.part)), `${id} ${picture}`).toEqual(new Set(normal.map((shape) => shape.part)));
        expect(sceneUse(own), `${id} ${picture}`).toEqual(sceneUse(normal));
        expect(own.map((shape) => shape.d), `${id} ${picture}`).not.toEqual(normal.map((shape) => shape.d));
      }
    }
    // The painter draws that seed's scene.
    const card = def("core-002");
    const theme = themeFor(card.tags, card.type);
    const alt = svgOf(proceduralArtUri(artSeedOf(card.id, "alt"), theme, compositionFor(card.type), false, motifFor(card.name, theme)));
    for (const shape of composeScene(recipe(card.id), artSeedOf(card.id, "alt"))) expect(alt).toContain(`d="${shape.d}"`);
  });

  it("R3331000 a card with no recipe draws the spec it drew without one", () => {
    let checked = 0;
    for (const card of Object.values(CATALOG)) {
      if (sceneRecipeOf(card.id) !== null) continue;
      for (const radiant of [false, true]) expect(specOf(card, radiant), card.id).toEqual(specOf(card, radiant, null));
      checked += 1;
    }
    expect(checked).toBe(Object.keys(CATALOG).length - PROOFS.length);
    // Only a recipe card's id, or a picture's own seed of one, finds a recipe.
    for (const id of ["t-12", "", "core-0021", "core-002-1", "#core-002", "classic-074x"]) expect(sceneRecipeOf(id), id).toBeNull();
    expect(sceneRecipeOf("core-002#tarot")).toBe(recipe("core-002"));
  });

  it("R3331000 a Legacy card keeps its Core card's old subject", () => {
    const legacy = def("traininggrounds-041");
    const core = def("core-002");
    const theme = themeFor(core.tags, core.type);
    for (const radiant of [false, true]) {
      const svg = legacySvg(legacy.id, core.id, core.name, core.tags, core.type, radiant);
      const old = artSpec(core.id, theme, compositionFor(core.type), radiant, motifFor(core.name, theme), null);
      const [from, to] = old.subjectRidges;
      expect(to).toBeGreaterThan(from);
      for (const ridge of old.ridges.slice(from, to)) expect(svg).toContain(`d="${ridge.d}"`);
      for (const shape of composeScene(recipe(core.id), core.id)) expect(svg).not.toContain(`d="${shape.d}"`);
    }
  });

  it("R3331000 Out of frame's subject is the subject layer and the emblem", () => {
    for (const id of PROOFS) {
      const card = def(id);
      const theme = themeFor(card.tags, card.type);
      const shapes = composeScene(recipe(id), id);
      const part = (which: "backdrop" | "subject"): string =>
        svgOf(
          id.startsWith("stinkyscape-")
            ? stinkyscapeArtUri(id, card.name, card.tags, card.type, false, null, id, null, which)
            : proceduralArtUri(id, theme, compositionOf(card.id, card.type), false, motifFor(card.name, theme), null, null, which),
        );
      const subject = part("subject");
      const backdrop = part("backdrop");
      for (const shape of shapes) {
        expect(subject.includes(`d="${shape.d}"`), `${id} ${shape.part}`).toBe(shape.layer === "subject");
        expect(backdrop.includes(`d="${shape.d}"`), `${id} ${shape.part}`).toBe(shape.layer !== "subject");
      }
      const own = ownSubjectOf(card, false).map((ridge) => ridge.d);
      for (const d of own) {
        expect(subject, `${id} own`).toContain(`d="${d}"`);
        expect(backdrop, `${id} own`).not.toContain(`d="${d}"`);
      }
      const spec = specOf(card, false);
      const [from, to] = spec.subjectRidges;
      expect(spec.ridges.slice(from, to).map((ridge) => ridge.d), id).toEqual([...shapes.filter((shape) => shape.layer === "subject").map((shape) => shape.d), ...own]);
      // The emblem rides with the subject, the sky with the backdrop.
      const emblem = `translate(${fmt(spec.emblem.x)} ${fmt(spec.emblem.y)})`;
      expect(subject, id).toContain(emblem);
      expect(backdrop, id).not.toContain(emblem);
      expect(subject, id).not.toContain('fill="url(#s)"');
    }
  });

  it("R3331000 every painter hands a recipe card to the scene, and a Stinkyscape proof's inked shapes are outlined", () => {
    const card = def("core-002");
    const shapes = composeScene(recipe(card.id), card.id);
    const yard = svgOf(trainingGroundsArtUri(card.id, card.name, card.tags, card.type, false));
    const stinky = svgOf(stinkyscapeArtUri(card.id, card.name, card.tags, card.type, false));
    for (const shape of shapes) {
      expect(yard).toContain(`d="${shape.d}"`);
      expect(stinky).toContain(`d="${shape.d}"`);
    }
    // Core's family draws no line; Stinkyscape's outlines in fine black, Classic's in slate ink.
    expect(svgOf(proceduralCardArtUri(card, false))).not.toContain('fill-rule="evenodd" stroke=');
    for (const [id, family] of [["stinkyscape-015", "stinkyscape"], ["classic-074", "classic"]] as const) {
      const line = SCENE_LINES[family];
      expect(line, family).not.toBeNull();
      if (line === null) continue;
      const inked = composeScene(recipe(id), id).filter((shape) => shape.inked === true);
      expect(inked.length, id).toBeGreaterThan(0);
      const svg = svgOf(proceduralCardArtUri(def(id), false));
      for (const shape of inked) expect(svg, id).toContain(`d="${shape.d}" fill="${SCENE_PALETTE[shape.paint]}" opacity="${fmt(shape.opacity)}" fill-rule="evenodd" stroke="${SCENE_PALETTE[line.paint]}" stroke-width="${line.width}"`);
    }
  });
});
