// R3398000: Core's alt scene recipes, part 1 of three (#3 to #35), one per card offered Alt art, each drawn from its
// brief (ART.md, "Core's Alt art restaged"): the card's own subject in another place, time or moment, under a sky
// sharing no colour with its normal picture's and a new camera (a close-up, a wide or distant view, or one from
// above), its figure the normal picture's person (`normalCoreLookOf`), in Core's glass-orb look and at or above its
// rarity's floor. scene.ts `sceneRecipeOf` reads one under the card's Alt art seed alone, at every Alt art level.
// Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";
import { CORE_SCENES_1 } from "./sceneRecipesCore1.ts";
import { CORE_SCENES_2 } from "./sceneRecipesCore2.ts";
import { CORE_SCENES_3 } from "./sceneRecipesCore3.ts";

/** The card's normal recipe, from the three Core tables. */
function normalOf(id: string): SceneRecipe | undefined {
  return CORE_SCENES_1[id] ?? CORE_SCENES_2[id] ?? CORE_SCENES_3[id];
}

/** The look of the card's normal figure, so its Alt picture draws the same person; with none, the default person. */
export function normalCoreLookOf(id: string): FigureLook {
  return normalOf(id)?.figure?.look ?? figureLookFor([]);
}

/** The look of the card's normal `k`th extra figure, so a crowd its brief keeps is the same people; with none, the default person. */
export function normalCoreExtraLookOf(id: string, k: number): FigureLook {
  return normalOf(id)?.extras?.[k]?.look ?? figureLookFor([]);
}

/** Right-house defender (Core #3, Common), Alt art: before he finds it, the guard squinting at two identical cottages at a rainy country crossroads, shield-light flickering, map dripping; a wide shot in slate blue and moss green. */
const RIGHT_HOUSE_DEFENDER_ALT: SceneRecipe = {
  family: "core",
  sky: ["slate-700", "slate-400"],
  light: { x: 24, y: 20, r: 36, paint: "slate-100", opacity: 0.25 },
  emblem: { x: 50, y: 30, size: 5, rotate: 0 },
  figure: { x: 50, y: 82, scale: 0.55, pose: "pose-hold", look: { ...normalCoreLookOf("core-003") } },
  layers: {
    sky: [{ part: "cloud", x: 40, y: 22, scale: 1.4, count: 3, spread: [30, 4], paint: "slate-800" }],
    far: [
      { part: "hills", x: 50, y: 58, paint: "green-900" },
      { part: "tree", x: 10, y: 62, scale: 1.2, paint: "green-900" },
      { part: "tree", x: 92, y: 61, scale: 1.1, flip: -1, paint: "green-900" },
    ],
    mid: [
      { part: "field", x: 50, y: 64, paint: "green-900" },
      { part: "road", x: 50, y: 64 },
      { part: "house", x: 24, y: 70, scale: 0.62, paint: "slate-400" },
      { part: "house", x: 76, y: 70, scale: 0.62, paint: "slate-400" },
      { part: "fence-post", x: 50, y: 72, scale: 0.7 },
    ],
    near: [
      { part: "grass-tuft", x: 14, y: 96, count: 3, spread: [10, 2], paint: "green-700" },
      { part: "grass-tuft", x: 86, y: 97, count: 2, spread: [6, 2], paint: "green-700" },
      { part: "drops", x: 54, y: 70, scale: 0.5, paint: "sky-300" },
    ],
    light: [{ part: "halo", x: 47, y: 68, scale: 0.45, paint: "cyan-400" }],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.4, paint: "slate-100" },
      { part: "rain", x: 50, y: 74, scale: 1.2, paint: "slate-100" },
    ],
  },
};

