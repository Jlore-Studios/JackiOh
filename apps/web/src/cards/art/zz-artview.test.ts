// SCRATCH (#3393 build): renders cards' faces and kit parts to PNG for a look. Delete before delivery.
import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";

import { it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { FIGURE_KIT, FIGURE_DEFAULT_LOOK, drawFigure, type PoseId } from "./figureKit.ts";
import { seededRandom } from "./hash.ts";
import { SCENE_PALETTE, type KitShape } from "./scene.ts";
import { SCENE_KIT, type SceneryPartId } from "./sceneKit.ts";
import { artElements } from "./richness.ts";

const OUT = process.env.ARTVIEW_OUT ?? "/tmp/artview/out";
const IDS = (process.env.ARTVIEW_IDS ?? "").split(",").filter(Boolean);
const PARTS = (process.env.ARTVIEW_PARTS ?? "").split(",").filter(Boolean);
const CELL = Number(process.env.ARTVIEW_CELL ?? "260");

function shot(html: string, name: string, cols: number, rows: number, cell: number): void {
  const file = `${OUT}/${name}.html`;
  writeFileSync(file, html);
  const w = cols * (cell + 8) + 16;
  const h = rows * (cell + 30) + 24;
  execFileSync("google-chrome", ["--headless=new", "--no-sandbox", "--disable-gpu", "--hide-scrollbars", `--window-size=${w},${h}`, `--screenshot=${OUT}/${name}.png`, `file://${file}`], { stdio: "ignore" });
}

function partSvg(shapes: readonly KitShape[]): string {
  const body = shapes.map((s) => `<path d="${s.d}" fill="${SCENE_PALETTE[s.paint]}" opacity="${s.opacity}" fill-rule="evenodd"${s.inked === true ? ' stroke="#120b07" stroke-width="0.4"' : ""}/>`).join("");
  return `<svg viewBox="0 0 100 100" width="${CELL}" height="${CELL}" xmlns="http://www.w3.org/2000/svg"><rect width="100" height="100" fill="#cfd8e3"/><line x1="0" y1="70" x2="100" y2="70" stroke="#999" stroke-width="0.2"/>${body}</svg>`;
}

it("artview", () => {
  mkdirSync(OUT, { recursive: true });
  if (IDS.length > 0) {
    const cells: string[] = [];
    for (const id of IDS) {
      const card = CATALOG[id];
      if (card === undefined) throw new Error(id);
      for (const radiant of [false, true]) {
        const uri = proceduralCardArtUri(card, radiant);
        const svg = decodeURIComponent(uri.slice("data:image/svg+xml,".length));
        const n = artElements(svg);
        cells.push(`<div class="c"><div class="w"><img src="${uri}" width="${CELL}" height="${CELL}"/><div class="band"></div></div><p>${id}${radiant ? " R" : ""} · ${card.rarity} · ${n} el · ${Math.round(uri.length / 1024)}KB</p></div>`);
      }
    }
    const cols = Number(process.env.ARTVIEW_COLS ?? "4");
    const html = `<html><body style="margin:8px;background:#222;font:11px sans-serif;color:#eee"><style>.c{margin:4px}.w{position:relative;width:${CELL}px;height:${CELL}px}.band{position:absolute;left:0;right:0;top:${CELL * 0.18}px;height:${CELL * 0.64}px;outline:1px dashed rgba(255,0,0,.6)}p{margin:2px 0}</style><div style="display:flex;flex-wrap:wrap;width:${cols * (CELL + 8)}px">${cells.join("")}</div></body></html>`;
    shot(html, process.env.ARTVIEW_NAME ?? "cards", cols, Math.ceil(cells.length / cols), CELL);
  }
  if (PARTS.length > 0) {
    const cells: string[] = [];
    for (const id of PARTS) {
      const scale = Number(process.env.ARTVIEW_SCALE ?? "2");
      let shapes: readonly KitShape[];
      if (Object.hasOwn(SCENE_KIT, id)) shapes = SCENE_KIT[id as SceneryPartId].draw({ x: 50, y: 70, s: scale, flip: 1 }, seededRandom(7));
      else if (Object.hasOwn(FIGURE_KIT, id) && id.startsWith("pose-")) shapes = drawFigure({ x: 50, y: 92, scale: 1.3, pose: id as PoseId, look: FIGURE_DEFAULT_LOOK }, seededRandom(7));
      else if (Object.hasOwn(FIGURE_KIT, id)) shapes = FIGURE_KIT[id as keyof typeof FIGURE_KIT].draw({ x: 50, y: 50, s: 3, flip: 1 }, seededRandom(7));
      else throw new Error(`no part ${id}`);
      cells.push(`<div class="c">${partSvg(shapes)}<p>${id} (${shapes.length})</p></div>`);
    }
    const cols = Number(process.env.ARTVIEW_COLS ?? "6");
    const html = `<html><body style="margin:8px;background:#222;font:11px sans-serif;color:#eee"><style>.c{margin:4px}p{margin:2px 0}</style><div style="display:flex;flex-wrap:wrap;width:${cols * (CELL + 8)}px">${cells.join("")}</div></body></html>`;
    shot(html, process.env.ARTVIEW_NAME ?? "parts", cols, Math.ceil(cells.length / cols), CELL);
  }
}, 600_000);
