// TEMPORARY (#3391 build tooling, deleted before delivery): measures the pass's pictures and writes contact sheets.
import { writeFileSync } from "node:fs";
import { it } from "vitest";
import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";
import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { artElements, ART_DETAIL_MIN } from "./richness.ts";
import { composeScene, sceneRecipeOf } from "./scene.ts";
import { STINKYSCAPE_RECIPES } from "./stinkyscapeRecipes.ts";
import { pictureSheet } from "./zz-preview.ts";

const ONLY = (process.env.ZZ_ONLY ?? "").split(",").filter((s) => s !== "");

it("measure", () => {
  const ids = Object.keys(STINKYSCAPE_RECIPES).sort().filter((id) => ONLY.length === 0 || ONLY.some((o) => id.endsWith(o)));
  const lines: string[] = [];
  const cells: { label: string; svg: string }[] = [];
  for (const id of ids) {
    const card = CATALOG[id] as CardDef;
    const base = proceduralCardArtUri(card, false);
    const rad = proceduralCardArtUri(card, true);
    const svg = decodeURIComponent(base.slice("data:image/svg+xml,".length));
    const scene = sceneRecipeOf(id);
    const shapes = scene === null ? [] : composeScene(scene, id);
    const el = artElements(svg);
    const floor = ART_DETAIL_MIN[card.rarity].elements;
    const layers = new Set(shapes.map((s) => s.layer)).size;
    lines.push(`${id} ${card.rarity.padEnd(9)} el=${el} floor=${floor} ${el < floor ? "UNDER" : ""} layers=${layers} base=${base.length} rad=${rad.length} ${card.name}`);
    cells.push({ label: `${id.slice(12)} ${card.name} ${el}`, svg });
    if (process.env.ZZ_RADIANT === "1") cells.push({ label: `${id.slice(12)} R`, svg: decodeURIComponent(rad.slice("data:image/svg+xml,".length)) });
  }
  writeFileSync("/tmp/measure.txt", lines.join("\n"));
  for (let k = 0; k * 16 < cells.length; k += 1) pictureSheet(cells.slice(k * 16, k * 16 + 16), `/tmp/rec/sheet-${k}.svg`, 4);
});
