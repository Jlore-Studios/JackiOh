// TEMPORARY (#3391 build tooling, deleted before delivery): the verdict table's measures.
import { writeFileSync } from "node:fs";
import { it } from "vitest";
import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";
import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { artElements } from "./richness.ts";
import { composeScene, SCENE_LAYERS } from "./scene.ts";
import { SCENE_KIT, type SceneryPartId } from "./sceneKit.ts";
import { STINKYSCAPE_RECIPES } from "./stinkyscapeRecipes.ts";

it("table", () => {
  const rows: string[] = [];
  for (const [id, recipe] of Object.entries(STINKYSCAPE_RECIPES).sort()) {
    const card = CATALOG[id] as CardDef;
    const base = proceduralCardArtUri(card, false);
    const rad = proceduralCardArtUri(card, true);
    const el = artElements(decodeURIComponent(base.slice("data:image/svg+xml,".length)));
    const craft = (recipe.layers.subject ?? []).filter((p) => SCENE_KIT[p.part as SceneryPartId].category === "craft" || SCENE_KIT[p.part as SceneryPartId].category === "creatures").map((p) => p.part);
    const f = recipe.figure;
    const subject = f !== undefined ? `figure: ${f.look.head}, ${f.look.hair ?? "-"}, ${f.look.outfit} (${f.look.cloth}), ${f.look.item ?? "-"}, ${f.pose}${(recipe.extras ?? []).length > 0 ? ` + ${(recipe.extras ?? []).length} more` : ""}` : craft.length > 0 ? [...new Set(craft)].join(", ") : (recipe.layers.subject ?? []).map((p) => p.part).filter((v, i, a) => a.indexOf(v) === i).join(", ");
    const arms = composeScene(recipe, id).filter((s) => s.part === "drone").length;
    rows.push(`| \`${id}\` | ${card.name} | ${card.rarity} | ${subject} | ${el} | ${Math.max(base.length, rad.length)} |${id === "stinkyscape-041" ? ` drone shapes ${arms}` : ""}`);
  }
  writeFileSync("/tmp/table.md", rows.join("\n"));
  void SCENE_LAYERS;
});
