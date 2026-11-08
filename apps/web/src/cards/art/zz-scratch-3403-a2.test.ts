import { it } from "vitest";
import { SCENE_KIT as SK } from "./sceneKit.ts";
import { FIGURE_KIT } from "./figureKit.ts";
const SCENE_KIT = { ...SK, ...FIGURE_KIT } as Record<string, { draw: (a: never, r: never) => { d: string }[] }>;
import { seededRandom } from "./hash.ts";
const IDS = (process.env.PARTS ?? "").split(",").filter(Boolean);
const YS = (process.env.YS ?? "60").split(",").map(Number);
const SS = (process.env.SS ?? "1").split(",").map(Number);
it("parts", () => {
  const rows: string[] = [];
  for (const id of IDS) for (const y of YS) for (const sc of SS) {
    const part = SCENE_KIT[id as keyof typeof SCENE_KIT];
    let n = 0, len = 0;
    for (let s = 0; s < 4; s += 1) {
      const shapes = part.draw({ x: 50, y, s: sc, flip: 1 } as never, seededRandom(1234 + s) as never);
      n += shapes.length; len += shapes.reduce((a, sh) => a + encodeURIComponent(sh.d).length + 70, 0);
    }
    rows.push(`${id} y=${y} s=${sc} n=${(n / 4).toFixed(1)} bytes=${Math.round(len / 4)}`);
  }
  throw new Error("PARTS\n" + rows.join("\n"));
});
