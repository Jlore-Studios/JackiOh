import { writeFileSync } from "node:fs";
import { it } from "vitest";
import { CATALOG } from "@jackioh/cards";
import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { artElements } from "./richness.ts";
import { composeScene, sceneUse } from "./scene.ts";
import { SCENE_KIT } from "./sceneKit.ts";
import { STINKYSCAPE_91_RECIPES_2 as R } from "./stinkyscape91Recipes2.ts";

it("measure", () => {
  if (process.env.PARTS === "1") {
    const rows: string[] = [];
    for (const [id, part] of Object.entries(SCENE_KIT)) {
      let seed = 1;
      const rng = (): number => {
        seed = (seed * 16807) % 2147483647;
        return seed / 2147483647;
      };
      const shapes = part.draw({ x: 50, y: 50, s: 1, flip: 1 }, rng);
      const bytes = shapes.reduce((n, s) => n + s.d.length, 0);
      rows.push(`${id}:${shapes.length}/${bytes}`);
    }
    writeFileSync("/tmp/scratch-b-parts.txt", rows.join("\n"));
  }
  const out: string[] = [];
  for (const [id, recipe] of Object.entries(R)) {
    const card = CATALOG[id]!;
    const base = proceduralCardArtUri(card, false),
      rad = proceduralCardArtUri(card, true);
    const svg = decodeURIComponent(base.slice("data:image/svg+xml,".length));
    const shapes = composeScene(recipe, id);
    const per = new Map<string, number>();
    for (const s of shapes) per.set(s.layer, (per.get(s.layer) ?? 0) + 1);
    out.push(`${id} ${card.rarity} el ${artElements(svg)} layers ${sceneUse(shapes).layers} bytes ${Math.max(base.length, rad.length)} ${[...per].map(([k, v]) => `${k}=${v}`).join(",")}`);
  }
  writeFileSync("/tmp/scratch-b.txt", out.join("\n") + "\n");
});
