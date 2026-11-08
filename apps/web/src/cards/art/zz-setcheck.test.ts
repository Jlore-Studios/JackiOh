// SCRATCH (#3393 build): per-card checks of setScenes.test.ts for a subset, reported as a table. Delete before delivery.
// SETCHECK=mp|tg|legacy|all (or SETCHECK_IDS=a,b) pnpm exec vitest run src/cards/art/zz-setcheck.test.ts
import { it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { FIGURE_DEFAULT_LOOK } from "./figureKit.ts";
import { ART_DETAIL_MAX_BYTES, ART_DETAIL_MIN, artElements } from "./richness.ts";
import { composeScene, SCENE_LAYERS, sceneRecipeOf, sceneUse } from "./scene.ts";
import { snapshotArtOf } from "./traininggrounds.ts";

const which = process.env.SETCHECK ?? "all";
const ids = (process.env.SETCHECK_IDS ?? "").split(",").filter(Boolean);
const pick = (card: CardDef): boolean => {
  if (ids.length > 0) return ids.includes(card.id);
  const legacy = snapshotArtOf(card.id) !== null;
  if (which === "mp") return card.set === "Meditative+";
  if (which === "tg") return card.set === "Training Grounds" && !legacy;
  if (which === "legacy") return legacy;
  return card.set === "Meditative+" || card.set === "Training Grounds";
};

it("setcheck", () => {
  const rows: string[] = [];
  const subjects = new Map<string, string>();
  let bad = 0;
  for (const card of Object.values(CATALOG).filter(pick)) {
    const problems: string[] = [];
    const scene = sceneRecipeOf(card.id);
    if (scene === null) {
      rows.push(`${card.id} | NO RECIPE`);
      bad += 1;
      continue;
    }
    let shapes;
    try {
      shapes = composeScene(scene, card.id);
    } catch (error) {
      rows.push(`${card.id} | THROWS ${String(error)}`);
      bad += 1;
      continue;
    }
    const base = decodeURIComponent(proceduralCardArtUri(card, false).slice("data:image/svg+xml,".length));
    const radiantUri = proceduralCardArtUri(card, true);
    const baseUri = proceduralCardArtUri(card, false);
    const el = artElements(base);
    const use = sceneUse(shapes);
    const floor = ART_DETAIL_MIN[card.rarity];
    if (el < floor.elements) problems.push(`elements ${el}<${floor.elements}`);
    if (use.layers < floor.layers) problems.push(`layers ${use.layers}<${floor.layers}`);
    const layers = new Set(shapes.map((s) => s.layer));
    if (!layers.has("subject")) problems.push("no subject layer");
    if (["Epic", "Legendary", "Mythic"].includes(card.rarity)) for (const l of ["far", "mid", "near"] as const) if (!layers.has(l)) problems.push(`no ${l}`);
    if (baseUri.length > ART_DETAIL_MAX_BYTES || radiantUri.length > ART_DETAIL_MAX_BYTES) problems.push("too big");
    if (/NaN|undefined|Infinity/.test(base)) problems.push("unclean");
    const placements = SCENE_LAYERS.flatMap((layer) => (scene.layers[layer] ?? []).map((p) => ({ layer, part: p.part, count: p.count ?? 1 })));
    const seals = placements.filter((p) => p.part === "seal");
    if (card.set === "Meditative+") {
      if (scene.family !== "meditative") problems.push("family");
      if (seals.length !== 1 || seals[0]?.layer !== "near" || seals[0]?.count !== 1) problems.push("seal");
      if (!placements.some((p) => p.part === "peaks")) problems.push("no peaks");
    } else {
      if (scene.family !== "training-grounds") problems.push("family");
      if (seals.length > 0) problems.push("seal");
    }
    if (card.type === "Unit") {
      if (scene.figure === undefined) problems.push("unit without figure");
      else if (JSON.stringify(scene.figure.look) === JSON.stringify(FIGURE_DEFAULT_LOOK)) problems.push("default look");
    }
    if (scene.ownSubject !== undefined) problems.push("ownSubject");
    const key = JSON.stringify([(scene.layers.subject ?? []).map((p) => p.part).sort(), scene.figure === undefined ? null : [scene.figure.pose, scene.figure.look], (scene.extras ?? []).map((e) => [e.pose, e.look])]);
    const twin = subjects.get(key);
    if (twin !== undefined) problems.push(`same subject as ${twin}`);
    subjects.set(key, card.id);
    if (problems.length > 0) bad += 1;
    rows.push(`${card.id} | ${card.rarity} | ${card.type} | el ${el} | layers ${use.layers} | parts ${use.parts} | ${Math.round(baseUri.length / 1024)}/${Math.round(radiantUri.length / 1024)} KB | ${problems.length === 0 ? "ok" : problems.join(", ")}`);
  }
  console.log(`\n${rows.join("\n")}\nBAD: ${bad}\n`);
}, 600_000);
