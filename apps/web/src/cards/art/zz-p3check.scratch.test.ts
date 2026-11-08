// SCRATCH (part 3's agent deletes it before it ends): classicPlusScenes.test.ts's and artRichness.test.ts's checks on
// CLASSIC_PLUS_RECIPES_3's ids alone, so another file's unfinished cards do not hide them.
import { writeFileSync } from "node:fs";

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
import { CLASSIC_PLUS_RECIPES_3 } from "./classicPlusRecipes3.ts";
import { FIGURE_KIT, figureLookFor } from "./figureKit.ts";
import { motifFor } from "./motifs.ts";
import { ART_ROLES, FULL_ART_MOTION } from "./motion.ts";
import { artSeedOf } from "./pictures.ts";
import { artSpec, type ArtSpec } from "./procedural.ts";
import { ART_DETAIL_MAX_BYTES, ART_DETAIL_MIN, artElements } from "./richness.ts";
import { SCENE_KIT } from "./sceneKit.ts";
import { composeScene, SCENE_LAYERS, sceneRecipeOf, sceneUse, type SceneLayer, type SceneRecipe } from "./scene.ts";
import { proceduralArtUri } from "./svg.ts";
import { compositionOf, themeFor } from "./themes.ts";

const SVG_PREFIX = "data:image/svg+xml,";
const MINE: readonly CardDef[] = Object.keys(CLASSIC_PLUS_RECIPES_3).map((id) => {
  const card = CATALOG[id];
  if (card === undefined) throw new Error(`no card ${id}`);
  return card;
});
const DEEP: readonly Rarity[] = ["Epic", "Legendary", "Mythic"];
const TARGET: Readonly<Record<Rarity, number>> = { Common: 34, Token: 34, Rare: 46, Epic: 68, Legendary: 100, Mythic: 150 };

function recipe(id: string): SceneRecipe {
  const found = sceneRecipeOf(id);
  if (found === null) throw new Error(`no recipe for ${id}`);
  return found;
}
function svgOf(uri: string): string {
  return decodeURIComponent(uri.slice(SVG_PREFIX.length));
}
function specOf(card: CardDef, radiant: boolean): ArtSpec {
  const theme = themeFor(card.tags, card.type);
  return artSpec(card.id, theme, compositionOf(card.id, card.type), radiant, motifFor(card.name, theme));
}
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
  return h >= RADIANT_GOLD_HUE_MIN_DEG && h <= RADIANT_GOLD_HUE_MAX_DEG && s >= RADIANT_GOLD_SATURATION_MIN && l >= RADIANT_GOLD_LIGHTNESS_MIN && l <= RADIANT_GOLD_LIGHTNESS_MAX;
}

