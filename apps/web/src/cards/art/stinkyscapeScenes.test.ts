// R3391000: Stinkyscape's scenes (stinkyscapeRecipes.ts). Each card of #3391's table, and no other Stinkyscape
// card but #3403's (R3403000), draws from a recipe of its own that places every part its brief names; a Unit draws
// its character; each clears its rarity's floor, richer the rarer; both faces share one geometry under the ceiling
// and outside Radiant's gold window; no two pictures are alike; and the kit's new reach (craft and a placement's
// turn) leaves every older recipe as it drew. The pixels are e2e/cypress/component/stinkyscape-scenes.cy.tsx's.

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
import { seededRandom } from "./hash.ts";
import { artSeedOf } from "./pictures.ts";
import { artSpec } from "./procedural.ts";
import { ART_DETAIL_MAX_BYTES, ART_DETAIL_MIN, ART_DETAIL_SHORT, artElements } from "./richness.ts";
import { composeScene, SCENE_LAYERS, SCENE_PALETTE, sceneRecipeOf, sceneUse, type KitCategory, type PaletteName, type SceneLayer, type ScenePlacement, type SceneRecipe } from "./scene.ts";
import { SCENE_KIT, type SceneryPartId } from "./sceneKit.ts";
import { ALT_SCENE_RECIPES, SCENE_RECIPES, STINKYSCAPE_91_RECIPES } from "./sceneRecipes.ts";
import { isStinkyscapeArt, STARSHIP_PIECES } from "./stinkyscape.ts";
import { STINKYSCAPE_RECIPES } from "./stinkyscapeRecipes.ts";
import { compositionFor, themeFor } from "./themes.ts";

