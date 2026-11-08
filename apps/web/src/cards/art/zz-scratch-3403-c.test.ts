import { it } from "vitest";
import { CATALOG } from "@jackioh/cards";
import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { artElements } from "./richness.ts";
import { composeScene, sceneUse } from "./scene.ts";
import { STINKYSCAPE_91_RECIPES_3 as R } from "./stinkyscape91Recipes3.ts";
it("measure", () => {
  const used = new Set<string>();
  for (const [id, recipe] of Object.entries(R)) {
    const card = CATALOG[id]!;
    const base = proceduralCardArtUri(card, false), rad = proceduralCardArtUri(card, true);
    const svg = decodeURIComponent(base.slice("data:image/svg+xml,".length));
    const shapes = composeScene(recipe, id);
    for (const s of shapes) used.add(s.part);
    const byLayer: Record<string, number> = {};
    for (const s of shapes) byLayer[s.layer] = (byLayer[s.layer] ?? 0) + 1;
    process.stdout.write([id, card.rarity, "el", artElements(svg), "layers", sceneUse(shapes).layers, "bytes", base.length, rad.length, JSON.stringify(byLayer)].join(" ") + "\n");
  }
  const must = "giant-asteroid catwalk warp-gate lamp-tent spice-tower drone-rack deck-floor viewport regolith interceptor corvette escort-disc combat-robot gloved-hand robot-arm ai-eye starship shuttle rocket probe satellite satellite-dish buoy ship-module missile fighter drone console ore-chunk pickaxe mining-drill light-ring energy-beam laser thruster-flame warp-flash credit-chip energy-wall".split(" ");
  const banned = "cyclone head-cloud puppet-hand strings outfit-void head-void rotunda map bow doll floating-island rails anvil".split(" ");
  process.stdout.write(["MISSING", must.filter((p) => !used.has(p)).join(" "), "BANNED", banned.filter((p) => used.has(p)).join(" "), "COUNT", Object.keys(R).length].join(" ") + "\n");
});
it("parts", () => {
  const want = (process.env.ZZ ?? "").split(",").filter(Boolean);
  for (const id of want) {
    const recipe = R[id];
    if (recipe === undefined) continue;
    const shapes = composeScene(recipe, id);
    const by: Record<string, [number, number]> = {};
    for (const s of shapes) {
      const k = `${s.layer}:${s.part}`;
      const v = by[k] ?? [0, 0];
      v[0] += 1;
      v[1] += s.d.length + 60;
      by[k] = v;
    }
    process.stdout.write(id + " " + Object.entries(by).sort((a, b) => b[1][1] - a[1][1]).map(([k, [n, b]]) => `${k}=${n}/${b}`).join(" ") + "\n");
  }
});
import { SCENE_KIT } from "./sceneKit.ts";
import { FIGURE_KIT } from "./figureKit.ts";
it("costs", () => {
  if (process.env.ZZC === undefined) return;
  const rows: string[] = [];
  for (const part of Object.keys(SCENE_KIT)) {
    const shapes = composeScene({ family: "stinkyscape", sky: ["black", "black"], light: { x: 0, y: 0, r: 1, paint: "black", opacity: 0 }, layers: { subject: [{ part: part as never, x: 50, y: 50 }] } }, "zz");
    rows.push(`${part}=${shapes.length}/${shapes.reduce((n, s) => n + s.d.length + 60, 0)}`);
  }
  const look = { head: "head-human", hair: "hair-short", outfit: "outfit-tunic", item: null, skin: "red-300", cloth: "blue-700" } as const;
  for (const part of Object.keys(FIGURE_KIT)) {
    const kind = FIGURE_KIT[part as keyof typeof FIGURE_KIT].category;
    const l = { ...look, [kind]: part } as never;
    const fig = { x: 50, y: 80, scale: 0.8, pose: kind === "pose" ? part : "pose-stand", look: l } as never;
    const shapes = composeScene({ family: "stinkyscape", sky: ["black", "black"], light: { x: 0, y: 0, r: 1, paint: "black", opacity: 0 }, figure: fig, layers: {} }, "zz");
    rows.push(`F:${part}=${shapes.filter((s) => s.part === part).length}/${shapes.filter((s) => s.part === part).reduce((n, s) => n + s.d.length + 60, 0)}`);
  }
  process.stdout.write("COSTS " + rows.join(" ") + "\n");
});
