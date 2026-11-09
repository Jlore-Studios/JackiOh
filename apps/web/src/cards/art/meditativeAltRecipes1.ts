// R3404000: Meditative's alt scene recipes, part 1 of three (#1 to #38), one per card offered Alt art, each drawn
// from its brief (ART.md, "Meditative's Alt art restaged"): the card's own subject in another place, time or moment,
// under a sky sharing no colour with its normal picture's and a new camera (a close-up, a wide or distant view, or
// one from above or below), its figure the normal picture's person (`meditativeLookOf`), in Meditative's ink wash,
// one red seal in its near layer and at or above its rarity's floor. scene.ts `sceneRecipeOf` reads one under the
// card's Alt art seed alone, at every Alt art level. Presentation only (CLAUDE.md rule 7): a recipe is read by the
// card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import { MEDITATIVE_RECIPES_1 } from "./meditativeRecipes1.ts";
import { MEDITATIVE_RECIPES_2 } from "./meditativeRecipes2.ts";
import { MEDITATIVE_RECIPES_3 } from "./meditativeRecipes3.ts";
import { MEDITATIVE_RECIPES_4 } from "./meditativeRecipes4.ts";
import { MEDITATIVE_RECIPES_5 } from "./meditativeRecipes5.ts";
import type { SceneRecipe } from "./scene.ts";

/** The card's normal recipe, from the five Meditative tables. */
function normalOf(id: string): SceneRecipe | undefined {
  return MEDITATIVE_RECIPES_1[id] ?? MEDITATIVE_RECIPES_2[id] ?? MEDITATIVE_RECIPES_3[id] ?? MEDITATIVE_RECIPES_4[id] ?? MEDITATIVE_RECIPES_5[id];
}

/** The look of the card's normal figure, so its Alt picture draws the same person; with none, the default person. */
export function meditativeLookOf(id: string): FigureLook {
  return normalOf(id)?.figure?.look ?? figureLookFor([]);
}

/** The look of the card's normal `k`th extra figure, so a companion its brief keeps is the same person; with none, the default person. */
export function meditativeExtraLookOf(id: string, k: number): FigureLook {
  return normalOf(id)?.extras?.[k]?.look ?? figureLookFor([]);
}

/** Disruptive Disruptor (Meditative #1, Common Field Spell), Alt art: after the meeting, seen from directly above: the empty council table at summer midday, every hand still bound in red cord, green bamboo shadows striping the floor. */
const DISRUPTIVE_DISRUPTOR_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["green-400", "lime-200"],
  light: { x: 64, y: 30, r: 44, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "tatami", x: 50, y: 10 }],
    mid: [
      { part: "bamboo", x: 10, y: 98, scale: 2.4, paint: "green-700", fade: 0.4 },
      { part: "bamboo", x: 90, y: 96, scale: 2.2, flip: -1, paint: "green-700", fade: 0.4 },
      { part: "bamboo", x: 72, y: 40, scale: 1.4, paint: "green-700", fade: 0.3 },
      { part: "ink-person", x: 22, y: 32, scale: 0.6, count: 4, spread: [8, 3] },
    ],
    subject: [
      { part: "table", x: 50, y: 70, scale: 2.4, paint: "wood-700" },
      { part: "ink-card-fan", x: 34, y: 54, scale: 0.42 },
      { part: "ink-card-fan", x: 50, y: 52, scale: 0.42 },
      { part: "ink-card-fan", x: 66, y: 54, scale: 0.42, flip: -1 },
      { part: "red-thread", x: 32, y: 52, scale: 0.3 },
      { part: "red-thread", x: 68, y: 52, scale: 0.3, flip: -1 },
    ],
    near: [
      { part: "incense", x: 86, y: 92, scale: 0.9 },
      { part: "ink-seal", x: 16, y: 74 },
    ],
    light: [{ part: "light-shaft", x: 62, y: 14, scale: 1, paint: "white" }],
  },
};

