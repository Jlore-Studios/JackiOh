// SCRATCH (delete before the run ends): draws kit parts into /tmp/kit/<KIT_SHEET>.svg for a look by eye.
// KIT_PARTS=water,stairs KIT_SHEET=places pnpm --dir apps/web exec vitest run src/cards/art/zz-kit.scratch.test.ts
// then: google-chrome --headless --disable-gpu --window-size=1200,1200 --screenshot=/tmp/kit/places.png /tmp/kit/places.svg
import { mkdirSync, writeFileSync } from "node:fs";

import { it } from "vitest";

import { drawFigure, FIGURE_KIT, type FigureLook, type PoseId } from "./figureKit.ts";
import { seededRandom } from "./hash.ts";
import { SCENE_LINES, SCENE_PALETTE, type KitShape } from "./scene.ts";
import { SCENE_KIT } from "./sceneKit.ts";

const CELL = 100;
const COLS = 6;

function paths(shapes: readonly KitShape[]): string {
  const line = SCENE_LINES.classic;
  return shapes
    .map(
      (s) =>
        `<path d="${s.d}" fill="${SCENE_PALETTE[s.paint]}" opacity="${s.opacity}" fill-rule="evenodd"${s.inked === true && line !== null ? ` stroke="${SCENE_PALETTE[line.paint]}" stroke-width="${line.width}" stroke-linejoin="round"` : ""}/>`,
    )
    .join("");
}

it("draws the sheet", () => {
  const ids = (process.env.KIT_PARTS ?? "").split(",").filter((id) => id !== "");
  const sheet = process.env.KIT_SHEET ?? "kit";
  const looks: FigureLook[] = JSON.parse(process.env.KIT_LOOKS ?? "[]") as FigureLook[];
  const poses = (process.env.KIT_POSES ?? "pose-stand").split(",") as PoseId[];
  const cells: string[] = [];
  const scale = Number(process.env.KIT_SCALE ?? "1");
  const y = Number(process.env.KIT_Y ?? "60");
  for (const id of ids) {
    const part = (SCENE_KIT as Record<string, { draw: (at: { x: number; y: number; s: number; flip: 1 | -1 }, rng: () => number) => readonly KitShape[] }>)[id] ?? (FIGURE_KIT as Record<string, { draw: (at: { x: number; y: number; s: number; flip: 1 | -1 }, rng: () => number) => readonly KitShape[] }>)[id];
    if (part === undefined) throw new Error(`no part ${id}`);
    cells.push(`<g>${paths(part.draw({ x: 50, y, s: scale, flip: 1 }, seededRandom(cells.length + 3)))}<text x="2" y="98" font-size="6" fill="#fff">${id}</text></g>`);
  }
  for (const look of looks) {
    for (const pose of poses) {
      cells.push(`<g>${paths(drawFigure({ x: 50, y: 92, scale: 1.2, pose, look }, seededRandom(cells.length + 3)))}<text x="2" y="98" font-size="5" fill="#fff">${look.head} ${look.outfit} ${pose}</text></g>`);
    }
  }
  const rows = Math.ceil(cells.length / COLS);
  const body = cells
    .map((cell, k) => `<g transform="translate(${(k % COLS) * CELL} ${Math.floor(k / COLS) * CELL})"><rect width="${CELL}" height="${CELL}" fill="#5a6478" stroke="#000" stroke-width="0.3"/>${cell}</g>`)
    .join("");
  mkdirSync("/tmp/kit", { recursive: true });
  writeFileSync(`/tmp/kit/${sheet}.svg`, `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${COLS * CELL} ${rows * CELL}" width="${COLS * 200}" height="${rows * 200}">${body}</svg>`);
});
