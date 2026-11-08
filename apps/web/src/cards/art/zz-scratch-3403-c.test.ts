import { it } from "vitest";
import { mkdirSync, writeFileSync } from "node:fs";
import { CATALOG } from "@jackioh/cards";
import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { artElements } from "./richness.ts";
import { composeScene, sceneUse } from "./scene.ts";
import { STINKYSCAPE_91_RECIPES_3 as R } from "./stinkyscape91Recipes3.ts";
const DETAIL = (process.env.DETAIL ?? "").split(",").filter((s) => s !== "");
it("measure", () => {
  const rows: string[] = [];
  for (const [id, recipe] of Object.entries(R)) {
    const card = CATALOG[id]!;
    const base = proceduralCardArtUri(card, false), rad = proceduralCardArtUri(card, true);
    const svg = decodeURIComponent(base.slice("data:image/svg+xml,".length));
    mkdirSync("/tmp/svg3403c", { recursive: true });
    writeFileSync(`/tmp/svg3403c/${id}.svg`, svg);
    const shapes = composeScene(recipe, id);
    const per = new Map<string, number>();
    for (const s of shapes) per.set(s.layer, (per.get(s.layer) ?? 0) + 1);
    const flag = card.rarity === "Token" || card.rarity === "Common" ? 30 : card.rarity === "Rare" ? 40 : card.rarity === "Epic" ? 60 : 90;
    const el = artElements(svg);
    const max = Math.max(base.length, rad.length);
    rows.push([id, card.rarity, "el", el, el < flag ? "LOW" : "", "layers", sceneUse(shapes).layers, "bytes", max, max > 44000 ? "BIG" : "", JSON.stringify(Object.fromEntries(per))].join(" "));
    if (DETAIL.includes(id)) {
      const parts = new Map<string, [number, number]>();
      for (const s of shapes) {
        const key = `${s.layer}:${s.part}`;
        const [n, b] = parts.get(key) ?? [0, 0];
        parts.set(key, [n + 1, b + s.d.length + 60]);
      }
      for (const [k, [n, b]] of parts) rows.push(`   ${k} n=${n} ~bytes=${b}`);
    }
  }
  throw new Error("MEASURE\n" + rows.join("\n"));
});