/** Juicy Kumquat Melon (Meditative #4, Epic Spell), Alt art: close-up: the six slices balanced on a fisherman's oar over a jade-green river in summer rain, droplets bouncing off the orange flesh. */
const JUICY_KUMQUAT_MELON_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["green-700", "jade-400"],
  light: { x: 50, y: 40, r: 40, paint: "white", opacity: 0.3 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "ink-cloud", x: 30, y: 20, scale: 1.2, paint: "slate-700" }],
    far: [
      { part: "ink-peaks", x: 50, y: 38, scale: 0.6, paint: "green-900" },
      { part: "still-water", x: 50, y: 40, paint: "jade-400" },
    ],
    mid: [
      { part: "reed", x: 8, y: 78, scale: 2.2, count: 2, spread: [4, 2] },
      { part: "reed", x: 92, y: 74, scale: 2, flip: -1 },
      { part: "ripples", x: 50, y: 84, scale: 1.4, paint: "green-200" },
      { part: "oar", x: 52, y: 70, scale: 2.8 },
    ],
    subject: [{ part: "melon", x: 48, y: 68, scale: 2.4, paint: "orange-400" }],
    near: [
      { part: "drops", x: 44, y: 54, scale: 1.3, paint: "blue-200" },
      { part: "drops", x: 64, y: 58, scale: 1, paint: "blue-200" },
      { part: "reed", x: 90, y: 99, scale: 2.4 },
      { part: "ink-seal", x: 16, y: 74 },
    ],
    particles: [
      { part: "rain", x: 50, y: 36, scale: 1.4, paint: "slate-100" },
      { part: "rain", x: 50, y: 72, scale: 1.2, paint: "slate-100" },
    ],
  },
};

/** Death by 1000 cuts (Meditative #5, Rare Spell), Alt art: low angle in a snowy mountain pass at blazing sunrise: the armoured warrior braced as a blizzard of paper-thin blades streaks past, orange sky behind. */
const DEATH_BY_1000_CUTS_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["orange-400", "red-300"],
  light: { x: 74, y: 34, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 60, size: 5, rotate: 0 },
  figure: { x: 50, y: 114, scale: 1.7, pose: "pose-guard", look: meditativeLookOf("meditative-005") },
  layers: {
    sky: [{ part: "sun", x: 76, y: 34, scale: 1.4 }],
    far: [{ part: "ink-peaks", x: 50, y: 66, scale: 0.9, paint: "red-900" }],
    mid: [{ part: "snowfield", x: 50, y: 76 }],
    near: [
      { part: "blade", x: 26, y: 40, scale: 0.8, count: 4, spread: [12, 14], paint: "slate-100" },
      { part: "streaks", x: 72, y: 50, scale: 1.4, paint: "white" },
      { part: "shards", x: 28, y: 70, scale: 1.2, count: 2, spread: [8, 6] },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    particles: [
      { part: "snow", x: 50, y: 40, scale: 1.4 },
      { part: "streaks", x: 40, y: 28, scale: 1.2, paint: "white" },
    ],
  },
};

