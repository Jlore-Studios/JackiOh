import { it } from "vitest";
import { CATALOG } from "@jackioh/cards";
import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { artElements } from "./richness.ts";
import { composeScene, sceneUse } from "./scene.ts";
import { GEAR_KIT } from "./sceneKitGear.ts";
import { seededRandom } from "./hash.ts";
import { STINKYSCAPE_91_RECIPES_2 as R } from "./stinkyscape91Recipes2.ts";
const placeholder = new Set(Object.entries(GEAR_KIT).filter(([, p]) => p.draw({ x: 50, y: 50, s: 1, flip: 1 }, seededRandom(7)).length === 1).map(([id]) => id));
it("measure", () => {
  const rows: string[] = [`placeholders: ${[...placeholder].join(",")}`];
  const only = process.env.ONLY ?? "";
  for (const [id, recipe] of Object.entries(R)) {
    const card = CATALOG[id]!;
    const base = proceduralCardArtUri(card, false), rad = proceduralCardArtUri(card, true);
    const svg = decodeURIComponent(base.slice("data:image/svg+xml,".length));
    const shapes = composeScene(recipe, id);
    const ph = shapes.filter((s) => placeholder.has(s.part)).length;
    rows.push([id, card.rarity, "el", artElements(svg), "layers", sceneUse(shapes).layers, "bytes", base.length, rad.length, "proj", Math.max(base.length, rad.length) + 2200 * ph, "projEl", artElements(svg) + 7 * ph].join(" "));
    if (only !== "" && (only === "all" || only.split(",").includes(id))) {
      const per = new Map<string, [number, number]>();
      for (const s of shapes) { const k = `${s.layer}:${s.part}`; const v = per.get(k) ?? [0, 0]; v[0] += 1; v[1] += encodeURIComponent(s.d).length + 137; per.set(k, v); }
      rows.push("   " + [...per.entries()].sort((a, b) => b[1][1] - a[1][1]).map(([k, [n, b]]) => `${k}=${n}/${b}`).join(" "));
    }
  }
  throw new Error("MEASURE\n" + rows.join("\n"));
});