/** Mana Well (Core #6, Common), Alt art: a high wide view of the well alone on a windswept hilltop in autumn drizzle, blue mana steaming up past bare trees, in rust and slate. */
const MANA_WELL_ALT: SceneRecipe = {
  family: "core",
  sky: ["slate-700", "stone-500"],
  light: { x: 70, y: 22, r: 34, paint: "slate-100", opacity: 0.25 },
  emblem: { x: 50, y: 58, size: 4, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 36, y: 22, scale: 1.3, count: 3, spread: [30, 5], paint: "slate-800" }],
    far: [{ part: "hills", x: 50, y: 56, paint: "red-900" }],
    mid: [
      { part: "field", x: 50, y: 62, paint: "wood-700" },
      { part: "dead-tree", x: 26, y: 66, scale: 0.9 },
      { part: "dead-tree", x: 76, y: 65, scale: 0.75, flip: -1 },
      { part: "grass-tuft", x: 50, y: 72, count: 4, spread: [30, 6], paint: "brass-600" },
    ],
    subject: [{ part: "well", x: 50, y: 68, scale: 0.8 }],
    near: [
      { part: "grass-tuft", x: 12, y: 96, count: 3, spread: [8, 2], paint: "brass-600" },
      { part: "grass-tuft", x: 88, y: 96, count: 3, spread: [8, 2], paint: "wood-700" },
    ],
    light: [
      { part: "steam", x: 50, y: 50, scale: 1.4, paint: "cyan-400" },
      { part: "halo", x: 50, y: 56, scale: 0.7, paint: "cyan-400" },
    ],
    particles: [
      { part: "rain", x: 50, y: 44, scale: 1.4, paint: "slate-100" },
      { part: "leaves", x: 40, y: 40, scale: 1.2 },
      { part: "leaves", x: 72, y: 74, scale: 1 },
    ],
  },
};

/** Jewelosco Scarab (Core #7, Rare), Alt art: a close-up of the scarab resting on its finished trinket ball beside a turquoise oasis pool at midday, palm reflections rippling over its shell. */
const JEWELOSCO_SCARAB_ALT: SceneRecipe = {
  family: "core",
  sky: ["blue-600", "cyan-400"],
  light: { x: 50, y: 10, r: 44, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 80, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 50, y: 12, scale: 0.6 }],
    far: [
      { part: "palm", x: 12, y: 60, scale: 1.4 },
      { part: "palm", x: 88, y: 58, scale: 1.2, flip: -1 },
      { part: "reed", x: 30, y: 60, count: 2, spread: [6, 1] },
    ],
    mid: [
      { part: "water", x: 50, y: 58, paint: "cyan-400" },
      { part: "reed", x: 8, y: 74, scale: 1.6, count: 2, spread: [4, 1] },
    ],
    subject: [
      { part: "gem-ball", x: 50, y: 82, scale: 2.6 },
      { part: "scarab", x: 50, y: 66, scale: 2.2 },
    ],
    near: [
      { part: "palm", x: 96, y: 112, scale: 2.4, flip: -1 },
      { part: "grass-tuft", x: 10, y: 98, count: 2, spread: [6, 1], paint: "jade-400" },
    ],
    light: [
      { part: "rim-light", x: 50, y: 58, scale: 1.4, paint: "white" },
      { part: "streaks", x: 46, y: 58, scale: 0.8 },
    ],
    particles: [{ part: "sparks", x: 50, y: 72, scale: 1.6, count: 2, spread: [16, 8], paint: "white" }],
  },
};