/** Introspection (Meditative #7, Legendary Field Spell), Alt art: wide vista: the monk meditating on a lone boulder in an autumn maple valley, red leaves circling him like thoughts, some streaking at distant shadow-foes. */
const INTROSPECTION_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-400", "orange-400"],
  light: { x: 50, y: 50, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  figure: { x: 50, y: 58, scale: 0.45, pose: "pose-sit", look: meditativeLookOf("meditative-007") },
  layers: {
    sky: [
      { part: "ink-cloud", x: 26, y: 22, scale: 0.8, paint: "white" },
      { part: "ink-cloud", x: 78, y: 18, scale: 0.6, paint: "white" },
    ],
    far: [
      { part: "mountains", x: 50, y: 48, paint: "wood-700" },
      { part: "tree", x: 14, y: 60, scale: 1.4, paint: "red-600" },
      { part: "tree", x: 86, y: 58, scale: 1.3, flip: -1, paint: "red-600" },
    ],
    mid: [
      { part: "field", x: 50, y: 62, paint: "wood-600" },
      { part: "tree", x: 26, y: 74, scale: 1.1, paint: "orange-400" },
      { part: "tree", x: 76, y: 76, scale: 1.2, flip: -1, paint: "red-600" },
      { part: "ghost-figure", x: 14, y: 82, scale: 0.6, count: 2, spread: [5, 2], paint: "slate-900", fade: 0.5 },
      { part: "ghost-figure", x: 88, y: 84, scale: 0.55, flip: -1, paint: "slate-900", fade: 0.5 },
    ],
    subject: [{ part: "mossy-boulder", x: 50, y: 66, scale: 1.4 }],
    near: [
      { part: "leaves", x: 50, y: 44, scale: 1.4, count: 2, spread: [10, 6], paint: "red-600" },
      { part: "grass-tuft", x: 12, y: 98, count: 3, spread: [8, 1], paint: "wood-600" },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    light: [
      { part: "halo", x: 50, y: 50, scale: 1.2, paint: "white" },
      { part: "light-shaft", x: 70, y: 14, scale: 0.9, paint: "white" },
    ],
    particles: [
      { part: "leaves", x: 30, y: 62, scale: 1.2, paint: "orange-400" },
      { part: "streaks", x: 34, y: 72, scale: 0.8, flip: -1, paint: "red-600" },
      { part: "motes", x: 50, y: 40, scale: 1, paint: "white" },
    ],
  },
};

/** Double Counting (Meditative #10, Epic Field Spell), Alt art: close-up of two clerks' hands on one abacus in a cramped attic during a winter snowfall, a warm stove casting every bead's shadow twice. */
const DOUBLE_COUNTING_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["slate-800", "wood-800"],
  light: { x: 18, y: 66, r: 40, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 50, y: 72, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 70, paint: "wood-700" },
      { part: "window", x: 74, y: 46, scale: 1.6 },
      { part: "snow", x: 74, y: 32, scale: 0.4 },
    ],
    mid: [
      { part: "hearth", x: 14, y: 78, scale: 1.3 },
      { part: "table", x: 50, y: 99, scale: 4.2, paint: "wood-600" },
      { part: "abacus", x: 57, y: 85, scale: 3.4, paint: "black", fade: 0.25 },
    ],
    subject: [
      { part: "abacus", x: 50, y: 84, scale: 3.4 },
      { part: "hand-hold", x: 34, y: 64, scale: 1.6 },
      { part: "hand-hold", x: 66, y: 66, scale: 1.6, flip: -1 },
    ],
    near: [
      { part: "steam", x: 14, y: 52, scale: 1 },
      { part: "ink-seal", x: 86, y: 74 },
    ],
    light: [{ part: "halo", x: 16, y: 68, scale: 1.6, paint: "orange-400" }],
    particles: [{ part: "embers", x: 18, y: 60, scale: 1, paint: "orange-400" }],
  },
};

/** Fear Mongerer (Meditative #12, Epic Unit), Alt art: close-up portrait: the gaunt man whispering in a listener's ear on a bright spring market day, his shadow alone stretching long and black across the cobbles. */
const FEAR_MONGERER_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "green-200"],
  light: { x: 80, y: 22, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 40, y: 64, size: 5, rotate: 0 },
  figure: { x: 38, y: 114, scale: 1.7, pose: "pose-hold", look: meditativeLookOf("meditative-012") },
  extras: [{ layer: "subject", x: 76, y: 118, scale: 1.6, flip: -1, pose: "pose-stand", look: { ...figureLookFor(["Human"]), hair: "kerchief", cloth: "jade-400" } }],
  layers: {
    sky: [{ part: "sun", x: 82, y: 22, scale: 0.8 }],
    far: [
      { part: "awning", x: 18, y: 60, scale: 1.2, paint: "red-600" },
      { part: "awning", x: 84, y: 58, scale: 1.1, flip: -1, paint: "jade-400" },
      { part: "tiled-house", x: 52, y: 52, scale: 1 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 62 },
      { part: "flowers", x: 12, y: 74, count: 3, spread: [6, 2] },
      { part: "brush-stroke", x: 22, y: 98, scale: 1.4, paint: "black", fade: 0.5 },
    ],
    near: [
      { part: "petals", x: 60, y: 30, scale: 1.2, paint: "red-300" },
      { part: "ink-seal", x: 84, y: 76 },
    ],
  },
};