describe("part 3", () => {
  it("recipes, floors, targets, bytes", () => {
    expect(MINE).toHaveLength(44);
    const errors: string[] = [];
    for (const card of MINE) {
      const scene = recipe(card.id);
      if (scene.family !== "classic") errors.push(`${card.id} family`);
      const shapes = composeScene(scene, card.id);
      if (!shapes.some((shape) => shape.inked === true)) errors.push(`${card.id} no inked`);
      const floor = ART_DETAIL_MIN[card.rarity];
      const elements = artElements(svgOf(proceduralCardArtUri(card, false)));
      if (elements < floor.elements) errors.push(`${card.id} under floor ${elements}`);
      if (elements < TARGET[card.rarity]) errors.push(`${card.id} under target ${elements} < ${TARGET[card.rarity]}`);
      if (sceneUse(shapes).layers < floor.layers) errors.push(`${card.id} layers ${sceneUse(shapes).layers}`);
      const layers = new Set<SceneLayer>(shapes.map((shape) => shape.layer));
      if (!layers.has("subject")) errors.push(`${card.id} no subject`);
      if (DEEP.includes(card.rarity)) for (const layer of ["far", "mid", "subject", "near"] as const) if (!layers.has(layer)) errors.push(`${card.id} no ${layer}`);
      for (const radiant of [false, true]) {
        const length = proceduralCardArtUri(card, radiant).length;
        if (length > ART_DETAIL_MAX_BYTES) errors.push(`${card.id} ${radiant ? "radiant" : "base"} ${length} bytes`);
        if (length > 46 * 1024) errors.push(`(soft) ${card.id} ${radiant ? "radiant" : "base"} ${(length / 1024).toFixed(1)} KiB > 46`);
      }
      const base = specOf(card, false);
      const radiant = specOf(card, true);
      expect(base.ridges.map((ridge) => ridge.d), card.id).toEqual(shapes.map((shape) => shape.d));
      expect(radiant.ridges.map((ridge) => ridge.d), card.id).toEqual(base.ridges.map((ridge) => ridge.d));
      base.ridges.forEach((ridge, k) => expect(radiant.ridges[k]?.fill, `${card.id} #${k}`).not.toBe(ridge.fill));
      expect(radiant.sky, card.id).not.toEqual(base.sky);
      expect(base.rays, card.id).toHaveLength(0);
      expect(radiant.rays.length, card.id).toBeGreaterThan(0);
      const colors = [base.sky.from, base.sky.to, base.glow.color, ...base.ridges.flatMap((ridge) => [ridge.fill, ...(ridge.stroke === undefined ? [] : [ridge.stroke])])];
      for (const hex of colors) if (inRadiantGold(hex)) errors.push(`${card.id} gold ${hex}`);
      const { emblem } = scene;
      if (emblem === undefined || emblem.size > 8 || emblem.x < 10 || emblem.x > 90 || emblem.y < 22 || emblem.y > 84) errors.push(`${card.id} emblem`);
      for (const layer of SCENE_LAYERS) for (const placed of scene.layers[layer] ?? []) if (placed.props === true && layer === "subject") errors.push(`${card.id} props in subject`);
      for (const extra of scene.extras ?? []) {
        if (!shapes.some((shape) => shape.layer === extra.layer && Object.hasOwn(FIGURE_KIT, shape.part) && shape.part === extra.pose)) errors.push(`${card.id} extra`);
      }
      const seed = artSeedOf(card.id, "alt");
      const own = composeScene(scene, seed);
      expect(sceneUse(own), card.id).toEqual(sceneUse(shapes));
      expect(own.map((shape) => shape.d), card.id).not.toEqual(shapes.map((shape) => shape.d));
      const motion = FULL_ART_MOTION[card.id];
      if (motion !== undefined) {
        const theme = themeFor(card.tags, card.type);
        const svg = svgOf(proceduralArtUri(card.id, theme, compositionOf(card.id, card.type), false, motifFor(card.name, theme), motion));
        for (const role of ART_ROLES) if (motion[role] !== undefined && !svg.includes(`class="art-m-${role}"`)) errors.push(`${card.id} motion ${role}`);
        if (motion.props !== undefined && !specOf(card, false).ridges.some((ridge) => ridge.role === "props")) errors.push(`${card.id} props role`);
      }
    }
    // Units: own looks, unique across every Classic+ recipe that exists.
    const looks = new Map<string, string>();
    for (const card of Object.values(CATALOG).filter((c) => c.set === "Classic+" && c.type === "Unit")) {
      const scene = sceneRecipeOf(card.id);
      if (scene?.figure === undefined) continue;
      const key = JSON.stringify(scene.figure.look);
      const other = looks.get(key);
      if (other !== undefined && (Object.hasOwn(CLASSIC_PLUS_RECIPES_3, card.id) || Object.hasOwn(CLASSIC_PLUS_RECIPES_3, other))) errors.push(`${card.id} looks as ${other}`);
      looks.set(key, card.id);
    }
    for (const card of MINE.filter((c) => c.type === "Unit")) {
      const scene = recipe(card.id);
      if (card.id === "classicplus-064") {
        if (scene.figure !== undefined || !(scene.layers.subject ?? []).some((p) => p.part === "boar")) errors.push(`${card.id} boar`);
        continue;
      }
      if (scene.figure === undefined) errors.push(`${card.id} no figure`);
      else if (JSON.stringify(scene.figure.look) === JSON.stringify(figureLookFor(card.tags))) errors.push(`${card.id} default look`);
    }
    expect(errors).toEqual([]);
  });
});

