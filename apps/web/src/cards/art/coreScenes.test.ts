// R3392000: Core's scenes (sceneRecipesCore1.ts to sceneRecipesCore3.ts, ART.md "Core's scenes"). Every Core
// entry but Ceaseless Void draws a scene recipe of its own in Core's family, on both faces with one geometry, at
// or above its rarity's floor (R3331001) and under the ceiling; a Unit's character is a figure or a creature part;
// a scene's cast stands in its own layer; each Mythic draws a part no other scene does; and the scenes this pass
// does not own draw as #3331 left them. The pixels are e2e/cypress/component/core-scenes.cy.tsx's.

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef, Rarity } from "@jackioh/shared";

import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { FIGURE_DEFAULT_LOOK, KY_TUTOR_LOOK } from "./figureKit.ts";
import { hashId } from "./hash.ts";
import { motifFor } from "./motifs.ts";
import { artSpec, type ArtSpec } from "./procedural.ts";
import { ART_DETAIL_MAX_BYTES, ART_DETAIL_MIN, ART_DETAIL_SHORT, artElements } from "./richness.ts";
import { composeScene, sceneRecipeOf, sceneUse, SCENE_LAYERS, type SceneLayer, type SceneRecipe } from "./scene.ts";
import { SCENE_KIT } from "./sceneKit.ts";
import { SCENE_RECIPES } from "./sceneRecipes.ts";
import { CORE_SCENES_1 } from "./sceneRecipesCore1.ts";
import { CORE_SCENES_2 } from "./sceneRecipesCore2.ts";
import { CORE_SCENES_3 } from "./sceneRecipesCore3.ts";
import { compositionOf, themeFor } from "./themes.ts";

const SVG_PREFIX = "data:image/svg+xml,";
/** Ceaseless Void, whose picture #3338 owns. */
const VOID = "core-100";
/** The rarities whose scene has a foreground, a middle and a background. */
const DEEP: readonly Rarity[] = ["Epic", "Legendary", "Mythic"];
/** The ladder the scenes' median detail climbs, tokens apart. */
const LADDER: readonly Rarity[] = ["Common", "Rare", "Epic", "Legendary", "Mythic"];
/** The Core cards that draw KY. */
const KY_SCENES = ["core-031", "core-051", "core-057", "core-082"] as const;

const FILES: readonly Readonly<Record<string, SceneRecipe>>[] = [CORE_SCENES_1, CORE_SCENES_2, CORE_SCENES_3];
const CORE: readonly CardDef[] = Object.values(CATALOG).filter((card) => card.set === "Core" && card.id !== VOID);

function svgOf(uri: string): string {
  return decodeURIComponent(uri.slice(SVG_PREFIX.length));
}

function recipe(id: string): SceneRecipe {
  const found = sceneRecipeOf(id);
  if (found === null) throw new Error(`no recipe for ${id}`);
  return found;
}

/** A card's spec as the procedural painter draws it, in its own composition (scene.test.ts's). */
function specOf(card: CardDef, radiant: boolean, scene?: SceneRecipe | null): ArtSpec {
  const theme = themeFor(card.tags, card.type);
  const motif = card.id.startsWith("stinkyscape-") ? null : motifFor(card.name, theme);
  const composition = compositionOf(card.id, card.type);
  return scene === undefined ? artSpec(card.id, theme, composition, radiant, motif) : artSpec(card.id, theme, composition, radiant, motif, scene);
}

function median(values: readonly number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 1 ? (sorted[mid] ?? 0) : ((sorted[mid - 1] ?? 0) + (sorted[mid] ?? 0)) / 2;
}

