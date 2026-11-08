// Scratch preview for sceneKitGear.ts (deleted before the work is done).
import { writeFileSync } from "node:fs";
import { it } from "vitest";
import { seededRandom } from "./hash.ts";
import { SCENE_PALETTE } from "./scene.ts";
import { GEAR_KIT } from "./sceneKitGear.ts";

it("previews gear parts", () => {
  const ids = (process.env.GEAR_PARTS ?? Object.keys(GEAR_KIT).join(",")).split(",");
  const cell = 100;
  const cols = 4;
  let body = "";
  ids.forEach((id, row) => {
    const part = GEAR_KIT[id as keyof typeof GEAR_KIT];
    for (let k = 0; k < cols; k += 1) {
      const x0 = k * cell;
      const y0 = row * cell;
      const flip = k === 3 ? -1 : 1;
      const s = Number(process.env.GEAR_SCALE ?? "2");
      const shapes = part.draw({ x: x0 + 50, y: y0 + 50, s, flip }, seededRandom(k * 7 + 3));
      body += `<rect x="${x0}" y="${y0}" width="${cell}" height="${cell}" fill="#1a2238" stroke="#333"/>`;
      for (const sh of shapes) {
        const stroke = sh.inked === true ? ` stroke="#000" stroke-width="0.35"` : "";
        body += `<path d="${sh.d}" fill="${SCENE_PALETTE[sh.paint]}" fill-opacity="${sh.opacity}" fill-rule="evenodd"${stroke}/>`;
      }
      body += `<circle cx="${x0 + 50}" cy="${y0 + 50}" r="0.8" fill="#f0f"/>`;
    }
  });
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${cols * cell} ${ids.length * cell}" width="${cols * 200}" height="${ids.length * 200}">${body}</svg>`;
  writeFileSync("/tmp/gear-preview.svg", svg);
});