/** Gatling Pea (Meditative #13, Epic Unit), Alt art: winter, seen from behind: the pea vine frosted over in a snowed-in greenhouse, firing hard peas through a broken pane, white and icy blue. */
const GATLING_PEA_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["blue-200", "white"],
  light: { x: 76, y: 40, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 36, y: 66, size: 6, rotate: 0 },
  layers: {
    far: [
      { part: "snowfield", x: 50, y: 50 },
      { part: "greenhouse", x: 50, y: 80, scale: 1.45, paint: "slate-400" },
    ],
    mid: [
      { part: "glass-wall", x: 80, y: 82, scale: 1.1 },
      { part: "snowfield", x: 50, y: 82 },
      { part: "mist", x: 50, y: 80, scale: 0.9, paint: "white" },
    ],
    subject: [{ part: "pea-vine", x: 36, y: 94, scale: 1.9, paint: "green-200" }],
    near: [
      { part: "shards", x: 76, y: 52, scale: 1.4 },
      { part: "pale-bean", x: 64, y: 48, scale: 0.8, count: 3, spread: [10, 4] },
      { part: "ink-seal", x: 16, y: 74 },
    ],
    particles: [
      { part: "snow", x: 50, y: 34, scale: 1.4 },
      { part: "snow", x: 70, y: 60, scale: 1 },
    ],
  },
};

/** Trenful Trickster (Meditative #16, Rare Unit), Alt art: next morning, a wide shot of a busy temple fair as his hidden snares spring one after another, the trickster grinning from a rooftop, festival red. */
const TRENFUL_TRICKSTER_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-400", "red-500"],
  light: { x: 30, y: 28, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 46, y: 74, size: 5, rotate: 0 },
  figure: { x: 78, y: 44, scale: 0.45, flip: -1, pose: "pose-kneel", look: meditativeLookOf("meditative-016") },
  layers: {
    sky: [{ part: "bunting", x: 46, y: 28, scale: 1.2 }],
    far: [
      { part: "temple", x: 32, y: 58, scale: 0.9, paint: "red-600" },
      { part: "tiled-house", x: 78, y: 62, scale: 1.2 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 62 },
      { part: "awning", x: 14, y: 76, scale: 0.8, paint: "red-600" },
      { part: "crowd", x: 50, y: 82, scale: 0.7 },
    ],
    subject: [{ part: "snare", x: 30, y: 88, scale: 1.4 }],
    near: [
      { part: "snare", x: 58, y: 94, scale: 1.2, flip: -1 },
      { part: "snare", x: 84, y: 90, scale: 1 },
      { part: "burst", x: 58, y: 88, scale: 0.6 },
      { part: "paper-lantern", x: 12, y: 26, scale: 1.2 },
      { part: "ink-seal", x: 16, y: 70 },
    ],
  },
};