/** Moths to the Flame (Core #9, Rare), Alt art: daytime, the lantern creature asleep and unlit in a misty birch forest, one moth resting on its glass shoulder, in cool green-grey and seen from below; white `tree`s stand in for the birches. */
const MOTHS_TO_THE_FLAME_ALT: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "green-200"],
  light: { x: 50, y: 8, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 70, y: 30, size: 5, rotate: 0 },
  figure: { x: 52, y: 100, scale: 1.5, pose: "pose-sit", look: { ...normalCoreLookOf("core-009"), cloth: "stone-500" } },
  layers: {
    far: [
      { part: "tree", x: 14, y: 64, scale: 1.6, paint: "white" },
      { part: "tree", x: 40, y: 60, scale: 1.2, paint: "white" },
      { part: "tree", x: 86, y: 62, scale: 1.5, flip: -1, paint: "white" },
      { part: "mist", x: 50, y: 58, scale: 1.2, paint: "white" },
    ],
    mid: [
      { part: "field", x: 50, y: 74, paint: "green-900" },
      { part: "fern", x: 18, y: 82, scale: 1.6, count: 2, spread: [8, 2] },
      { part: "fern", x: 84, y: 84, scale: 1.4, flip: -1 },
    ],
    near: [
      { part: "moth", x: 60, y: 54, scale: 0.8 },
      { part: "fern", x: 8, y: 100, scale: 2.2 },
      { part: "mist", x: 50, y: 96, scale: 1.2, paint: "slate-100" },
    ],
    light: [{ part: "light-shaft", x: 40, y: 0, scale: 1.4, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" }],
  },
};

/** Tempo Timmy (Core #11, Common), Alt art: after the race, Timmy flopped on a hay bale under falling autumn leaves, stopwatch dangling, a ribbon on his chest (the emblem); a russet-red close-up. */
const TEMPO_TIMMY_ALT: SceneRecipe = {
  family: "core",
  sky: ["orange-400", "red-600"],
  light: { x: 76, y: 20, r: 38, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 48, y: 72, size: 5, rotate: 0 },
  figure: { x: 46, y: 116, scale: 1.65, pose: "pose-sit", look: { ...normalCoreLookOf("core-011") } },
  layers: {
    far: [
      { part: "tree", x: 12, y: 60, scale: 2, paint: "red-600" },
      { part: "tree", x: 88, y: 58, scale: 1.8, flip: -1, paint: "orange-400" },
    ],
    mid: [{ part: "field", x: 50, y: 70, paint: "wood-700" }],
    subject: [{ part: "hay-bale", x: 50, y: 112, scale: 3.4 }],
    near: [{ part: "leaves", x: 80, y: 92, scale: 1.2 }],
    light: [{ part: "rim-light", x: 46, y: 56, scale: 1.4, paint: "orange-400" }],
    particles: [
      { part: "leaves", x: 30, y: 34, scale: 1.4, props: true },
      { part: "leaves", x: 72, y: 46, scale: 1.2, props: true },
    ],
  },
};

/** Duplicating Felinors (Core #12, Rare), Alt art: seen from above, the whole row of identical Felinors napping on a seaside cottage's window seat, their scarves (their striped cloth) tangled together, in pastel teal; the `bench` stands in for the window seat. */
const DUPLICATING_FELINORS_ALT: SceneRecipe = {
  family: "core",
  sky: ["cyan-400", "sky-300"],
  light: { x: 50, y: 14, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 30, y: 78, scale: 0.6, pose: "pose-sit", look: { ...normalCoreLookOf("core-012"), cloth: "jade-400" } },
  extras: [
    { layer: "subject", x: 50, y: 78, scale: 0.6, pose: "pose-sit", look: { ...normalCoreExtraLookOf("core-012", 0), cloth: "jade-400" } },
    { layer: "subject", x: 70, y: 78, scale: 0.6, flip: -1, pose: "pose-sit", look: { ...normalCoreExtraLookOf("core-012", 1), cloth: "jade-400" } },
  ],
  layers: {
    sky: [{ part: "sea", x: 50, y: 24 }],
    far: [
      { part: "boards", x: 50, y: 42, paint: "slate-100" },
      { part: "window", x: 22, y: 44, scale: 1.2 },
      { part: "window", x: 78, y: 44, scale: 1.2 },
    ],
    mid: [{ part: "gull", x: 50, y: 30, scale: 0.8, count: 2, spread: [24, 4] }],
    subject: [{ part: "bench", x: 50, y: 82, scale: 2.6 }],
    near: [{ part: "sparks", x: 50, y: 62, scale: 1.2, count: 2, spread: [20, 4], paint: "white", props: true }],
    light: [{ part: "halo", x: 50, y: 64, scale: 1.2, paint: "white" }],
  },
};

/** Hit Job (Core #16, Common), Alt art: before the hit, the assassin waiting at a sunny rooftop cafe, checking a pocket watch (the stopwatch), the dagger hidden by a folded napkin on the table; a wide shot in warm terracotta. */
const HIT_JOB_ALT: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "orange-400"],
  light: { x: 78, y: 18, r: 40, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  figure: { x: 42, y: 80, scale: 0.6, pose: "pose-sit", look: { ...normalCoreLookOf("core-016"), item: "item-stopwatch" } },
  layers: {
    sky: [{ part: "sun-disc", x: 78, y: 18, scale: 1.2 }],
    far: [{ part: "skyline", x: 50, y: 62, scale: 1.1, paint: "red-600" }],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "red-600" },
      { part: "awning", x: 82, y: 72, scale: 0.8 },
      { part: "chair", x: 66, y: 80, scale: 0.55 },
    ],
    subject: [
      { part: "table", x: 54, y: 80, scale: 0.55 },
      { part: "dagger", x: 54, y: 74.6, scale: 0.6 },
      { part: "card", x: 55, y: 74.4, scale: 0.6 },
      { part: "cup", x: 50, y: 75.4, scale: 0.8 },
    ],
    near: [
      { part: "bush", x: 10, y: 96, scale: 1.4 },
      { part: "bush", x: 92, y: 98, scale: 1.2 },
    ],
    light: [{ part: "rim-light", x: 42, y: 66, scale: 0.8, paint: "white" }],
  },
};

