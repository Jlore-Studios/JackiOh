import { it } from "vitest";
import { writeFileSync } from "node:fs";
import { CATALOG } from "@jackioh/cards";
import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { artElements } from "./richness.ts";
import { composeScene, sceneUse, SCENE_LAYERS } from "./scene.ts";
import { SCENE_KIT } from "./sceneKit.ts";
import { seededRandom } from "./hash.ts";
import { STINKYSCAPE_91_RECIPES_1 as R } from "./stinkyscape91Recipes1.ts"; // your file
it("measure", () => {
  const rows: string[] = [];
  for (const [id, part] of Object.entries(SCENE_KIT)) {
    const shapes = part.draw({ x: 50, y: 50, s: 1, flip: 1 }, seededRandom(7));
    rows.push(`${id} ${shapes.length} ${shapes.reduce((n, s) => n + s.d.length, 0)} main=${[...new Set(shapes.filter((s) => s.main).map((s) => s.paint))].join(",")} all=${[...new Set(shapes.map((s) => s.paint))].join(",")}`);
  }
  writeFileSync("/tmp/scratch3403a/parts.txt", rows.join("\n") + "\n");
  const out: string[] = [];
  for (const [id, recipe] of Object.entries(R)) {
    const card = CATALOG[id]!;
    const base = proceduralCardArtUri(card, false), rad = proceduralCardArtUri(card, true);
    const svg = decodeURIComponent(base.slice("data:image/svg+xml,".length));
    const shapes = composeScene(recipe, id);
    writeFileSync(`/tmp/scratch3403a/svg/${id}.svg`, svg);
    const per = SCENE_LAYERS.map((l) => `${l[0]}${shapes.filter((s) => s.layer === l).length}`).join(" ");
    const cost = new Map<string, number>();
    for (const sh of shapes) cost.set(sh.part, (cost.get(sh.part) ?? 0) + encodeURIComponent(sh.d).length + 100);
    const top = [...cost.entries()].sort((a, b) => b[1] - a[1]).slice(0, 6).map(([p, n]) => `${p}:${(n / 1000).toFixed(1)}k`).join(" ");
    out.push(`${id} ${card.rarity} el ${artElements(svg)} layers ${sceneUse(shapes).layers} bytes ${base.length} ${rad.length} [${per}] ${top}`);
  }
  writeFileSync("/tmp/scratch3403a/measure.txt", out.join("\n") + "\n");
});
