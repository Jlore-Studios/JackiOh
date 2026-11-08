// TEMPORARY (#3397 build): prints each alt recipe's metrics and writes its normal and alt SVGs for a pixel check.
import { mkdirSync, writeFileSync } from "node:fs";
import { it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { MEDITATIVE_PLUS_ALT_RECIPES } from "./meditativePlusAltRecipes.ts";
import { motifFor } from "./motifs.ts";
import { artSeedOf } from "./pictures.ts";
import { ART_DETAIL_MIN, artElements } from "./richness.ts";
import { composeScene, sceneUse } from "./scene.ts";
import { SCENE_RECIPES } from "./sceneRecipes.ts";
import { proceduralArtUri, trainingGroundsArtUri } from "./svg.ts";
import { compositionOf, themeFor } from "./themes.ts";
import { isTrainingGroundsArt } from "./traininggrounds.ts";
import { TRAINING_GROUNDS_ALT_RECIPES } from "./trainingGroundsAltRecipes.ts";

const ALT = { ...MEDITATIVE_PLUS_ALT_RECIPES, ...TRAINING_GROUNDS_ALT_RECIPES };

function uriOf(card: CardDef, seedId: string, radiant: boolean): string {
  const theme = themeFor(card.tags, card.type);
  if (isTrainingGroundsArt(card.id)) return trainingGroundsArtUri(card.id, card.name, card.tags, card.type, radiant, null, seedId);
  return proceduralArtUri(seedId, theme, compositionOf(card.id, card.type), radiant, motifFor(card.name, theme), null);
}

it("measure", () => {
  mkdirSync("/tmp/altwork/svg", { recursive: true });
  const rows: string[] = [];
  for (const [id, scene] of Object.entries(ALT)) {
    const card = CATALOG[id] as CardDef;
    const seed = artSeedOf(id, "alt");
    const svg = decodeURIComponent(uriOf(card, seed, false).slice("data:image/svg+xml,".length));
    const floor = ART_DETAIL_MIN[card.rarity];
    const els = artElements(svg);
    const layers = sceneUse(composeScene(scene, seed)).layers;
    const bytes = Math.max(uriOf(card, seed, false).length, uriOf(card, seed, true).length);
    const was = SCENE_RECIPES[id];
    const skyShared = scene.sky.filter((name) => was?.sky.includes(name));
    rows.push(`${id} ${card.type} ${card.rarity} els=${els}/${floor.elements}${els < floor.elements ? " LOW" : ""} layers=${layers}/${floor.layers}${layers < floor.layers ? " LOW" : ""} bytes=${bytes}${bytes > 48 * 1024 ? " BIG" : ""}${skyShared.length > 0 ? ` SKY ${skyShared.join(",")}` : ""}`);
    for (const radiant of [false, true]) {
      for (const [name, s] of [["normal", id], ["alt", seed]] as const) {
        writeFileSync(`/tmp/altwork/svg/${id}-${name}-${radiant ? "radiant" : "base"}.svg`, decodeURIComponent(uriOf(card, s, radiant).slice("data:image/svg+xml,".length)));
      }
    }
  }
  writeFileSync("/tmp/altwork/metrics.txt", rows.join("\n") + "\n");
});