/** Flood (Core #17, Rare), Alt art: after the flood, the drained courtyard under a pink-orange sunset, a puddle mirroring the sky and a lone helmet floating in it; a wide, still vista. */
const FLOOD_ALT: SceneRecipe = {
  family: "core",
  sky: ["red-300", "orange-400"],
  light: { x: 50, y: 52, r: 40, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 50, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun-disc", x: 50, y: 52, scale: 1.4 },
      { part: "cloud", x: 26, y: 24, scale: 1.2, count: 2, spread: [30, 4], paint: "red-300" },
    ],
    far: [
      { part: "fortress", x: 50, y: 58, scale: 1.1, paint: "red-900" },
      { part: "battlement", x: 14, y: 62, scale: 0.8, paint: "red-900" },
      { part: "battlement", x: 86, y: 62, scale: 0.8, paint: "red-900" },
    ],
    subject: [
      { part: "cobbles", x: 50, y: 62, paint: "wood-700" },
      { part: "water", x: 50, y: 82, paint: "red-300" },
      { part: "helmet", x: 62, y: 87, scale: 1.4 },
    ],
    near: [
      { part: "banner", x: 10, y: 94, scale: 1.2 },
      { part: "spray", x: 30, y: 90, scale: 0.8, paint: "white" },
    ],
    light: [{ part: "rim-light", x: 50, y: 60, scale: 1.4, paint: "orange-400" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" }],
  },
};

/** Bread and Butter (Core #18, Epic), Alt art: before it springs, the trap waiting on a picnic blanket by a windy lakeside, a basket of rolls beside it (the fruit basket, its rolls `bread`), seen from low in cool blue morning light. */
const BREAD_AND_BUTTER_ALT: SceneRecipe = {
  family: "core",
  sky: ["blue-800", "sky-400"],
  light: { x: 22, y: 30, r: 40, paint: "blue-200", opacity: 0.4 },
  emblem: { x: 46, y: 70, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 62, y: 22, scale: 1.2, count: 2, spread: [26, 4], paint: "slate-100" }],
    far: [
      { part: "mountains", x: 50, y: 56, paint: "slate-700" },
      { part: "pine", x: 86, y: 58, scale: 0.7, count: 3, spread: [8, 1] },
    ],
    mid: [
      { part: "sea", x: 50, y: 60 },
      { part: "field", x: 50, y: 74, paint: "green-900" },
      { part: "reed", x: 8, y: 80, scale: 2, count: 3, spread: [5, 1] },
      { part: "gingham-blanket", x: 48, y: 88, scale: 2.2, paint: "blue-600" },
    ],
    subject: [{ part: "loaf", x: 46, y: 84, scale: 2.4 }],
    near: [
      { part: "basket", x: 80, y: 92, scale: 1.6 },
      { part: "bread", x: 76, y: 86, scale: 1.2, count: 2, spread: [3, 1], props: true },
      { part: "grass-tuft", x: 10, y: 98, count: 3, spread: [8, 1], paint: "green-700" },
    ],
    light: [{ part: "rim-light", x: 46, y: 74, scale: 1.4, paint: "blue-200" }],
    particles: [
      { part: "gust", x: 30, y: 46, scale: 1.2 },
      { part: "leaves", x: 64, y: 50, scale: 1.2 },
    ],
  },
};