/** True Craft a Card (Meditative #17, Mythic Spell), Alt art: the finished card cooling on an anvil in a sunlit open-air forge yard at noon, the inventor wiping her brow, warm terracotta tones, wide shot. */
const TRUE_CRAFT_A_CARD_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "red-400"],
  light: { x: 50, y: 20, r: 44, paint: "white", opacity: 0.45 },
  emblem: { x: 60, y: 64, size: 5, rotate: 0 },
  figure: { x: 30, y: 80, scale: 0.55, pose: "pose-raise", look: meditativeLookOf("meditative-017") },
  layers: {
    sky: [
      { part: "sun", x: 50, y: 18, scale: 1 },
      { part: "cloud", x: 82, y: 26, scale: 1, paint: "white" },
      { part: "cloud", x: 16, y: 22, scale: 0.8, paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 56, paint: "red-900" },
      { part: "smithy", x: 82, y: 66, scale: 1.1 },
      { part: "forge", x: 14, y: 62, scale: 0.6 },
    ],
    mid: [
      { part: "yard-sand", x: 50, y: 66, paint: "red-300" },
      { part: "crate", x: 90, y: 82, scale: 0.8, count: 2, spread: [4, 2] },
      { part: "barrel", x: 10, y: 82, scale: 1 },
    ],
    subject: [
      { part: "anvil", x: 60, y: 82, scale: 1.4 },
      { part: "card", x: 60, y: 71, scale: 1.1, paint: "slate-100" },
    ],
    near: [
      { part: "steam", x: 60, y: 60, scale: 1.2 },
      { part: "pail", x: 78, y: 94, scale: 1 },
      { part: "hammer", x: 42, y: 94, scale: 0.9 },
      { part: "playing-card", x: 70, y: 90, scale: 0.6, count: 2, spread: [6, 2] },
      { part: "ink-seal", x: 84, y: 74 },
    ],
    light: [
      { part: "halo", x: 60, y: 64, scale: 1.4, paint: "white" },
      { part: "god-rays", x: 50, y: 18, scale: 1, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 82, y: 56, scale: 1.2, count: 3, spread: [6, 4] },
      { part: "embers", x: 16, y: 50, scale: 1 },
      { part: "motes", x: 50, y: 46, scale: 1.2, paint: "white" },
    ],
  },
};

/** Expedition123 (Meditative #19, Epic Spell), Alt art: day one: the explorer setting off through a rain-soaked green tea plantation, the map still rolled under her arm, the rift a faint shimmer far off. */
const EXPEDITION123_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "green-400"],
  light: { x: 76, y: 36, r: 30, paint: "violet-300", opacity: 0.3 },
  emblem: { x: 40, y: 58, size: 5, rotate: 0 },
  figure: { x: 40, y: 78, scale: 0.5, pose: "pose-stride", look: { ...meditativeLookOf("meditative-019"), item: "item-map" } },
  layers: {
    sky: [{ part: "ink-cloud", x: 30, y: 22, scale: 1, paint: "slate-700" }],
    far: [
      { part: "ink-peaks", x: 50, y: 46, scale: 0.6, paint: "green-900" },
      { part: "rift", x: 78, y: 36, scale: 0.5, fade: 0.4 },
    ],
    mid: [
      { part: "terraces", x: 50, y: 52 },
      { part: "bush", x: 20, y: 70, scale: 1.2, count: 3, spread: [10, 3], paint: "green-700" },
      { part: "mist", x: 62, y: 60, scale: 1 },
    ],
    near: [
      { part: "bush", x: 84, y: 96, scale: 1.8, count: 2, spread: [6, 2], paint: "green-700" },
      { part: "grass-tuft", x: 12, y: 98, count: 2, spread: [5, 1], paint: "green-700" },
      { part: "ink-seal", x: 16, y: 72 },
    ],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.4, paint: "slate-100" },
      { part: "rain", x: 50, y: 70, scale: 1.2, paint: "slate-100" },
    ],
  },
};

