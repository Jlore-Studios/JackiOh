// SCRATCH (#3392 build): a preview of cards' pictures and kit parts as HTML under /tmp/preview, shot
// by /tmp/preview/shot.sh. Never delivered: delete before the run ends.

import { mkdirSync, writeFileSync } from "node:fs";

import { it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { FIGURE_KIT, drawFigure, type FigureLook, type PoseId } from "./figureKit.ts";
import { seededRandom } from "./hash.ts";
import { artElements } from "./richness.ts";
import { composeScene, sceneRecipeOf, sceneUse, SCENE_PALETTE } from "./scene.ts";
import { SCENE_KIT } from "./sceneKit.ts";

const OUT = "/tmp/preview";
const SVG_PREFIX = "data:image/svg+xml,";

it("preview", () => {
  mkdirSync(OUT, { recursive: true });
  const ids = (process.env.PREVIEW_IDS ?? "").split(",").filter((id) => id !== "");
  const parts = (process.env.PREVIEW_PARTS ?? "").split(",").filter((id) => id !== "");
  const size = Number(process.env.PREVIEW_SIZE ?? "220");
  const name = process.env.PREVIEW_NAME ?? "preview";
  const cells: string[] = [];
  const report: string[] = [];
  for (const id of ids) {
    const card = CATALOG[id] as CardDef | undefined;
    if (card === undefined) continue;
    const row: string[] = [];
    for (const radiant of [false, true]) {
      const uri = proceduralCardArtUri(card, radiant);
      const svg = decodeURIComponent(uri.slice(SVG_PREFIX.length));
      const recipe = sceneRecipeOf(id);
      const use = recipe === null ? { layers: 0, parts: 0 } : sceneUse(composeScene(recipe, id));
      if (!radiant) report.push(`${id}\t${card.rarity}\t${card.type}\telements=${artElements(svg)}\tlayers=${use.layers}\tparts=${use.parts}\tbytes=${uri.length}`);
      row.push(
        `<div class="c"><div class="w" style="width:${size}px;height:${size}px;background-image:url(&quot;${uri}&quot;)"><div class="crop"></div></div>` +
          `<div class="l">${id} ${radiant ? "R" : ""} ${card.rarity[0]} ${artElements(svg)}e ${Math.round(uri.length / 1024)}k</div></div>`,
      );
    }
    cells.push(row.join(""));
  }
  // The kit's rules (scene.test.ts "every part draws clean shapes", the palette test), for the named parts alone.
  const GOLD = new Set(["gold-400", "gold-300", "gold-200", "yellow-400", "parchment"]);
  const PATH = /^M[-\d. MLQCAZ]+$/;
  for (const part of parts) {
    const drawer = ((SCENE_KIT as Record<string, unknown>)[part] ?? (FIGURE_KIT as Record<string, unknown>)[part]) as
      | { draw: (at: { x: number; y: number; s: number; flip: 1 | -1 }, rng: () => number) => { d: string; paint: string; opacity: number }[] }
      | undefined;
    if (drawer === undefined) {
      report.push(`CHECK ${part}: no such part`);
      continue;
    }
    for (let seed = 1; seed <= 40; seed += 1) {
      const flip = seed % 2 === 0 ? 1 : -1;
      const shapes = drawer.draw({ x: 50, y: 60, s: 0.5 + (seed % 5) * 0.25, flip }, seededRandom(seed));
      const bad: string[] = [];
      if (shapes.length < 1 || shapes.length > 12) bad.push(`${shapes.length} shapes`);
      for (const shape of shapes) {
        if (!PATH.test(shape.d) || /NaN|Infinity|undefined/.test(shape.d)) bad.push(`bad d ${shape.d.slice(0, 60)}`);
        if (!(shape.paint in SCENE_PALETTE) || GOLD.has(shape.paint)) bad.push(`paint ${shape.paint}`);
        if (!(shape.opacity > 0 && shape.opacity <= 1)) bad.push(`opacity ${shape.opacity}`);
      }
      if (bad.length > 0) {
        report.push(`CHECK ${part} seed ${seed}: ${bad.slice(0, 3).join("; ")}`);
        break;
      }
    }
  }
  for (const part of parts) {
    const kit =(SCENE_KIT as Record<string, { draw: (at: { x: number; y: number; s: number; flip: 1 | -1 }, rng: () => number) => { d: string; paint: keyof typeof SCENE_PALETTE; opacity: number }[] }>)[part];
    const fig = (FIGURE_KIT as Record<string, unknown>)[part];
    const draws: string[] = [];
    for (const [k, place] of [
      { x: 50, y: 60, s: 2.5, flip: 1 as const },
      { x: 50, y: 60, s: 1.2, flip: -1 as const },
    ].entries()) {
      let shapes: { d: string; paint: keyof typeof SCENE_PALETTE; opacity: number }[] = [];
      if (kit !== undefined) shapes = kit.draw(place, seededRandom(k + 3));
      else if (fig !== undefined) {
        // A figure part: drawn on a whole figure that wears it.
        const pose = (part.startsWith("pose-") ? part : "pose-stand") as PoseId;
        const look: FigureLook = {
          head: part.startsWith("head-") ? (part as FigureLook["head"]) : "head-human",
          hair: !part.startsWith("head-") && !part.startsWith("pose-") && !part.startsWith("outfit-") && !part.startsWith("item-") ? (part as FigureLook["hair"]) : "hair-short",
          outfit: part.startsWith("outfit-") ? (part as FigureLook["outfit"]) : "outfit-tunic",
          item: part.startsWith("item-") ? (part as FigureLook["item"]) : null,
          skin: "red-300",
          cloth: "blue-700",
        };
        shapes = drawFigure({ x: 50, y: 92, scale: k === 0 ? 1.5 : 0.8, flip: place.flip, pose, look }, seededRandom(k + 3));
      }
      const body = shapes.map((shape) => `<path d="${shape.d}" fill="${SCENE_PALETTE[shape.paint]}" opacity="${shape.opacity}" fill-rule="evenodd"/>`).join("");
      draws.push(`<div class="c"><svg viewBox="0 0 100 100" width="${size}" height="${size}" style="background:#9fb4c8">${body}</svg><div class="l">${part} ${shapes.length}sh</div></div>`);
    }
    cells.push(draws.join(""));
  }
  const html =
    `<html><head><style>body{background:#1b1f2b;color:#ddd;font:11px sans-serif;margin:6px;display:flex;flex-wrap:wrap;gap:6px}` +
    `.c{display:inline-block}.w{background-size:cover;position:relative}.crop{position:absolute;left:0;right:0;top:18%;bottom:18%;outline:1px dashed rgba(255,255,255,.35)}.l{width:${size}px}</style></head><body>` +
    cells.join("") +
    `</body></html>`;
  writeFileSync(`${OUT}/${name}.html`, html);
  writeFileSync(`${OUT}/${name}.txt`, report.join("\n"));
});