/** Carnivorous Cube (Core #22, Epic), Alt art: later, the cube bursting open on a sunny lawn, its two knights tumbling out dazed and dripping goo; a wide shot in candy pink. */
const CARNIVOROUS_CUBE_ALT: SceneRecipe = {
  family: "core",
  sky: ["red-300", "white"],
  light: { x: 76, y: 18, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 30, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 30, y: 82, scale: 0.42, flip: -1, pose: "pose-spread", look: { ...normalCoreExtraLookOf("core-022", 0) } },
    { layer: "subject", x: 72, y: 84, scale: 0.42, pose: "pose-kneel", look: { ...normalCoreExtraLookOf("core-022", 1) } },
  ],
  layers: {
    sky: [
      { part: "sun-disc", x: 76, y: 18, scale: 1.1 },
      { part: "cloud", x: 26, y: 26, scale: 1.1, paint: "white" },
    ],
    far: [
      { part: "candy-mountain", x: 18, y: 64, scale: 0.8 },
      { part: "hills", x: 50, y: 60, paint: "red-300" },
    ],
    mid: [
      { part: "field", x: 50, y: 66, paint: "lime-400" },
      { part: "bush", x: 90, y: 72, scale: 1.2, paint: "red-300" },
      { part: "flower", x: 14, y: 78, scale: 1.4, count: 3, spread: [6, 2] },
    ],
    subject: [
      { part: "cube", x: 50, y: 78, scale: 1.1 },
      { part: "drops", x: 30, y: 74, scale: 0.8, paint: "green-400" },
      { part: "drops", x: 72, y: 76, scale: 0.8, paint: "green-400" },
    ],
    near: [
      { part: "spray", x: 50, y: 70, scale: 1.2, paint: "green-200" },
      { part: "flower", x: 88, y: 96, scale: 2, count: 2, spread: [5, 1] },
    ],
    light: [{ part: "rim-light", x: 50, y: 62, scale: 1.2, paint: "white" }],
    particles: [{ part: "sparks", x: 50, y: 56, scale: 1.4, count: 2, spread: [16, 6], paint: "white" }],
  },
};

/** Reoccurring Dream (Core #23, Rare), Alt art: the morning after, sun on rumpled sheets, the dream door faintly drawn in the steam of a coffee cup on the windowsill; a warm peach close-up. */
const REOCCURRING_DREAM_ALT: SceneRecipe = {
  family: "core",
  sky: ["orange-400", "red-300"],
  light: { x: 30, y: 30, r: 44, paint: "white", opacity: 0.5 },
  emblem: { x: 18, y: 76, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 70, paint: "red-300" }],
    far: [{ part: "window", x: 36, y: 56, scale: 2.2 }],
    mid: [
      { part: "cup", x: 44, y: 56, scale: 3.4 },
      { part: "steam", x: 44, y: 48, scale: 1.6, paint: "white" },
      { part: "framed-door", x: 44, y: 38, scale: 0.7, paint: "white" },
    ],
    subject: [{ part: "bed", x: 56, y: 100, scale: 3 }],
    light: [
      { part: "light-shaft", x: 36, y: 34, scale: 1.4, paint: "white" },
      { part: "halo", x: 44, y: 32, scale: 0.7, paint: "white" },
    ],
    particles: [{ part: "motes", x: 50, y: 60, scale: 1.4, paint: "white" }],
  },
};