/** Aestheticize the Game (Meditative #20, Mythic Spell), Alt art: overhead: the artist lying back on a finished painting the size of a courtyard, the three victories drying in vivid ink, deep blue afternoon sky. */
const AESTHETICIZE_THE_GAME_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["blue-700", "blue-600"],
  light: { x: 78, y: 16, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 50, size: 5, rotate: 0 },
  figure: { x: 50, y: 68, scale: 0.5, pose: "pose-sit", look: meditativeLookOf("meditative-020") },
  layers: {
    sky: [
      { part: "cloud", x: 28, y: 18, scale: 1.2, paint: "white" },
      { part: "sun", x: 80, y: 14, scale: 0.6 },
    ],
    far: [
      { part: "tile-floor", x: 50, y: 26, paint: "stone-500" },
      { part: "tiled-house", x: 12, y: 32, scale: 0.8 },
      { part: "tiled-house", x: 88, y: 32, scale: 0.8, flip: -1 },
    ],
    mid: [
      { part: "ink-person", x: 22, y: 44, scale: 0.8, count: 4, spread: [10, 3] },
      { part: "paper-sheet", x: 50, y: 62, scale: 3 },
      { part: "brush-stroke", x: 24, y: 72, scale: 0.8, paint: "red-600" },
      { part: "brush-stroke", x: 50, y: 76, scale: 0.9, paint: "jade-400" },
      { part: "brush-stroke", x: 76, y: 72, scale: 0.8, paint: "black" },
    ],
    subject: [{ part: "brush", x: 64, y: 72, scale: 0.6 }],
    near: [
      { part: "jar", x: 12, y: 92, scale: 1, count: 2, spread: [4, 1] },
      { part: "petals", x: 70, y: 86, scale: 1, paint: "red-300" },
      { part: "bamboo", x: 92, y: 99, scale: 1.2 },
      { part: "ink-seal", x: 86, y: 76 },
    ],
    light: [
      { part: "halo", x: 50, y: 62, scale: 2, paint: "white" },
      { part: "light-shaft", x: 78, y: 10, scale: 1, paint: "white" },
      { part: "god-rays", x: 80, y: 14, scale: 0.9, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" },
      { part: "petals", x: 30, y: 40, scale: 1.2, paint: "red-300" },
      { part: "sparks", x: 50, y: 70, scale: 1, count: 2, spread: [20, 6], paint: "jade-400" },
    ],
  },
};

/** API Key Fishing (Meditative #21, Rare Spell), Alt art: ice fishing at noon: the hooded angler crouched over a hole in a frozen white lake, dangling the glowing key as a rival's cards drift up from below. */
const API_KEY_FISHING_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "white"],
  light: { x: 54, y: 62, r: 30, paint: "sky-300", opacity: 0.35 },
  emblem: { x: 34, y: 56, size: 5, rotate: 0 },
  figure: { x: 34, y: 72, scale: 0.6, pose: "pose-kneel", look: meditativeLookOf("meditative-021") },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 48, scale: 0.6, paint: "blue-800" },
      { part: "snowfield", x: 50, y: 50 },
    ],
    mid: [
      { part: "ice-floor", x: 50, y: 60 },
      { part: "halo", x: 56, y: 78, scale: 0.7, paint: "blue-900" },
      { part: "ripples", x: 56, y: 78, scale: 0.5, paint: "blue-200" },
    ],
    near: [
      { part: "halo", x: 56, y: 62, scale: 0.9, paint: "sky-300", props: true },
      { part: "key", x: 56, y: 62, scale: 0.9, props: true },
      { part: "playing-card", x: 62, y: 86, scale: 0.6, count: 3, spread: [8, 4] },
      { part: "ink-seal", x: 84, y: 72 },
    ],
    light: [{ part: "light-shaft", x: 56, y: 20, scale: 0.8, paint: "white" }],
    particles: [{ part: "snow", x: 50, y: 36, scale: 1.2 }],
  },
};

/** Polymorph (Meditative #24, Common Spell), Alt art: aftermath close-up: the bewildered sheep still wearing the warrior's dented helmet, blinking in a rain-soaked green spring meadow. */
const POLYMORPH_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["grey-500", "green-400"],
  light: { x: 30, y: 30, r: 34, paint: "white", opacity: 0.3 },
  emblem: { x: 44, y: 74, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "hills", x: 50, y: 48, paint: "green-700" }],
    mid: [
      { part: "field", x: 50, y: 58, paint: "green-500" },
      { part: "flowers", x: 16, y: 74, count: 3, spread: [10, 3] },
    ],
    subject: [
      { part: "ink-sheep", x: 46, y: 80, scale: 2.6 },
      { part: "helmet", x: 60, y: 60, scale: 1.6 },
    ],
    near: [
      { part: "grass-tuft", x: 50, y: 99, count: 4, spread: [40, 1], paint: "green-700" },
      { part: "flowers", x: 86, y: 96, count: 2, spread: [5, 1] },
      { part: "ink-seal", x: 16, y: 72 },
    ],
    particles: [{ part: "rain", x: 50, y: 40, scale: 1.4, paint: "slate-100" }],
  },
};