const SVG_PREFIX = "data:image/svg+xml,";
/** #3391's table, by id: every Stinkyscape card but the Starship Pieces, the Dormant Starship and Olivian Expanse. */
const PASS: readonly string[] = [1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 39, 41, 42, 44, 45, 46, 47, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89].map(
  (n) => `stinkyscape-${String(n).padStart(3, "0")}`,
);
/** The last card number the table covers (cards 1 to 90). */
const TABLE_LAST = 90;
/** The cards other issues draw (#3333's shipyard, #3338's Olivian Expanse). */
const OWNED_ELSEWHERE: readonly string[] = ["stinkyscape-004", "stinkyscape-t-dormant-starship"];
/** The ladder, bottom to top. */
const LADDER: readonly Rarity[] = ["Common", "Rare", "Epic", "Legendary", "Mythic"];
/** The rarer than Rare: a scene with a foreground, a middle and a background. */
const DEEP: readonly Rarity[] = ["Epic", "Legendary", "Mythic"];
/** Each Mythic's part of its own: the silhouette nobody else has (ART.md's bible). */
const MYTHIC_PARTS: Readonly<Record<string, SceneryPartId>> = {
  "stinkyscape-006": "skiff",
  "stinkyscape-016": "ancient-rod",
  "stinkyscape-036": "drone-battleship",
  "stinkyscape-044": "wire-coil",
  "stinkyscape-074": "blade-fighter",
  "stinkyscape-086": "fused-hull",
};
/** The parts that turn with their placement's `rotate`. */
const POINTING: readonly SceneryPartId[] = ["beam", "light-stream", "light-cone", "meteor", "smoke-trail", "comet-streak"];
const PATH = /^M[-\d. MLQCAZ]+$/;
/** The seven kinds of scenery (scene.ts's `KitCategory` but the figure's five). */
const SCENERY_KINDS: readonly KitCategory[] = ["terrain", "architecture", "plants", "creatures", "objects", "weather", "craft"];
/** The seeds every part is drawn from (scene.test.ts's). */
const SEEDS = 40;
/** Each card's parts its brief names, the scenery and the figure's (the pass's plan, #3391). */
const BRIEF_PARTS: Readonly<Record<string, readonly string[]>> = {
  "stinkyscape-001": ["wedge-fighter", "dock-gantry", "ray-gun", "beam", "frost-shard"],
  "stinkyscape-002": ["scout", "ice-cube", "star-glare", "frost-shard"],
  "stinkyscape-003": ["walker", "wreck", "moon-surface", "shockwave", "meteor", "smoke-trail"],
  "stinkyscape-005": ["cockpit-frame", "cracked-mask", "light-stream"],
  "stinkyscape-006": ["skiff", "ring-station", "vault-door", "credit-chips", "duffel", "pose-reach", "item-drill"],
  "stinkyscape-007": ["vault-door", "credit-chips", "swing-rope", "space-window", "comet-streak", "riveted-porthole", "light-grid"],
  "stinkyscape-008": ["ship-deck", "herb-cracker", "drum", "herb-sprig", "outfit-work-apron"],
  "stinkyscape-009": ["scout", "moon-surface", "boot-clamp", "light-cone"],
  "stinkyscape-010": ["ship-deck", "reaching-hand", "ringed-rift", "flame-column", "hex-shield"],
  "stinkyscape-011": ["ship-deck", "lectern", "ringed-rift", "frost-shard", "missiles", "flame-column"],
  "stinkyscape-012": ["bunk-pod", "space-window", "riveted-porthole", "sleeper", "shockwave", "ringed-rift"],
  "stinkyscape-013": ["corridor", "cracked-helmet"],
  "stinkyscape-014": ["ship-deck", "outfit-hydraulic"],
  "stinkyscape-015": ["hangar-wall", "ship-deck", "pose-spread"],
  "stinkyscape-016": ["hover-platform", "ancient-rod", "debris", "light-stream"],
  "stinkyscape-017": ["cell-bars", "ship-deck", "anvil", "axe-head", "outfit-work-apron", "item-hammer"],
  "stinkyscape-018": ["server-core", "ship-deck", "beam"],
  "stinkyscape-019": ["containment-sphere", "cosmality"],
  "stinkyscape-020": ["ring-station", "ship-deck", "beam", "pose-reach", "item-scanner"],
  "stinkyscape-021": ["moon-surface", "berry-vine", "gas-giant"],
  "stinkyscape-022": ["wedge-fighter", "gunship", "disruptor-beacon", "beam", "ringed-rift"],
  "stinkyscape-023": ["wedge-fighter", "disruptor-beacon", "shockwave"],
  "stinkyscape-024": ["disruptor-beacon", "gas-giant", "space-shards"],
  "stinkyscape-025": ["ship-deck", "debris", "light-grid", "pose-reach", "outfit-labcoat", "item-stylus"],
  "stinkyscape-026": ["retro-rocket", "ring-station", "ship-deck", "gas-giant", "pose-reach", "hair-professor", "outfit-academic", "item-chart"],
  "stinkyscape-027": ["card", "eye-drone", "moon-surface", "drone-core", "debris"],
  "stinkyscape-028": ["eye-drone", "drone-core", "reaching-hand"],
  "stinkyscape-029": ["eye-drone", "moon-surface", "drone-core"],
  "stinkyscape-030": ["eye-drone", "antenna-mast", "drone-core"],
  "stinkyscape-031": ["drone-corvette", "dock-gantry", "drone-core", "light-cone"],
  "stinkyscape-032": ["eye-drone", "drone-core", "light-stream"],
  "stinkyscape-033": ["eye-drone", "drone-corvette", "drone-core", "comet-streak"],
  "stinkyscape-034": ["eye-drone", "dock-gantry", "conveyor", "reactor-core", "drone-core", "welding-arm"],
  "stinkyscape-035": ["drone-corvette", "lava-flats", "drone-core", "debris", "smoke-trail"],
  "stinkyscape-036": ["eye-drone", "drone-battleship", "dock-gantry", "drone-core"],
  "stinkyscape-037": ["wedge-fighter", "ring-station", "hover-platform", "light-stream", "light-cone", "headset"],
  "stinkyscape-039": ["glass-case", "outfit-labcoat"],
  "stinkyscape-041": ["helper-drone", "tram", "drone-core"],
  "stinkyscape-042": ["control-desk", "ship-deck", "drone-core", "goggles", "outfit-labcoat"],
  "stinkyscape-044": ["riveted-porthole", "drone-core", "wire-coil"],
  "stinkyscape-045": ["eye-drone", "hangar-wall", "ship-deck", "item-core"],
  "stinkyscape-046": ["eye-drone", "ring-station", "control-desk", "pilot-seat", "cockpit-frame", "welding-arm", "coffee-bulb", "pose-sit"],
  "stinkyscape-047": ["drone-corvette", "control-desk", "pilot-seat", "riveted-porthole", "welding-arm", "sleeper"],
  "stinkyscape-068": ["wedge-fighter", "dock-gantry", "ship-deck"],
  "stinkyscape-069": ["needle-fighter", "hangar-wall", "ship-deck"],
  "stinkyscape-070": ["wreck", "ghost-ship", "debris", "head-wraith", "outfit-wreck"],
  "stinkyscape-071": ["wreck", "debris", "shockwave", "star-glare", "pose-spread", "head-horn", "outfit-wreck"],
  "stinkyscape-072": ["arrow-fighter", "beam", "gas-giant"],
  "stinkyscape-073": ["arrow-fighter", "beam", "hex-shield"],
  "stinkyscape-074": ["blade-fighter", "wreck", "moon-surface", "beam", "smoke-trail", "star-glare", "hex-shield"],
  "stinkyscape-075": ["miner-craft"],
  "stinkyscape-076": ["miner-craft", "barge", "beam"],
  "stinkyscape-077": ["wedge-fighter", "trader", "ring-station"],
  "stinkyscape-078": ["miner-craft", "barge", "credit-chips"],
  "stinkyscape-080": ["wedge-fighter", "flagship"],
  "stinkyscape-081": ["frame-ship", "hangar-wall", "ship-deck"],
  "stinkyscape-082": ["gunship", "debris", "smoke-trail"],
  "stinkyscape-083": ["wedge-fighter", "gun-ring", "pilot-seat", "cockpit-frame", "pose-sit"],
  "stinkyscape-084": ["reactor-core", "debris"],
  "stinkyscape-085": ["frame-ship", "wreck", "antenna-mast", "moon-surface", "smoke-trail", "light-cone"],
  "stinkyscape-086": ["fused-hull", "debris", "smoke-trail"],
  "stinkyscape-087": ["ring-station", "hover-platform", "light-stream", "pose-reach", "shades"],
  "stinkyscape-088": ["corridor", "item-venom-blade"],
  "stinkyscape-089": ["market-stall", "credit-chips", "scales"],
};

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

