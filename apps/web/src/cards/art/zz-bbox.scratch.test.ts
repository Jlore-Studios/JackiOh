// SCRATCH (delete before the run ends): writes /tmp/kit/bbox.html, every scene part drawn at (50, 50) scale 1, so
// `google-chrome --headless --dump-dom` prints each part's bounding box relative to its anchor.
import { mkdirSync, writeFileSync } from "node:fs";

import { it } from "vitest";

import { seededRandom } from "./hash.ts";
import { SCENE_KIT } from "./sceneKit.ts";

it("writes the bbox page", () => {
  const groups = Object.entries(SCENE_KIT)
    .map(([id, part], k) => `<g id="${id}" data-cat="${part.category}">${part.draw({ x: 50, y: 50, s: 1, flip: 1 }, seededRandom(k + 7)).map((s) => `<path d="${s.d}"/>`).join("")}</g>`)
    .join("");
  const script = `<script>const out=[];for(const g of document.querySelectorAll('g')){const b=g.getBBox();out.push(g.id+'\\t'+g.dataset.cat+'\\tx '+(b.x-50).toFixed(0)+'..'+(b.x+b.width-50).toFixed(0)+'\\ty '+(b.y-50).toFixed(0)+'..'+(b.y+b.height-50).toFixed(0)+'\\tshapes '+g.children.length);}document.getElementById('o').textContent=out.join('\\n');</script>`;
  mkdirSync("/tmp/kit", { recursive: true });
  writeFileSync("/tmp/kit/bbox.html", `<html><body><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">${groups}</svg><pre id="o"></pre>${script}</body></html>`);
});