/** The Conductor (Meditative #31, Rare Unit), Alt art: inside the moving carriage: the conductor clipping tickets for glowing conjured passengers as a sunlit valley of red autumn maples rushes past the windows. */
const THE_CONDUCTOR_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-400", "red-500"],
  light: { x: 26, y: 26, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 66, y: 62, size: 5, rotate: 0 },
  figure: { x: 68, y: 94, scale: 1.3, flip: -1, pose: "pose-hold", look: meditativeLookOf("meditative-031") },
  layers: {
    sky: [{ part: "sun", x: 26, y: 24, scale: 0.7 }],
    far: [
      { part: "hills", x: 50, y: 56, paint: "wood-700" },
      { part: "tree", x: 22, y: 60, scale: 1.2, count: 3, spread: [14, 2], paint: "red-600" },
      { part: "streaks", x: 60, y: 48, scale: 1.4, paint: "white" },
    ],
    mid: [
      { part: "column", x: 8, y: 82, scale: 1.3, paint: "red-900" },
      { part: "column", x: 42, y: 82, scale: 1.3, paint: "red-900" },
      { part: "column", x: 94, y: 82, scale: 1.3, paint: "red-900" },
      { part: "boards", x: 50, y: 76 },
      { part: "bench", x: 28, y: 88, scale: 1 },
    ],
    subject: [
      { part: "ghost-figure", x: 26, y: 88, scale: 0.9, paint: "sky-300" },
      { part: "ghost-figure", x: 44, y: 90, scale: 0.8, flip: -1, paint: "sky-300" },
    ],
    near: [
      { part: "ticket-stub", x: 52, y: 58, scale: 1.2 },
      { part: "ink-seal", x: 16, y: 70 },
    ],
    light: [{ part: "light-shaft", x: 24, y: 20, scale: 0.9, paint: "white" }],
  },
};

/** Spiritually 中国 (Meditative #32, Epic Spell), Alt art: a lone traveller climbing a snowy monastery stair as every card in her hand turns red, lion dancers bounding out of the falling snow, white and crimson. */
const SPIRITUALLY_CN_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["white", "red-600"],
  light: { x: 50, y: 40, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 48, y: 56, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "snow", x: 50, y: 24, scale: 1.4 }],
    far: [
      { part: "ink-peaks", x: 50, y: 44, scale: 0.7, paint: "slate-400" },
      { part: "snowfield", x: 50, y: 46 },
      { part: "pagoda", x: 18, y: 52, scale: 0.7 },
      { part: "temple", x: 50, y: 42, scale: 0.5 },
    ],
    mid: [{ part: "stairs", x: 50, y: 42 }],
    subject: [
      { part: "lion-dancer", x: 76, y: 70, scale: 0.9 },
      { part: "ink-person", x: 48, y: 72, scale: 1.4 },
      { part: "ink-card-fan", x: 50, y: 62, scale: 0.5, paint: "red-600" },
      { part: "lion-dancer", x: 22, y: 82, scale: 0.8, flip: -1 },
    ],
    near: [
      { part: "snow", x: 50, y: 62, scale: 1.4 },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    particles: [{ part: "snow", x: 50, y: 40, scale: 1.2 }],
  },
};

/** CN Peptides (Meditative #36, Common Spell), Alt art: close-up of the warrior's face mid-change in a sunny training yard, one arm bulging, the other curling into a tentacle, bright turquoise sky. */
const CN_PEPTIDES_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["cyan-400", "sky-300"],
  light: { x: 80, y: 22, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  figure: { x: 50, y: 116, scale: 1.8, pose: "pose-flex", look: meditativeLookOf("meditative-036") },
  layers: {
    sky: [{ part: "sun", x: 82, y: 22, scale: 0.9 }],
    far: [{ part: "yard-sand", x: 50, y: 66 }],
    mid: [
      { part: "pell-post", x: 12, y: 82, scale: 1.2 },
      { part: "pell-post", x: 88, y: 80, scale: 1, flip: -1 },
    ],
    near: [
      { part: "vine", x: 76, y: 78, scale: 1.4, paint: "violet-500" },
      { part: "ink-seal", x: 16, y: 72 },
    ],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1, paint: "white" }],
  },
};

