import { expect, it } from "vitest";
import { CATALOG } from "@jackioh/cards";
import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { RADIANT_GOLD_HUE_MAX_DEG, RADIANT_GOLD_HUE_MIN_DEG, RADIANT_GOLD_LIGHTNESS_MAX, RADIANT_GOLD_LIGHTNESS_MIN, RADIANT_GOLD_SATURATION_MIN } from "../constants.ts";
import { figureLookFor } from "./figureKit.ts";
import { artSeedOf } from "./pictures.ts";
import { artSpec } from "./procedural.ts";
import { ART_DETAIL_MIN, ART_DETAIL_SHORT, artElements } from "./richness.ts";
import { composeScene, sceneRecipeOf, sceneUse, SCENE_LAYERS } from "./scene.ts";
import { STINKYSCAPE_91_RECIPES } from "./sceneRecipes.ts";
import { compositionFor, themeFor } from "./themes.ts";
import { STINKYSCAPE_91_RECIPES_2 as R } from "./stinkyscape91Recipes2.ts";
function hsl(hex: string) { const d0 = hex.replace("#", ""); const [r, g, b] = [0, 2, 4].map((a) => parseInt(d0.slice(a, a + 2), 16) / 255) as [number, number, number]; const max = Math.max(r, g, b), min = Math.min(r, g, b); const l = (max + min) / 2; const d = max - min; if (d === 0) return { h: 0, s: 0, l }; const s = d / (1 - Math.abs(2 * l - 1)); const sec = max === r ? ((g - b) / d + 6) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4; return { h: sec * 60, s, l }; }
const gold = (hex: string) => { const { h, s, l } = hsl(hex); return h >= RADIANT_GOLD_HUE_MIN_DEG && h <= RADIANT_GOLD_HUE_MAX_DEG && s >= RADIANT_GOLD_SATURATION_MIN && l >= RADIANT_GOLD_LIGHTNESS_MIN && l <= RADIANT_GOLD_LIGHTNESS_MAX; };
const MACHINES: Record<string, string> = { "stinkyscape-135-1": "kicking-sack", "stinkyscape-150-1": "scrap-golem" };
it("checks", () => {
  const errs: string[] = [];
  const seen = new Map<string, string>();
  for (const [id, sc] of Object.entries(STINKYSCAPE_91_RECIPES)) seen.set(JSON.stringify(sc), id);
  const looks = new Map<string, string>();
  for (const [id, scene] of Object.entries(R)) {
    const card = CATALOG[id]!;
    if (sceneRecipeOf(id) !== scene) errs.push(`${id} not wired`);
    if (seen.get(JSON.stringify(scene)) !== id) errs.push(`${id} dup`);
    const shapes = composeScene(scene, id);
    if (!shapes.some((s) => s.inked === true)) errs.push(`${id} no ink`);
    const floor = ART_DETAIL_MIN[card.rarity];
    const svg = decodeURIComponent(proceduralCardArtUri(card, false).slice("data:image/svg+xml,".length));
    if (artElements(svg) < floor.elements) errs.push(`${id} elements ${artElements(svg)}`);
    if (sceneUse(shapes).layers < floor.layers) errs.push(`${id} layers`);
    const layers = new Set(shapes.map((s) => s.layer));
    if (!layers.has("subject")) errs.push(`${id} no subject layer`);
    if (["Epic", "Legendary", "Mythic"].includes(card.rarity)) for (const l of ["far", "mid", "subject", "near"] as const) if (!layers.has(l)) errs.push(`${id} no ${l}`);
    if (ART_DETAIL_SHORT.has(id)) errs.push(`${id} short`);
    const base = artSpec(card.id, themeFor(card.tags, card.type), compositionFor(card.type), false, null);
    const rad = artSpec(card.id, themeFor(card.tags, card.type), compositionFor(card.type), true, null);
    base.ridges.forEach((r, k) => { if (rad.ridges[k]?.fill === r.fill) errs.push(`${id} radiant same fill #${k}`); });
    for (const hex of [base.sky.from, base.sky.to, base.glow.color, ...base.ridges.flatMap((r) => [r.fill, ...(r.stroke === undefined ? [] : [r.stroke])])]) if (gold(hex)) errs.push(`${id} gold ${hex}`);
    const e = scene.emblem;
    if (e === undefined || e.size > 8 || e.x < 10 || e.x > 90 || e.y < 22 || e.y > 84) errs.push(`${id} emblem`);
    const own = composeScene(scene, artSeedOf(id, "alt"));
    if (JSON.stringify([...new Set(own.map((s) => s.part))].sort()) !== JSON.stringify([...new Set(shapes.map((s) => s.part))].sort())) errs.push(`${id} alt parts differ`);
    for (const x of scene.extras ?? []) if (!shapes.some((s) => s.layer === x.layer && s.part === x.pose)) errs.push(`${id} extra ${x.layer}`);
    if (card.type === "Unit") {
      if (MACHINES[id] !== undefined) { if (scene.figure !== undefined || !(scene.layers.subject ?? []).some((p) => p.part === MACHINES[id])) errs.push(`${id} machine`); }
      else if (scene.figure === undefined) errs.push(`${id} no figure`);
      else { if (JSON.stringify(scene.figure.look) === JSON.stringify(figureLookFor(card.tags))) errs.push(`${id} tag look`); const k = JSON.stringify(scene.figure.look); if (looks.has(k)) errs.push(`${id} look dup`); looks.set(k, id); }
    }
    for (const l of SCENE_LAYERS) for (const p of scene.layers[l] ?? []) if (["cyclone","head-cloud","puppet-hand","strings","rotunda","map","bow","doll","floating-island","rails","anvil","seal"].includes(p.part)) errs.push(`${id} forbidden ${p.part}`);
  }
  expect(errs).toEqual([]);
});