it("costs", () => {
  const ids = (process.env.CP_COST ?? "").split(",").filter((id) => id !== "");
  const out: string[] = [];
  for (const id of ids) {
    const scene = recipe(id);
    const shapes = composeScene(scene, id);
    const cost = new Map<string, { n: number; bytes: number }>();
    // Shapes come out in placement order per layer; tag each with its placement by recomposing one placement at a time.
    for (const layer of SCENE_LAYERS) {
      (scene.layers[layer] ?? []).forEach((placed, k) => {
        const one = composeScene({ ...scene, figure: undefined, extras: [], layers: { [layer]: [placed] } }, id);
        cost.set(`${layer}#${k} ${placed.part}${placed.count === undefined ? "" : `x${placed.count}`} s${placed.scale ?? 1}`, { n: one.length, bytes: one.reduce((sum, s) => sum + s.d.length + 60, 0) });
      });
    }
    const figure = shapes.filter((s) => Object.hasOwn(FIGURE_KIT, s.part));
    cost.set("figures", { n: figure.length, bytes: figure.reduce((sum, s) => sum + s.d.length + 60, 0) });
    for (const s of figure) {
      const key = `  fig ${s.part}`;
      const was = cost.get(key) ?? { n: 0, bytes: 0 };
      cost.set(key, { n: was.n + 1, bytes: was.bytes + s.d.length + 60 });
    }
    const rows = [...cost.entries()].sort((a, b) => b[1].bytes - a[1].bytes).map(([k, v]) => `  ${(v.bytes / 1024).toFixed(1)}K ${v.n} ${k}`);
    out.push(`${id} total ${(shapes.reduce((sum, s) => sum + s.d.length + 60, 0) / 1024).toFixed(1)}K\n${rows.join("\n")}`);
  }
  writeFileSync("/tmp/p3/cost.txt", `${out.join("\n")}\n`);
});

it("part costs", () => {
  if (process.env.CP_PARTS === undefined) return;
  const rows: string[] = [];
  for (const part of Object.keys(SCENE_KIT)) {
    const one = composeScene({ family: "classic", sky: ["black", "black"], light: { x: 0, y: 0, r: 1, paint: "black", opacity: 0 }, layers: { mid: [{ part: part as keyof typeof SCENE_KIT, x: 50, y: 50 }] } }, "x");
    rows.push(`${one.reduce((sum, s) => sum + s.d.length + 60, 0)}\t${one.length}\t${part}`);
  }
  writeFileSync("/tmp/p3/parts.tsv", `${rows.sort((a, b) => Number(b.split("\t")[0]) - Number(a.split("\t")[0])).join("\n")}\n`);
});

it("figure part costs", () => {
  if (process.env.CP_FIG === undefined) return;
  const rows: string[] = [];
  const base = { family: "classic" as const, sky: ["black", "black"] as const, light: { x: 0, y: 0, r: 1, paint: "black" as const, opacity: 0 }, layers: {} };
  const look = figureLookFor(["Human"]);
  for (const part of Object.keys(FIGURE_KIT)) {
    const cat = FIGURE_KIT[part as keyof typeof FIGURE_KIT].category;
    const over = cat === "hair" ? { hair: part } : cat === "outfit" ? { outfit: part } : cat === "item" ? { item: part } : cat === "head" ? { head: part } : {};
    const pose = cat === "pose" ? part : "pose-stand";
    const shapes = composeScene({ ...base, figure: { x: 50, y: 86, scale: 1, pose: pose as "pose-stand", look: { ...look, ...over } as typeof look } }, "x").filter((s) => s.part === part);
    rows.push(`${shapes.reduce((sum, s) => sum + s.d.length + 60, 0)}\t${shapes.length}\t${cat}\t${part}`);
  }
  writeFileSync("/tmp/p3/fig.tsv", `${rows.sort((a, b) => Number(b.split("\t")[0]) - Number(a.split("\t")[0])).join("\n")}\n`);
});