/** Efficiency Dividend (Core #24, Epic), Alt art: year's end, the worker asleep on a heap of empty cups in a grey, rainy break room, the three cups still glowing; seen from low, in cool blue. */
const EFFICIENCY_DIVIDEND_ALT: SceneRecipe = {
  family: "core",
  sky: ["slate-700", "blue-800"],
  light: { x: 74, y: 30, r: 38, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  figure: { x: 50, y: 98, scale: 1.3, pose: "pose-sit", look: { ...normalCoreLookOf("core-024") } },
  layers: {
    far: [
      { part: "window", x: 22, y: 52, scale: 2 },
      { part: "window", x: 78, y: 52, scale: 2 },
      { part: "rain", x: 22, y: 38, scale: 0.4, paint: "slate-100" },
      { part: "rain", x: 78, y: 38, scale: 0.4, paint: "slate-100" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 74, paint: "slate-800" },
      { part: "counter", x: 84, y: 82, scale: 0.8 },
    ],
    subject: [{ part: "cup", x: 50, y: 90, scale: 3.4, count: 10, spread: [28, 5] }],
    near: [
      { part: "cup", x: 26, y: 98, scale: 4 },
      { part: "cup", x: 50, y: 100, scale: 4 },
      { part: "cup", x: 74, y: 98, scale: 4 },
    ],
    light: [
      { part: "heal-glow", x: 50, y: 92, scale: 1.6 },
      { part: "halo", x: 26, y: 94, scale: 0.6, paint: "cyan-400" },
      { part: "halo", x: 74, y: 94, scale: 0.6, paint: "cyan-400" },
    ],
    particles: [{ part: "drops", x: 50, y: 30, scale: 1.4, paint: "sky-300" }],
  },
};

/** 4-mana 7/7 (Core #25, Common), Alt art: a close-up of the giant's armoured face peering out from under the crate's lid in a snowy railyard, cold blue-white. */
const FOUR_MANA_SEVEN_SEVEN_ALT: SceneRecipe = {
  family: "core",
  sky: ["slate-400", "slate-100"],
  light: { x: 30, y: 20, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 82, size: 5, rotate: 0 },
  figure: { x: 50, y: 128, scale: 1.8, pose: "pose-stand", look: { ...normalCoreLookOf("core-025") } },
  layers: {
    far: [
      { part: "skyline", x: 50, y: 56, scale: 1.1, paint: "slate-700" },
      { part: "snowfield", x: 50, y: 58 },
    ],
    mid: [
      { part: "rails", x: 50, y: 62 },
      { part: "crate", x: 12, y: 74, scale: 1.6 },
      { part: "crate", x: 88, y: 72, scale: 1.4 },
    ],
    near: [{ part: "crate-big", x: 50, y: 118, scale: 1.7 }],
    light: [{ part: "rim-light", x: 50, y: 40, scale: 1.4, paint: "white" }],
    particles: [
      { part: "snow", x: 50, y: 40, scale: 1.4 },
      { part: "snow", x: 50, y: 76, scale: 1.2 },
    ],
  },
};

/** Blood Ridden Glowy Jelly Bean (Core #27, Rare), Alt art: the bean cracked open on a mossy forest stump in morning fog, its red glow leaking into the moss; deep green, seen from low. */
const BLOOD_RIDDEN_ALT: SceneRecipe = {
  family: "core",
  sky: ["green-900", "green-700"],
  light: { x: 50, y: 56, r: 30, paint: "red-500", opacity: 0.35 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "pine", x: 12, y: 62, scale: 2.2, count: 2, spread: [6, 2] },
      { part: "pine", x: 88, y: 60, scale: 2, count: 2, spread: [6, 2] },
      { part: "mist", x: 50, y: 56, scale: 1.4, paint: "slate-100" },
    ],
    mid: [{ part: "field", x: 50, y: 74, paint: "green-700" }],
    subject: [
      { part: "stump", x: 50, y: 90, scale: 3.4 },
      { part: "jelly-bean", x: 50, y: 66, scale: 3.6, paint: "red-500" },
      { part: "drops", x: 58, y: 80, scale: 1.2, paint: "red-500" },
    ],
    near: [
      { part: "fern", x: 14, y: 100, scale: 2.4 },
      { part: "mushroom", x: 80, y: 98, scale: 1.6 },
      { part: "mist", x: 50, y: 96, scale: 1.2, paint: "slate-100" },
    ],
    light: [{ part: "halo", x: 50, y: 70, scale: 1, paint: "red-400" }],
    particles: [{ part: "spore-motes", x: 50, y: 50, scale: 1.4, paint: "green-200" }],
  },
};

/** Knockoff Temu Glowy Jelly Bean (Core #28, Rare), Alt art: mid-journey, the bean jostling in a shipping crate on a cargo ship's deck in grey seas, glowing patchily through the slats; wide, storm grey. */
const KNOCKOFF_TEMU_ALT: SceneRecipe = {
  family: "core",
  sky: ["slate-800", "slate-400"],
  light: { x: 50, y: 66, r: 26, paint: "lime-400", opacity: 0.3 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 40, y: 22, scale: 1.4, count: 3, spread: [30, 4], paint: "slate-700" }],
    far: [
      { part: "sea", x: 50, y: 56 },
      { part: "ship", x: 84, y: 58, scale: 0.6, flip: -1 },
    ],
    mid: [
      { part: "boards", x: 50, y: 70, paint: "wood-700" },
      { part: "crate", x: 20, y: 78, scale: 1.2, count: 2, spread: [6, 2] },
      { part: "crate", x: 82, y: 80, scale: 1.3 },
    ],
    subject: [
      { part: "crate-big", x: 50, y: 88, scale: 0.6 },
      { part: "jelly-bean", x: 50, y: 76, scale: 3, paint: "lime-400" },
    ],
    near: [{ part: "spray", x: 14, y: 72, scale: 1.4, paint: "white" }],
    light: [{ part: "halo", x: 50, y: 76, scale: 0.8, paint: "lime-200" }],
    particles: [{ part: "rain", x: 50, y: 44, scale: 1.4, paint: "slate-100" }],
  },
};

