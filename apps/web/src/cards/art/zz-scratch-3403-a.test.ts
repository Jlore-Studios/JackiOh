import { it } from "vitest";
import { writeFileSync, mkdirSync } from "node:fs";
import { CATALOG } from "@jackioh/cards";
import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { artElements } from "./richness.ts";
import { composeScene, sceneUse } from "./scene.ts";
import { STINKYSCAPE_91_RECIPES_1 as R } from "./stinkyscape91Recipes1.ts"; // your file
const DETAIL = (process.env.DETAIL ?? "").split(",").filter(Boolean);
it("measure", () => {
  const rows: string[] = [];
  mkdirSync("/tmp/s3403a/svg", { recursive: true });
  for (const [id, recipe] of Object.entries(R)) {
    const card = CATALOG[id]!;
    const base = proceduralCardArtUri(card, false), rad = proceduralCardArtUri(card, true);
    const svg = decodeURIComponent(base.slice("data:image/svg+xml,".length));
    writeFileSync(`/tmp/s3403a/svg/${id}.svg`, svg);
    const shapes = composeScene(recipe, id);
    const cap = base.length > 44000 || rad.length > 44000 ? " OVER44K" : "";
    const layers = [...new Set(shapes.map((s) => s.layer))].join("/");
    rows.push([id, card.rarity, "el", artElements(svg), "layers", sceneUse(shapes).layers, layers, "bytes", base.length, rad.length].join(" ") + cap);
    if (DETAIL.includes(id) || DETAIL.includes("all")) {
      const by = new Map<string, [number, number]>();
      let total = 0;
      for (const s of shapes) { const k = `${s.layer}:${s.part}`; const c = encodeURIComponent(s.d).length + 70; total += c; const v = by.get(k) ?? [0, 0]; by.set(k, [v[0] + 1, v[1] + c]); }
      rows.push(`   overhead≈${base.length - total} ` + [...by].map(([k, [n, b]]) => `${k}=${n}/${b}`).join(" "));
    }
  }
  throw new Error("MEASURE\n" + rows.join("\n")); // vitest prints the message; console.log output is swallowed
});
