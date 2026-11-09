import { writeFileSync } from "node:fs";

import { it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { offeredOn, VERSION_ALT_ART_ID } from "../versions.ts";
import { FULL_ART_CARD_MOTION } from "./motion.ts";
import { SCENE_LAYERS } from "./scene.ts";
import { STINKYSCAPE_91_RECIPES } from "./sceneRecipes.ts";

it("dump", () => {
  const late = Object.values(CATALOG)
    .filter((c) => c.set === "Stinkyscape" && offeredOn(VERSION_ALT_ART_ID, c.id))
    .map((c) => c.id)
    .sort();
  const out: string[] = [`offered: ${late.join(" ")}`];
  for (const id of late) {
    const c = CATALOG[id];
    if (c === undefined) continue;
    const r = STINKYSCAPE_91_RECIPES[id];
    out.push(`\n## ${id} ${c.name} | ${c.rarity} ${c.type} | tags ${c.tags.join(",")} | motion ${FULL_ART_CARD_MOTION[id] === undefined ? "-" : JSON.stringify(FULL_ART_CARD_MOTION[id])}`);
    if (r === undefined) {
      out.push("  NO NORMAL RECIPE");
      continue;
    }
    out.push(`  sky ${r.sky.join(",")} light ${JSON.stringify(r.light)} emblem ${JSON.stringify(r.emblem)}`);
    if (r.figure) out.push(`  figure x${r.figure.x} y${r.figure.y} s${r.figure.scale} ${r.figure.pose} look ${JSON.stringify(r.figure.look)}`);
    for (const [k, e] of (r.extras ?? []).entries()) out.push(`  extra${k} ${e.layer} x${e.x} y${e.y} s${e.scale} ${e.pose} look ${JSON.stringify(e.look)}`);
    for (const layer of SCENE_LAYERS) {
      const ps = r.layers[layer];
      if (ps === undefined) continue;
      out.push(`  ${layer}: ${ps.map((p) => `${p.part}@${p.x},${p.y}${p.scale === undefined ? "" : "s" + p.scale}${p.count === undefined ? "" : "x" + p.count}${p.paint === undefined ? "" : ":" + p.paint}`).join(" ")}`);
    }
  }
  writeFileSync("/tmp/stinky-dump.txt", out.join("\n"));
});