/** GIGA Glowy Jelly Bean (Core #29, Rare), Alt art: from the ground, a tiny hand holds a fan of cards up toward the enormous bean blotting out a teal sky; an extreme low angle. */
const GIGA_JELLY_BEAN_ALT: SceneRecipe = {
  family: "core",
  sky: ["cyan-400", "jade-400"],
  light: { x: 50, y: 30, r: 44, paint: "lime-200", opacity: 0.45 },
  emblem: { x: 80, y: 74, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "pine", x: 8, y: 104, scale: 3 },
      { part: "pine", x: 94, y: 104, scale: 2.8 },
    ],
    mid: [{ part: "field", x: 50, y: 92, paint: "green-900" }],
    subject: [
      { part: "jelly-bean", x: 50, y: 32, scale: 11, paint: "lime-400" },
      { part: "open-hand", x: 24, y: 100, scale: 2 },
      { part: "card-fan", x: 36, y: 86, scale: 2 },
    ],
    near: [{ part: "grass-tuft", x: 70, y: 98, count: 3, spread: [12, 2], paint: "green-700" }],
    light: [{ part: "heal-glow", x: 36, y: 80, scale: 1.8 }],
    particles: [{ part: "sparks", x: 50, y: 50, scale: 1.8, count: 2, spread: [20, 10], paint: "white" }],
  },
};

/** Prem Panther (Core #32, Rare), Alt art: a close-up portrait, the panther on a wet rock beside a waterfall, spray beading on its fur, in white mist and slate; the kit's panther bounds, so it springs from its rock. */
const PREM_PANTHER_ALT: SceneRecipe = {
  family: "core",
  sky: ["slate-400", "slate-100"],
  light: { x: 70, y: 24, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "cliff", x: 90, y: 30, flip: -1, paint: "slate-700" },
      { part: "waterfall", x: 74, y: 66, scale: 1.9 },
    ],
    mid: [
      { part: "water", x: 50, y: 70, paint: "slate-700" },
      { part: "mist", x: 70, y: 66, scale: 1.2, paint: "white" },
      { part: "standing-stone", x: 40, y: 108, scale: 2.8, flip: -1, paint: "slate-800" },
    ],
    subject: [{ part: "panther", x: 42, y: 66, scale: 2.8 }],
    near: [{ part: "mist", x: 50, y: 94, scale: 1.4, paint: "white" }],
    light: [{ part: "rim-light", x: 42, y: 62, scale: 1.6, paint: "white" }],
    particles: [
      { part: "spray", x: 60, y: 60, scale: 1.4, paint: "white" },
      { part: "drops", x: 40, y: 56, scale: 1.2, paint: "blue-200" },
    ],
  },
};

/** Unstable Clone Machine (Core #33, Rare), Alt art: the machine dumped and sparking in a sunlit junkyard, duplicate cards fluttering away on the breeze; a wide shot in rusty orange. */
const UNSTABLE_CLONE_MACHINE_ALT: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "orange-400"],
  light: { x: 20, y: 20, r: 40, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun-disc", x: 20, y: 20, scale: 1.2 }],
    far: [{ part: "hills", x: 50, y: 60, paint: "wood-700" }],
    mid: [
      { part: "sand", x: 50, y: 66, paint: "brass-600" },
      { part: "crate", x: 16, y: 76, scale: 1.1, count: 3, spread: [6, 2], paint: "wood-700" },
      { part: "bin", x: 86, y: 74, scale: 0.8 },
      { part: "cog", x: 78, y: 76, scale: 0.9, count: 2, spread: [6, 2] },
    ],
    subject: [
      { part: "pod", x: 44, y: 78, scale: 0.9 },
      { part: "hatch", x: 66, y: 82, scale: 0.7 },
    ],
    near: [
      { part: "cog", x: 12, y: 96, scale: 1.4 },
      { part: "playing-card", x: 70, y: 52, scale: 1.1, count: 4, spread: [18, 10] },
    ],
    light: [{ part: "zap", x: 44, y: 60, scale: 0.5, paint: "orange-400" }],
    particles: [
      { part: "sparks", x: 44, y: 62, scale: 1.4, count: 2, spread: [6, 4], paint: "orange-400" },
      { part: "gust", x: 64, y: 44, scale: 1.2 },
    ],
  },
};

