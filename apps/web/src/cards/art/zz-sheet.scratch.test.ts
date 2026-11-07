// SCRATCH (delete before the run ends): writes Classic+ pictures to /tmp/cp for a look by eye, and a TSV of measures.
// CP_IDS=classicplus-001,classicplus-002 CP_SHEET=part1 pnpm --dir apps/web exec vitest run src/cards/art/zz-sheet.scratch.test.ts
// then: google-chrome --headless --disable-gpu --window-size=1400,<height> --screenshot=/tmp/cp/part1.png file:///tmp/cp/part1.html
import { mkdirSync, writeFileSync } from "node:fs";

import { it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { ART_DETAIL_MIN, artElements } from "./richness.ts";
import { composeScene, sceneRecipeOf, sceneUse } from "./scene.ts";

it("writes the sheet", () => {
  const wanted = (process.env.CP_IDS ?? "").split(",").filter((id) => id !== "");
  const sheet = process.env.CP_SHEET ?? "index";
  const width = Number(process.env.CP_PX ?? "150");
  const cards = Object.values(CATALOG)
    .filter((card) => (wanted.length === 0 ? card.set === "Classic+" : wanted.includes(card.id)))
    .sort((a, b) => (a.id < b.id ? -1 : 1));
  mkdirSync("/tmp/cp", { recursive: true });
  const rows: string[] = ["id\tname\trarity\ttype\telements\tfloor\tlayers\tparts\tbaseKiB\tradiantKiB"];
  const cells: string[] = [];
  for (const card of cards) {
    const base = proceduralCardArtUri(card, false);
    const radiant = proceduralCardArtUri(card, true);
    const svg = decodeURIComponent(base.slice("data:image/svg+xml,".length));
    writeFileSync(`/tmp/cp/${card.id}-base.svg`, svg);
    writeFileSync(`/tmp/cp/${card.id}-radiant.svg`, decodeURIComponent(radiant.slice("data:image/svg+xml,".length)));
    const scene = sceneRecipeOf(card.id);
    const use = scene === null ? { layers: 0, parts: 0 } : sceneUse(composeScene(scene, card.id));
    const floor = ART_DETAIL_MIN[card.rarity];
    rows.push([card.id, card.name, card.rarity, card.type, artElements(svg), floor.elements, use.layers, use.parts, (base.length / 1024).toFixed(1), (radiant.length / 1024).toFixed(1)].join("\t"));
    cells.push(
      `<figure><div class="w"><img src="${card.id}-base.svg" width="${width}"><i></i></div><div class="w"><img src="${card.id}-radiant.svg" width="${width}"><i></i></div><figcaption>${card.id.replace("classicplus-", "")} ${card.name} (${card.rarity})</figcaption></figure>`,
    );
  }
  writeFileSync(
    `/tmp/cp/${sheet}.html`,
    `<html><body style="background:#1b1f2b;color:#ddd;font:11px sans-serif;margin:4px"><style>figure{display:inline-block;margin:3px;width:${width * 2 + 8}px}.w{display:inline-block;position:relative;margin-right:2px}.w i{position:absolute;left:0;right:0;top:${width * 0.18}px;height:${width * 0.64}px;outline:1px dashed rgba(255,255,255,.35)}figcaption{white-space:nowrap;overflow:hidden}</style>${cells.join("")}</body></html>`,
  );
  writeFileSync(`/tmp/cp/${sheet}.tsv`, `${rows.join("\n")}\n`);
  console.log(rows.join("\n"));
});