function median(values: readonly number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 1 ? (sorted[mid] ?? 0) : ((sorted[mid - 1] ?? 0) + (sorted[mid] ?? 0)) / 2;
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

/** A card's base picture as its face draws it. */
const BASE: ReadonlyMap<string, string> = new Map(PASS.map((id) => [id, svgOf(proceduralCardArtUri(def(id), false))]));

function baseOf(id: string): string {
  const found = BASE.get(id);
  if (found === undefined) throw new Error(`${id} was not drawn`);
  return found;
}

describe("R3391000 Stinkyscape's scenes", () => {
  it("R3391000 the pass's 64 entries and no other Stinkyscape card but #3403's table draw from recipes of their own", () => {
    expect(PASS).toHaveLength(64);
    expect(Object.keys(STINKYSCAPE_RECIPES).sort()).toEqual([...PASS].sort());
    for (const id of PASS) {
      expect(def(id).id, id).toBe(id);
      expect(SCENE_RECIPES[id], id).toBe(STINKYSCAPE_RECIPES[id]);
      expect(recipe(id).family, id).toBe("stinkyscape");
      expect(STARSHIP_PIECES.has(id), id).toBe(false);
    }
    expect(new Set(PASS.map((id) => STINKYSCAPE_RECIPES[id])).size).toBe(PASS.length);
    for (const id of OWNED_ELSEWHERE) expect(sceneRecipeOf(id), id).toBeNull();
    // No other Stinkyscape card draws a scene but #3403's table (R3403000: the later cards and the early cards' tokens it
    // names, stinkyscape91Scenes.test.ts), and every card numbered 1 to 90 the table leaves out prints Starship Piece or
    // is another issue's.
    for (const id of PASS) expect(Object.hasOwn(STINKYSCAPE_91_RECIPES, id), id).toBe(false);
    for (const card of Object.values(CATALOG)) {
      if (isStinkyscapeArt(card.id) && !PASS.includes(card.id) && !Object.hasOwn(STINKYSCAPE_91_RECIPES, card.id)) expect(sceneRecipeOf(card.id), card.id).toBeNull();
    }
    const rest = Object.values(CATALOG).filter((card) => {
      const number = /^stinkyscape-(\d{3})$/.exec(card.id)?.[1];
      return number !== undefined && Number(number) <= TABLE_LAST && !PASS.includes(card.id);
    });
    expect(rest.length).toBeGreaterThan(0);
    for (const card of rest) expect(STARSHIP_PIECES.has(card.id) || OWNED_ELSEWHERE.includes(card.id), card.id).toBe(true);
  });

  it("R3391000 each places every part its brief names", () => {
    expect(Object.keys(BRIEF_PARTS).sort()).toEqual([...PASS].sort());
    for (const id of PASS) {
      const placed = new Set(composeScene(recipe(id), id).map((shape) => shape.part));
      const named = BRIEF_PARTS[id] ?? [];
      expect(named.length, id).toBeGreaterThan(0);
      for (const part of named) {
        expect(Object.hasOwn(SCENE_KIT, part) || Object.hasOwn(FIGURE_KIT, part), `${id}: ${part} is a kit part`).toBe(true);
        expect(placed, `${id} places ${part}`).toContain(part);
      }
    }
  });

  it("R3391000 a Unit draws its character, a figure or a craft or creature part in its subject layer, and no recipe keeps the painter's own subject", () => {
    for (const id of PASS) {
      const scene = recipe(id);
      expect(scene.ownSubject, id).not.toBe(true);
      if (def(id).type !== "Unit") continue;
      const subject = composeScene(scene, id).filter((shape) => shape.layer === "subject");
      const character = subject.some((shape) => Object.hasOwn(FIGURE_KIT, shape.part) || ["craft", "creatures"].includes(SCENE_KIT[shape.part as SceneryPartId]?.category ?? ""));
      expect(character, id).toBe(true);
    }
    // A figure's look starts from its tag's, so a Human Unit wears a Human's head.
    for (const id of PASS) {
      const { figure } = recipe(id);
      if (figure !== undefined && def(id).tags.includes("Human")) expect(figure.look.head, id).toBe(figureLookFor(["Human"]).head);
    }
  });

  it("R3391000 each clears its rarity's floor and is off ART_DETAIL_SHORT; an Epic or rarer draws far, mid and near; a Legendary or Mythic every layer", () => {
    for (const id of PASS) {
      const card = def(id);
      const floor = ART_DETAIL_MIN[card.rarity];
      const shapes = composeScene(recipe(id), id);
      expect(artElements(baseOf(id)), id).toBeGreaterThanOrEqual(floor.elements);
      expect(sceneUse(shapes).layers, id).toBeGreaterThanOrEqual(floor.layers);
      expect(ART_DETAIL_SHORT.has(id), id).toBe(false);
      const layers = new Set<SceneLayer>(shapes.map((shape) => shape.layer));
      expect(layers, id).toContain("subject");
      if (DEEP.includes(card.rarity)) for (const layer of ["far", "mid", "near"] as const) expect(layers, `${id} ${layer}`).toContain(layer);
      if (card.rarity === "Legendary" || card.rarity === "Mythic") expect(layers.size, id).toBe(SCENE_LAYERS.length);
    }
  });

  it("R3391000 richer the rarer: the median element count rises strictly from Common to Mythic", () => {
    const medians = LADDER.map((rarity) => median(PASS.filter((id) => def(id).rarity === rarity).map((id) => artElements(baseOf(id)))));
    for (const [k, value] of medians.entries()) expect(value, LADDER[k]).toBeGreaterThan(0);
    medians.slice(1).forEach((value, k) => expect(value, `${LADDER[k + 1] ?? ""} over ${LADDER[k] ?? ""}`).toBeGreaterThan(medians[k] ?? Infinity));
  });

  it("R3391000 each Mythic's subject places a part no other recipe places", () => {
    const mythics = PASS.filter((id) => def(id).rarity === "Mythic").sort();
    expect(mythics).toEqual(Object.keys(MYTHIC_PARTS).sort());
    for (const [id, part] of Object.entries(MYTHIC_PARTS)) {
      const subject = composeScene(recipe(id), id).filter((shape) => shape.layer === "subject").map((shape) => shape.part);
      expect(subject, id).toContain(part);
      for (const [other, scene] of Object.entries(SCENE_RECIPES)) {
        if (other === id) continue;
        const placed = SCENE_LAYERS.flatMap((layer) => (scene.layers[layer] ?? []).map((placement) => placement.part));
        expect(placed, `${other} places ${id}'s ${part}`).not.toContain(part);
      }
    }
  });

  it("R3391000 both faces share one geometry, the Radiant repainted, every shape drawn, under ART_DETAIL_MAX_BYTES, no base colour in Radiant's gold window", () => {
    for (const id of PASS) {
      const card = def(id);
      const theme = themeFor(card.tags, card.type);
      const scene = recipe(id);
      const shapes = composeScene(scene, id);
      const base = artSpec(id, theme, compositionFor(card.type), false, null, scene);
      const radiant = artSpec(id, theme, compositionFor(card.type), true, null, scene);
      expect(base.ridges.map((ridge) => ridge.d), id).toEqual(shapes.map((shape) => shape.d));
      expect(radiant.ridges.map((ridge) => ridge.d), id).toEqual(base.ridges.map((ridge) => ridge.d));
      base.ridges.forEach((ridge, k) => expect(radiant.ridges[k]?.fill, `${id} #${k}`).not.toBe(ridge.fill));
      expect(radiant.sky, id).not.toEqual(base.sky);
      expect(radiant.rays.length, id).toBeGreaterThan(0);
      for (const face of [false, true]) {
        const uri = proceduralCardArtUri(card, face);
        expect(uri.length, `${id} ${face ? "radiant" : "base"}`).toBeLessThanOrEqual(ART_DETAIL_MAX_BYTES);
        const svg = face ? svgOf(uri) : baseOf(id);
        for (const shape of shapes) expect(svg.includes(`d="${shape.d}"`), `${id} ${face ? "radiant" : "base"} ${shape.part}`).toBe(true);
      }
      // What the base face prints, and every colour the recipe names, keeps out of Radiant's gold window.
      const colors = [base.sky.from, base.sky.to, base.glow.color, ...base.ridges.flatMap((ridge) => [ridge.fill, ...(ridge.stroke === undefined ? [] : [ridge.stroke])])];
      for (const hex of colors) expect(inRadiantGold(hex), `${id}: ${hex}`).toBe(false);
      const named = new Set<PaletteName>([scene.sky[0], scene.sky[1], scene.light.paint]);
      for (const layer of SCENE_LAYERS) for (const placed of scene.layers[layer] ?? []) if (placed.paint !== undefined) named.add(placed.paint);
      for (const figure of [...(scene.figure === undefined ? [] : [scene.figure]), ...(scene.extras ?? [])]) {
        named.add(figure.look.skin);
        named.add(figure.look.cloth);
      }
      for (const name of named) expect(inRadiantGold(SCENE_PALETTE[name]), `${id}: ${name}`).toBe(false);
    }
  });

  it("R3391000 no two of the pass's pictures are alike, and Alt art grows the same parts from its own seed", () => {
    const radiants = PASS.map((id) => proceduralCardArtUri(def(id), true));
    expect(new Set(PASS.map(baseOf)).size).toBe(PASS.length);
    expect(new Set(radiants).size).toBe(PASS.length);
    // No two recipes place the same parts in the same places.
    const layouts = PASS.map((id) => JSON.stringify(recipe(id).layers));
    expect(new Set(layouts).size).toBe(PASS.length);
    // R3394000: an entry of #3394's table draws an alt recipe of its own instead (stinkyscapeAltScenes.test.tsx).
    const regrown = PASS.filter((id) => !Object.hasOwn(ALT_SCENE_RECIPES, id));
    expect(regrown).toHaveLength(30);
    for (const id of regrown) {
      const scene = recipe(id);
      const normal = composeScene(scene, id);
      const seed = artSeedOf(id, "alt");
      expect(sceneRecipeOf(seed), id).toBe(scene);
      const own = composeScene(scene, seed);
      expect(new Set(own.map((shape) => shape.part)), id).toEqual(new Set(normal.map((shape) => shape.part)));
      expect(sceneUse(own), id).toEqual(sceneUse(normal));
      expect(own.map((shape) => shape.d), id).not.toEqual(normal.map((shape) => shape.d));
    }
  });

  it("R3391000 a placement turns a pointing part only when it says so, and no other recipe places a turn", () => {
    // A turn of 0 draws a pointing part as it is drawn unturned, and a turn on a part that does not point changes nothing.
    for (const id of PASS) {
      const scene = recipe(id);
      const turnedEverywhere = (placed: ScenePlacement): ScenePlacement => (POINTING.includes(placed.part) ? { ...placed, rotate: placed.rotate ?? 0 } : { ...placed, rotate: 90 });
      const layers = Object.fromEntries(SCENE_LAYERS.map((layer) => [layer, (scene.layers[layer] ?? []).map(turnedEverywhere)]));
      expect(composeScene({ ...scene, layers }, id), id).toEqual(composeScene(scene, id));
    }
    // So every other recipe (the proofs left in sceneRecipes.ts, the other sets' passes' and the Alt art's), none of
    // which places a turn, draws as it always did.
    const others = [...Object.entries(SCENE_RECIPES).filter(([id]) => !PASS.includes(id)), ...Object.entries(ALT_SCENE_RECIPES)];
    expect(others.map(([id]) => id)).toEqual(expect.arrayContaining(["classic-074", "core-002", "core-098", "core-100"]));
    const turned = others.flatMap(([id, scene]) => SCENE_LAYERS.flatMap((layer) => (scene.layers[layer] ?? []).filter((placed) => placed.rotate !== undefined).map((placed) => `${id} ${placed.part}`)));
    expect(turned).toEqual([]);
  });

  // One test per kind of scenery: the same parts, seeds, turns and checks as one sweep of the kit, each a seventh of its time.
  for (const category of SCENERY_KINDS) {
    it(`R3391000 every ${category} part draws clean shapes at any turn; the pointing parts turn, the rest ignore it`, () => {
      // Problems are gathered and checked once: an expect per shape over every part, seed and turn outruns the test's time.
      const palette = new Set<string>(Object.keys(SCENE_PALETTE));
      const problems: string[] = [];
      for (const [id, part] of Object.entries(SCENE_KIT) as [SceneryPartId, (typeof SCENE_KIT)[SceneryPartId]][]) {
        if (part.category !== category) continue;
        const pointing = POINTING.includes(id);
        for (let seed = 1; seed <= SEEDS; seed += 1) {
          const flip = seed % 2 === 0 ? 1 : -1;
          const s = 0.5 + (seed % 5) * 0.25;
          const still = JSON.stringify(part.draw({ x: 50, y: 60, s, flip }, seededRandom(seed)));
          for (const rotate of [0, 45, 90, 180, 270]) {
            const shapes = part.draw({ x: 50, y: 60, s, flip, rotate }, seededRandom(seed));
            const where = `${id} seed ${seed} at ${rotate}`;
            if (shapes.length < 1 || shapes.length > 12) problems.push(`${where}: ${shapes.length} shapes`);
            for (const shape of shapes) {
              if (!PATH.test(shape.d) || /NaN|Infinity|undefined/.test(shape.d)) problems.push(`${where}: path ${shape.d.slice(0, 40)}`);
              if (!palette.has(shape.paint)) problems.push(`${where}: paint ${shape.paint}`);
              if (!(shape.opacity > 0 && shape.opacity <= 1)) problems.push(`${where}: opacity ${shape.opacity}`);
            }
            const same = JSON.stringify(shapes) === still;
            if (rotate === 0 || !pointing ? !same : same) problems.push(`${where}: ${pointing ? "did not turn" : "turned"}`);
          }
        }
      }
      expect(problems).toEqual([]);
    });
  }

  it("R3391000 the sweeps above cover every part of the kit, the pointing parts among them, and craft are a kind of their own", () => {
    for (const part of Object.values(SCENE_KIT)) expect(SCENERY_KINDS).toContain(part.category);
    for (const id of POINTING) expect(SCENE_KIT[id], id).toBeDefined();
    expect(Object.values(SCENE_KIT).filter((part) => part.category === "craft").length).toBeGreaterThanOrEqual(6);
  });
});