describe("R3392000 Core's scenes", () => {
  it("R3392000 every Core entry but Ceaseless Void draws a scene recipe of its own, in Core's family", () => {
    const ids = FILES.flatMap((file) => Object.keys(file));
    expect(new Set(ids).size, "no id in two files").toBe(ids.length);
    expect([...ids].sort()).toEqual(CORE.map((card) => card.id).sort());
    expect(ids).toHaveLength(110);
    expect(ids).not.toContain(VOID);
    const seen = new Map<string, string>();
    for (const file of FILES) {
      for (const [id, scene] of Object.entries(file)) {
        expect(SCENE_RECIPES[id], id).toBe(scene);
        expect(sceneRecipeOf(id), id).toBe(scene);
        expect(scene.family, id).toBe("core");
        expect(scene.ownSubject, id).toBeUndefined();
        const json = JSON.stringify(scene);
        expect(seen.get(json), `${id} draws ${seen.get(json) ?? ""}'s recipe`).toBeUndefined();
        seen.set(json, id);
      }
    }
    // Off the short list, every one.
    for (const card of CORE) expect(ART_DETAIL_SHORT.has(card.id), card.id).toBe(false);
  });

  it("R3392000 every Core scene clears its floor under the ceiling, and its Radiant face repaints every shape", () => {
    const elements = new Map<Rarity, number[]>();
    for (const card of CORE) {
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
    // Richer the rarer: the median scene of each rarity draws more than the one below it.
    const medians = LADDER.map((rarity) => median(elements.get(rarity) ?? []));
    medians.slice(1).forEach((value, k) => expect(value, `${LADDER[k + 1] ?? ""} over ${LADDER[k] ?? ""}`).toBeGreaterThan(medians[k] ?? Infinity));
  });

  it("R3392000 a Core Unit draws its character from the kits, never the painter's silhouette", () => {
    const units = CORE.filter((card) => card.type === "Unit");
    expect(units.length).toBeGreaterThan(0);
    for (const card of units) {
      const scene = recipe(card.id);
      const creature = (scene.layers.subject ?? []).some((placed) => SCENE_KIT[placed.part].category === "creatures");
      expect(scene.figure !== undefined || creature, `${card.id} draws no character`).toBe(true);
      // The face draws the scene alone: none of the painter's own subject.
      expect(specOf(card, false).ridges.map((ridge) => ridge.d), card.id).toEqual(composeScene(scene, card.id).map((shape) => shape.d));
    }
    // KY, wherever Core draws him, wears one look.
    for (const id of KY_SCENES) {
      const scene = recipe(id);
      const looks = [scene.figure, ...(scene.cast ?? [])].filter((figure) => figure !== undefined).map((figure) => figure.look);
      expect(looks, id).toContainEqual(KY_TUTOR_LOOK);
    }
  });

  it("R3392000 a cast figure stands in its layer, in front of that layer's parts", () => {
    const base: SceneRecipe = {
      family: "core",
      sky: ["blue-800", "sky-300"],
      light: { x: 50, y: 40, r: 30, paint: "white", opacity: 0.3 },
      layers: { mid: [{ part: "crate", x: 30, y: 80 }], subject: [{ part: "crate", x: 60, y: 80 }] },
    };
    const cast: SceneRecipe = { ...base, cast: [{ x: 40, y: 86, scale: 0.6, pose: "pose-stand", look: FIGURE_DEFAULT_LOOK, layer: "mid" }] };
    const without = composeScene(base, "core-cast-test");
    const shapes = composeScene(cast, "core-cast-test");
    const mid = shapes.filter((shape) => shape.layer === "mid");
    const crate = without.filter((shape) => shape.layer === "mid");
    // The mid crate first, as it is drawn with no cast, then the figure's shapes, tagged mid.
    expect(mid.slice(0, crate.length)).toEqual(crate);
    const figure = mid.slice(crate.length);
    expect(figure.length).toBeGreaterThan(0);
    expect(figure.every((shape) => shape.part !== "crate")).toBe(true);
    expect(new Set(figure.map((shape) => shape.part))).toContain("pose-stand");
    // Then the subject layer's crate, after every mid shape.
    const lastMid = shapes.map((shape) => shape.layer).lastIndexOf("mid");
    const firstSubject = shapes.findIndex((shape) => shape.layer === "subject");
    expect(firstSubject).toBeGreaterThan(lastMid);
    expect(shapes[firstSubject]?.part).toBe("crate");
    // Every Core cast figure draws its pose in its own layer.
    let casts = 0;
    for (const card of CORE) {
      const scene = recipe(card.id);
      const drawn = composeScene(scene, card.id);
      for (const extra of scene.cast ?? []) {
        expect(SCENE_LAYERS, card.id).toContain(extra.layer);
        expect(drawn.some((shape) => shape.layer === extra.layer && shape.part === extra.pose), `${card.id} ${extra.pose} in ${extra.layer}`).toBe(true);
        casts += 1;
      }
    }
    expect(casts).toBeGreaterThan(0);
  });

  it("R3392000 each Mythic's scene holds a part no other scene uses", () => {
    const partsOf = (id: string): Set<string> => new Set(composeScene(recipe(id), id).map((shape) => shape.part));
    const all = Object.keys(SCENE_RECIPES);
    const mythics = CORE.filter((card) => card.rarity === "Mythic");
    expect(mythics.length).toBeGreaterThan(0);
    for (const card of mythics) {
      const others = new Set(all.filter((id) => id !== card.id).flatMap((id) => [...partsOf(id)]));
      const own = [...partsOf(card.id)].filter((part) => !others.has(part));
      expect(own.length, `${card.id} draws only parts another scene draws`).toBeGreaterThan(0);
    }
  });

  it("R3392000 the scenes this pass does not own draw as #3331 left them", () => {
    // Measured on the base of #3392 (1fbbecc): a recipe with no cast draws the same stream, and no kit part changed.
    const pinned: Readonly<Record<string, readonly [number, number]>> = {
      "classic-074": [1210823500, 2735167760],
      "core-100": [894252645, 4246782103],
      "stinkyscape-015": [4157590552, 724339904],
    };
    for (const [id, [base, radiant]] of Object.entries(pinned)) {
      const card = CATALOG[id];
      if (card === undefined) throw new Error(`the catalog has no ${id}`);
      expect(recipe(id).cast, id).toBeUndefined();
      expect(hashId(proceduralCardArtUri(card, false)), id).toBe(base);
      expect(hashId(proceduralCardArtUri(card, true)), id).toBe(radiant);
    }
  });
});