/** Collateral Damage (Core #34, Rare), Alt art: the aftermath, an inspector in a hard hat (the steel helmet) surveying a clean crater on a quiet green hillside, one card scrap drifting by; an overcast wide vista. */
const COLLATERAL_DAMAGE_ALT: SceneRecipe = {
  family: "core",
  sky: ["slate-400", "stone-500"],
  light: { x: 30, y: 22, r: 40, paint: "white", opacity: 0.3 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  figure: {
    x: 24,
    y: 82,
    scale: 0.5,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-work-apron", item: "item-clipboard", cloth: "orange-400" },
  },
  layers: {
    sky: [{ part: "cloud", x: 40, y: 22, scale: 1.4, count: 3, spread: [30, 4], paint: "slate-100" }],
    far: [
      { part: "hills", x: 50, y: 56, paint: "green-700" },
      { part: "tree", x: 88, y: 60, scale: 1.1 },
    ],
    mid: [
      { part: "crater-field", x: 56, y: 62 },
      { part: "fence", x: 14, y: 70, scale: 0.8 },
    ],
    subject: [{ part: "playing-card", x: 66, y: 46, scale: 0.8 }],
    near: [{ part: "grass-tuft", x: 84, y: 96, count: 3, spread: [10, 2], paint: "green-500" }],
    light: [{ part: "rim-light", x: 24, y: 66, scale: 0.7, paint: "white" }],
    particles: [{ part: "dust", x: 56, y: 82, scale: 1.2 }],
  },
};

/** Lunar Eclipse (Core #35, Rare), Alt art: a daytime eclipse over a turquoise seaside cliff, its thin silver beam striking the surf, gulls scattering; wide, in bright teal. */
const LUNAR_ECLIPSE_ALT: SceneRecipe = {
  family: "core",
  sky: ["cyan-400", "sky-300"],
  light: { x: 68, y: 22, r: 36, paint: "white", opacity: 0.45 },
  emblem: { x: 20, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "moon", x: 68, y: 22, scale: 1.2 }],
    far: [{ part: "sea", x: 50, y: 58, paint: "jade-400" }],
    mid: [
      { part: "cliff", x: 30, y: 54, paint: "jade-400" },
      { part: "gull", x: 76, y: 46, scale: 1, count: 4, spread: [16, 8] },
    ],
    subject: [
      { part: "eclipse", x: 68, y: 22, scale: 1.2 },
      { part: "light-shaft", x: 68, y: 22, scale: 1.2, paint: "slate-100" },
    ],
    near: [{ part: "spray", x: 70, y: 84, scale: 1.6, paint: "white" }],
    light: [{ part: "halo", x: 68, y: 22, scale: 0.9, paint: "white" }],
    particles: [{ part: "motes", x: 60, y: 60, scale: 1.4, paint: "white" }],
  },
};

/** R3398000: Core's alt recipes #3 to #35, by card id. */
export const CORE_ALT_RECIPES_1: Readonly<Record<string, SceneRecipe>> = {
  "core-003": RIGHT_HOUSE_DEFENDER_ALT,
  "core-006": MANA_WELL_ALT,
  "core-007": JEWELOSCO_SCARAB_ALT,
  "core-009": MOTHS_TO_THE_FLAME_ALT,
  "core-011": TEMPO_TIMMY_ALT,
  "core-012": DUPLICATING_FELINORS_ALT,
  "core-016": HIT_JOB_ALT,
  "core-017": FLOOD_ALT,
  "core-018": BREAD_AND_BUTTER_ALT,
  "core-022": CARNIVOROUS_CUBE_ALT,
  "core-023": REOCCURRING_DREAM_ALT,
  "core-024": EFFICIENCY_DIVIDEND_ALT,
  "core-025": FOUR_MANA_SEVEN_SEVEN_ALT,
  "core-027": BLOOD_RIDDEN_ALT,
  "core-028": KNOCKOFF_TEMU_ALT,
  "core-029": GIGA_JELLY_BEAN_ALT,
  "core-032": PREM_PANTHER_ALT,
  "core-033": UNSTABLE_CLONE_MACHINE_ALT,
  "core-034": COLLATERAL_DAMAGE_ALT,
  "core-035": LUNAR_ECLIPSE_ALT,
};