/** CN in a Bottle (Meditative #37, Epic Spell), Alt art: inside the bottle: the tiny city seen from street level, its sky the curved green glass, a giant eye peering in from outside. */
const CN_IN_A_BOTTLE_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["green-500", "green-200"],
  light: { x: 50, y: 60, r: 40, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "eye", x: 70, y: 32, scale: 3, paint: "green-900" },
      { part: "skyline", x: 50, y: 64, scale: 0.8, paint: "slate-700" },
    ],
    mid: [
      { part: "street", x: 50, y: 64 },
      { part: "tiled-house", x: 14, y: 80, scale: 1.4 },
      { part: "tiled-house", x: 86, y: 80, scale: 1.4, flip: -1 },
      { part: "tiled-house", x: 30, y: 70, scale: 0.8 },
    ],
    subject: [{ part: "bottle", x: 54, y: 50, scale: 6.5 }],
    near: [
      { part: "glass-dome", x: 50, y: 99, scale: 2.2, paint: "green-200" },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" }],
  },
};

/** H1B Printer (Meditative #38, Rare Field Spell), Alt art: a rooftop printer at sunrise feeding workers down a ramp of paper to a harbour, a ship of applicants waiting, rose-orange sky, wide shot. */
const H1B_PRINTER_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["red-300", "orange-400"],
  light: { x: 76, y: 34, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 30, y: 36, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 76, y: 34, scale: 1 }],
    far: [
      { part: "sea", x: 50, y: 62 },
      { part: "ship", x: 78, y: 68, scale: 1 },
    ],
    mid: [
      { part: "tiled-house", x: 26, y: 74, scale: 1.8 },
      { part: "boards", x: 50, y: 84 },
      { part: "crowd", x: 68, y: 90, scale: 0.5 },
    ],
    subject: [
      { part: "printer", x: 28, y: 48, scale: 0.9 },
      { part: "ink-person", x: 46, y: 64, scale: 0.8, count: 3, spread: [6, 6] },
    ],
    near: [
      { part: "paper-sheet", x: 46, y: 62, scale: 0.5, count: 3, spread: [8, 8] },
      { part: "ink-seal", x: 84, y: 74 },
    ],
  },
};

/** R3404000: the alt recipes of Meditative #1 to #38, by card id. */
export const MEDITATIVE_ALT_RECIPES_1: Readonly<Record<string, SceneRecipe>> = {
  "meditative-001": DISRUPTIVE_DISRUPTOR_ALT,
  "meditative-004": JUICY_KUMQUAT_MELON_ALT,
  "meditative-005": DEATH_BY_1000_CUTS_ALT,
  "meditative-007": INTROSPECTION_ALT,
  "meditative-010": DOUBLE_COUNTING_ALT,
  "meditative-012": FEAR_MONGERER_ALT,
  "meditative-013": GATLING_PEA_ALT,
  "meditative-016": TRENFUL_TRICKSTER_ALT,
  "meditative-017": TRUE_CRAFT_A_CARD_ALT,
  "meditative-019": EXPEDITION123_ALT,
  "meditative-020": AESTHETICIZE_THE_GAME_ALT,
  "meditative-021": API_KEY_FISHING_ALT,
  "meditative-024": POLYMORPH_ALT,
  "meditative-031": THE_CONDUCTOR_ALT,
  "meditative-032": SPIRITUALLY_CN_ALT,
  "meditative-036": CN_PEPTIDES_ALT,
  "meditative-037": CN_IN_A_BOTTLE_ALT,
  "meditative-038": H1B_PRINTER_ALT,
};
