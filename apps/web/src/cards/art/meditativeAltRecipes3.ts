// R3404000: Meditative's alt scene recipes, part 3 of three (#72 to #100), one per card offered Alt art, each drawn
// from its brief as meditativeAltRecipes1.ts's are: the card's own subject in another place, time or moment, under a
// sky sharing no colour with its normal picture's and a new camera, its figure the normal picture's person, one red
// seal in its near layer and at or above its rarity's floor. Presentation only (CLAUDE.md rule 7).

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import { meditativeExtraLookOf, meditativeLookOf } from "./meditativeAltRecipes1.ts";
import type { SceneRecipe } from "./scene.ts";

/** The Banisher (Meditative #72, Common Unit), Alt art: daylight in a bleached desert ruin: the hooded figure's struck foe unravelling into a portal in the sand, ochre stone under a hard blue sky. */
const THE_BANISHER_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["blue-600", "sky-300"],
  light: { x: 80, y: 20, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 34, y: 58, size: 5, rotate: 0 },
  figure: { x: 34, y: 78, scale: 0.6, pose: "pose-strike", look: meditativeLookOf("meditative-072") },
  layers: {
    sky: [{ part: "sun", x: 80, y: 20, scale: 0.8 }],
    far: [{ part: "mesa", x: 76, y: 60, scale: 0.8, paint: "brass-600" }],
    mid: [
      { part: "sand", x: 50, y: 62 },
      { part: "ruin-wall", x: 14, y: 76, scale: 1, paint: "brass-600" },
      { part: "column", x: 88, y: 78, scale: 1, paint: "stone-500" },
    ],
    subject: [
      { part: "vortex", x: 64, y: 74, scale: 0.5, paint: "violet-500" },
      { part: "portal-ring", x: 64, y: 74, scale: 0.7, paint: "brass-600" },
      { part: "ghost-figure", x: 64, y: 82, scale: 0.8, fade: 0.5 },
    ],
    near: [
      { part: "fallen-column", x: 80, y: 94, scale: 0.9 },
      { part: "ink-seal", x: 16, y: 72 },
    ],
  },
};

/** Plate Packer (Meditative #73, Common Unit), Alt art: close-up of the weightlifter's straining face at a rainy outdoor market, a stack of armour plates on his shoulders, grey-green downpour. */
const PLATE_PACKER_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["grey-500", "green-700"],
  light: { x: 50, y: 30, r: 36, paint: "slate-100", opacity: 0.3 },
  emblem: { x: 50, y: 68, size: 5, rotate: 0 },
  figure: { x: 50, y: 116, scale: 1.8, pose: "pose-lift", look: meditativeLookOf("meditative-073") },
  layers: {
    far: [
      { part: "awning", x: 16, y: 64, scale: 1.2, paint: "jade-400" },
      { part: "awning", x: 84, y: 62, scale: 1.1, flip: -1, paint: "red-600" },
    ],
    mid: [{ part: "cobbles", x: 50, y: 66 }],
    near: [
      { part: "barbell", x: 50, y: 32, scale: 1.4, paint: "slate-700" },
      { part: "weights", x: 18, y: 96, scale: 1.2 },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.4, paint: "slate-100" },
      { part: "rain", x: 50, y: 72, scale: 1.2, paint: "slate-100" },
    ],
  },
};

/** A postman on his round, a sack over his shoulder. */
const POSTMAN: FigureLook = { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-coat", item: "item-sack", cloth: "blue-800" };

/** Montaña Giant (Meditative #74, Epic Unit), Alt art: close-up of the giant's mossy stone face at snowy dawn, a postman on its shoulder delivering a sack of unanswered envelopes, cold white and blue. */
const MONTANA_GIANT_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["white", "blue-200"],
  light: { x: 76, y: 30, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 70, size: 6, rotate: 0 },
  extras: [{ layer: "near", x: 76, y: 50, scale: 0.35, flip: -1, pose: "pose-hold", look: POSTMAN }],
  layers: {
    sky: [{ part: "snow", x: 50, y: 24, scale: 1.4 }],
    far: [
      { part: "ink-peaks", x: 50, y: 60, scale: 0.8, paint: "blue-800" },
      { part: "snowfield", x: 50, y: 70 },
    ],
    mid: [
      { part: "pine", x: 10, y: 86, scale: 1.4, paint: "slate-800" },
      { part: "pine", x: 90, y: 88, scale: 1.2, flip: -1, paint: "slate-800" },
    ],
    subject: [{ part: "rock-giant", x: 50, y: 128, scale: 2.4 }],
    near: [
      { part: "envelope", x: 30, y: 46, scale: 1, count: 3, spread: [10, 8] },
      { part: "ink-seal", x: 16, y: 76 },
    ],
    particles: [{ part: "snow", x: 50, y: 50, scale: 1.4 }],
  },
};

/** Ever Growing Tree (Meditative #75, Legendary Field Spell), Alt art: close-up: a seedling in a clay pot on a scholar's windowsill, already wrapping a tiny shrine and a toy teahouse in its roots, pink morning light. */
const EVER_GROWING_TREE_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["red-300", "white"],
  light: { x: 74, y: 28, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 74, size: 6, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 74, y: 26, scale: 0.8 }],
    far: [{ part: "round-window", x: 50, y: 44, scale: 4 }],
    mid: [{ part: "wide-table", x: 50, y: 100, scale: 3.2, paint: "wood-700" }],
    subject: [
      { part: "great-tree", x: 50, y: 64, scale: 0.42 },
      { part: "tiled-house", x: 41, y: 64, scale: 0.15 },
      { part: "pagoda", x: 59, y: 64, scale: 0.22 },
      { part: "vine", x: 50, y: 70, scale: 0.6, paint: "wood-700" },
    ],
    near: [
      { part: "jar", x: 50, y: 84, scale: 2.8, paint: "red-400" },
      { part: "brush", x: 80, y: 86, scale: 0.6 },
      { part: "closed-tome", x: 20, y: 90, scale: 1.2, count: 2, spread: [2, 3] },
      { part: "teapot", x: 82, y: 96, scale: 0.8 },
      { part: "ink-seal", x: 16, y: 74 },
    ],
    light: [
      { part: "light-shaft", x: 74, y: 14, scale: 1, paint: "red-300" },
      { part: "halo", x: 50, y: 50, scale: 1.2, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 50, y: 46, scale: 1, paint: "white" },
      { part: "petals", x: 30, y: 40, scale: 1, paint: "red-300" },
    ],
  },
};

/** Occidentless Mandate (Meditative #78, Epic Spell), Alt art: after the edict, a wide grey dawn harbour: foreign ships sailing off into fog, only red-sailed junks left at the quay. */
const OCCIDENTLESS_MANDATE_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["grey-500", "slate-400"],
  light: { x: 30, y: 40, r: 40, paint: "slate-100", opacity: 0.35 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "sea", x: 50, y: 54 },
      { part: "ship", x: 30, y: 56, scale: 0.6, fade: 0.5 },
      { part: "ship", x: 66, y: 55, scale: 0.5, flip: -1, fade: 0.5 },
      { part: "mist", x: 50, y: 52, scale: 1.4 },
    ],
    mid: [
      { part: "boat", x: 24, y: 74, scale: 0.8, paint: "red-600" },
      { part: "boat", x: 52, y: 76, scale: 0.7, flip: -1, paint: "red-600" },
      { part: "boat", x: 78, y: 74, scale: 0.75, paint: "red-600" },
    ],
    subject: [{ part: "gust", x: 40, y: 40, scale: 1.4 }],
    near: [
      { part: "boards", x: 50, y: 86 },
      { part: "ink-seal", x: 84, y: 74 },
    ],
    particles: [{ part: "mist", x: 50, y: 66, scale: 1 }],
  },
};

/** Touched by KY (Meditative #79, Legendary Unit), Alt art: the next morning, low angle: the student on a crowded sunrise bus, his fan of cards wider than the aisle, passengers ducking, warm orange light. */
const TOUCHED_BY_KY_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["orange-400", "red-300"],
  light: { x: 24, y: 40, r: 40, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  figure: { x: 50, y: 114, scale: 1.7, pose: "pose-hold", look: meditativeLookOf("meditative-079") },
  layers: {
    sky: [{ part: "sun", x: 24, y: 40, scale: 1 }],
    far: [{ part: "skyline", x: 50, y: 64, scale: 0.8, paint: "red-900" }],
    mid: [
      { part: "column", x: 6, y: 86, scale: 1.4, paint: "slate-700" },
      { part: "column", x: 94, y: 86, scale: 1.4, paint: "slate-700" },
      { part: "ink-person", x: 16, y: 84, scale: 2.2, count: 2, spread: [6, 2] },
      { part: "ink-person", x: 84, y: 84, scale: 2.2, flip: -1, count: 2, spread: [6, 2] },
    ],
    near: [
      { part: "ink-card-fan", x: 50, y: 62, scale: 2.6 },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    light: [
      { part: "light-shaft", x: 24, y: 20, scale: 1, paint: "orange-400" },
      { part: "halo", x: 50, y: 50, scale: 1.6, paint: "orange-400" },
    ],
    particles: [
      { part: "motes", x: 50, y: 40, scale: 1.2, paint: "white" },
      { part: "sparks", x: 50, y: 46, scale: 1, count: 2, spread: [14, 4] },
    ],
  },
};

/** Aluneth (Meditative #80, Legendary Field Spell), Alt art: summer, wide shot: the eternal lantern on a sunny lakeside reading pier, three books flapping like birds toward a swimmer's outstretched hand, green willows. */
const ALUNETH_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "green-400"],
  light: { x: 80, y: 20, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 30, y: 70, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 80, y: 20, scale: 0.8 },
      { part: "cloud", x: 26, y: 24, scale: 1, paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 50, paint: "green-700" },
      { part: "still-water", x: 50, y: 54 },
    ],
    mid: [
      { part: "tree", x: 12, y: 60, scale: 1.4, paint: "green-500" },
      { part: "tree", x: 88, y: 58, scale: 1.3, flip: -1, paint: "green-500" },
      { part: "walkway", x: 30, y: 80, scale: 1.2 },
      { part: "reed", x: 86, y: 82, scale: 1.4, count: 2, spread: [4, 2] },
    ],
    subject: [
      { part: "lantern", x: 30, y: 60, scale: 1.2 },
      { part: "closed-tome", x: 54, y: 44, scale: 0.6, count: 3, spread: [10, 5] },
    ],
    near: [
      { part: "hand-open", x: 76, y: 72, scale: 1.4, flip: -1 },
      { part: "ripples", x: 76, y: 80, scale: 0.8, paint: "white" },
      { part: "ink-seal", x: 16, y: 74 },
    ],
    light: [
      { part: "halo", x: 30, y: 64, scale: 1.2, paint: "white" },
      { part: "light-shaft", x: 80, y: 14, scale: 0.9, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 50, y: 46, scale: 1, paint: "white" },
      { part: "leaves", x: 20, y: 50, scale: 1, paint: "green-500" },
    ],
  },
};

/** Deadman's Hand (Meditative #81, Rare Spell), Alt art: from above, close: the dealer's skeletal hands shuffling translucent cards on a green-felt table on a riverboat deck in bright daylight. */
const DEADMANS_HAND_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-400", "green-700"],
  light: { x: 50, y: 60, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 52, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "water", x: 50, y: 20, paint: "blue-600" }],
    mid: [{ part: "boards", x: 50, y: 34 }],
    subject: [
      { part: "wide-table", x: 50, y: 104, scale: 4.4, paint: "green-700" },
      { part: "deck", x: 50, y: 74, scale: 1.6 },
    ],
    near: [
      { part: "bone-hand", x: 30, y: 84, scale: 1.4 },
      { part: "bone-hand", x: 70, y: 84, scale: 1.4, flip: -1 },
      { part: "playing-card", x: 50, y: 64, scale: 0.9, count: 4, spread: [16, 4], fade: 0.5 },
      { part: "ink-seal", x: 84, y: 74 },
    ],
    light: [{ part: "halo", x: 50, y: 66, scale: 1.4, paint: "white" }],
  },
};

/** Medina Enforcer (Meditative #83, Rare Unit), Alt art: night on the medina rooftops: the enforcer polishing a merchant's cards beneath a crescent moon, indigo sky over lamplit domes, wide shot. */
const MEDINA_ENFORCER_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["blue-900", "violet-700"],
  light: { x: 78, y: 24, r: 34, paint: "blue-200", opacity: 0.3 },
  emblem: { x: 26, y: 46, size: 5, rotate: 0 },
  figure: { x: 24, y: 57, scale: 0.5, pose: "pose-hold", look: meditativeLookOf("meditative-083") },
  layers: {
    sky: [
      { part: "moon", x: 78, y: 24, scale: 0.7 },
      { part: "stars", x: 40, y: 24, scale: 1.2 },
    ],
    far: [
      { part: "rotunda", x: 56, y: 64, scale: 0.5, paint: "slate-400" },
      { part: "whitewashed-house", x: 84, y: 62, scale: 0.8, paint: "slate-400" },
    ],
    mid: [
      { part: "whitewashed-house", x: 20, y: 80, scale: 1.2, paint: "slate-400" },
      { part: "whitewashed-house", x: 66, y: 84, scale: 1.4, paint: "slate-400" },
      { part: "lantern-string", x: 50, y: 46, scale: 1.2 },
    ],
    subject: [{ part: "playing-card", x: 32, y: 50, scale: 0.4 }],
    near: [
      { part: "lantern", x: 88, y: 66, scale: 0.8 },
      { part: "ink-seal", x: 16, y: 74 },
    ],
    light: [{ part: "halo", x: 88, y: 70, scale: 1, paint: "orange-400" }],
  },
};

/** Mayor Medinamogger (Meditative #86, Legendary Unit), Alt art: close-up portrait on election day in a rainy town hall, the mayor's square jaw lit by camera flashes as he catches the coin, cool grey-blue. */
const MAYOR_MEDINAMOGGER_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "blue-200"],
  light: { x: 18, y: 50, r: 34, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  figure: { x: 50, y: 116, scale: 1.8, pose: "pose-raise", look: meditativeLookOf("meditative-086") },
  layers: {
    sky: [{ part: "rain", x: 50, y: 24, scale: 1.4, paint: "slate-100" }],
    far: [
      { part: "room-wall", x: 50, y: 70, paint: "slate-400" },
      { part: "window", x: 26, y: 50, scale: 1.2 },
      { part: "window", x: 74, y: 50, scale: 1.2 },
      { part: "column", x: 10, y: 86, scale: 1.6, paint: "slate-100" },
      { part: "column", x: 90, y: 86, scale: 1.6, paint: "slate-100" },
    ],
    mid: [
      { part: "bunting", x: 50, y: 30, scale: 1.4 },
      { part: "crowd", x: 50, y: 100, scale: 1 },
    ],
    near: [
      { part: "burst", x: 18, y: 50, scale: 0.8, paint: "white" },
      { part: "burst", x: 84, y: 40, scale: 0.7, paint: "white" },
      { part: "coin", x: 70, y: 20, scale: 1.4 },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    light: [
      { part: "halo", x: 18, y: 50, scale: 1.2, paint: "white" },
      { part: "rim-light", x: 50, y: 40, scale: 1.4, paint: "blue-200" },
    ],
    particles: [
      { part: "rain", x: 50, y: 60, scale: 1.4, paint: "slate-100" },
      { part: "sparks", x: 70, y: 22, scale: 1 },
    ],
  },
};

/** Tatches the Totem (Meditative #87, Legendary Unit), Alt art: before it rises: the totem buried in spring snowmelt in a river valley, only its top face showing as travellers of every tribe step past, pale green. */
const TATCHES_THE_TOTEM_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["green-200", "white"],
  light: { x: 50, y: 40, r: 36, paint: "jade-400", opacity: 0.35 },
  emblem: { x: 50, y: 44, size: 5, rotate: 0 },
  extras: [
    { layer: "near", x: 20, y: 76, scale: 0.4, pose: "pose-stride", look: meditativeExtraLookOf("meditative-087", 0) },
    { layer: "near", x: 78, y: 78, scale: 0.4, flip: -1, pose: "pose-stride", look: meditativeExtraLookOf("meditative-087", 1) },
    { layer: "near", x: 88, y: 84, scale: 0.45, flip: -1, pose: "pose-stride", look: figureLookFor(["Jlockeed"]) },
  ],
  layers: {
    sky: [{ part: "ink-cloud", x: 24, y: 22, scale: 0.9, paint: "white" }],
    far: [
      { part: "ink-peaks", x: 50, y: 46, scale: 0.7, paint: "green-700" },
      { part: "field", x: 50, y: 52, paint: "green-400" },
    ],
    mid: [{ part: "water", x: 50, y: 58, paint: "blue-200" }],
    subject: [{ part: "totem", x: 50, y: 98, scale: 1.4 }],
    near: [
      { part: "snowfield", x: 50, y: 58 },
      { part: "ripples", x: 50, y: 60, scale: 1, paint: "blue-200" },
      { part: "pebbles", x: 50, y: 70, scale: 1.4, props: true },
      { part: "drops", x: 50, y: 52, scale: 1, paint: "blue-200", props: true },
      { part: "ink-seal", x: 16, y: 72 },
    ],
    light: [{ part: "light-shaft", x: 50, y: 8, scale: 1, paint: "white" }],
    particles: [
      { part: "motes", x: 50, y: 40, scale: 1, paint: "white" },
      { part: "sparks", x: 50, y: 40, scale: 1, count: 2, spread: [10, 4], paint: "jade-400" },
    ],
  },
};

/** Spell Basket (Meditative #90, Common Spell), Alt art: close-up on a rainy porch: the basket tipped over, half-built spells crawling out like crabs as a cat bats at one, soft grey-blue. */
const SPELL_BASKET_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "blue-200"],
  light: { x: 50, y: 30, r: 34, paint: "slate-100", opacity: 0.35 },
  emblem: { x: 42, y: 74, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "hills", x: 50, y: 52, paint: "slate-700" },
      { part: "field", x: 50, y: 58, paint: "green-700" },
    ],
    mid: [
      { part: "boards", x: 50, y: 66 },
      { part: "column", x: 8, y: 82, scale: 1.4, paint: "wood-700" },
      { part: "column", x: 92, y: 82, scale: 1.4, paint: "wood-700" },
    ],
    subject: [
      { part: "basket", x: 42, y: 88, scale: 2.2 },
      { part: "ink-imp", x: 66, y: 88, scale: 1.2, count: 3, spread: [10, 4], paint: "jade-400" },
    ],
    near: [
      { part: "house-cat", x: 84, y: 94, scale: 1.8, flip: -1 },
      { part: "ink-seal", x: 16, y: 72 },
    ],
    particles: [{ part: "rain", x: 50, y: 30, scale: 1.4, paint: "slate-100" }],
  },
};

/** Windfast (Meditative #91, Epic Unit), Alt art: close-up portrait: the swordsman in a windless autumn temple courtyard, red leaves frozen mid-air around him, his eyes just opening. */
const WINDFAST_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["red-500", "orange-400"],
  light: { x: 50, y: 30, r: 40, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 68, size: 5, rotate: 0 },
  figure: { x: 50, y: 116, scale: 1.8, pose: "pose-guard", look: meditativeLookOf("meditative-091") },
  layers: {
    far: [{ part: "temple", x: 50, y: 66, scale: 1.4, paint: "red-900" }],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "stone-500" },
      { part: "tree", x: 12, y: 70, scale: 1.4, paint: "red-600" },
    ],
    near: [
      { part: "leaves", x: 22, y: 40, scale: 1.2, paint: "red-600" },
      { part: "leaves", x: 78, y: 46, scale: 1.2, paint: "red-600" },
      { part: "leaves", x: 50, y: 24, scale: 1, paint: "orange-400" },
      { part: "ink-seal", x: 84, y: 76 },
    ],
  },
};

/** Shrinking Felinor (Meditative #94, Rare Unit), Alt art: top-down on a sunny beach: the cat-folk's paw prints shrinking step by step toward a tiny kitten at the turquoise water's edge. */
const SHRINKING_FELINOR_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-400", "cyan-400"],
  light: { x: 70, y: 30, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 46, y: 70, size: 5, rotate: 0 },
  figure: { x: 52, y: 42, scale: 0.3, pose: "pose-stand", look: meditativeLookOf("meditative-094") },
  layers: {
    far: [{ part: "sea", x: 50, y: 16, paint: "cyan-400" }],
    mid: [
      { part: "sand", x: 50, y: 40 },
      { part: "ripples", x: 50, y: 40, scale: 1.2, paint: "white" },
    ],
    subject: [
      { part: "paw-prints", x: 40, y: 90, scale: 1.2 },
      { part: "paw-prints", x: 46, y: 76, scale: 0.9 },
      { part: "paw-prints", x: 50, y: 64, scale: 0.7 },
      { part: "paw-prints", x: 52, y: 54, scale: 0.55 },
      { part: "paw-prints", x: 52, y: 47, scale: 0.4 },
    ],
    near: [
      { part: "pebbles", x: 80, y: 92, scale: 1 },
      { part: "ink-seal", x: 16, y: 72 },
    ],
  },
};

/** Call to Chaos (Meditative Edition) (Meditative #95, Legendary Spell), Alt art: before: the zen garden perfectly raked under cherry blossom at sunrise, the monk calm, a tiny jade die rolling unseen toward the gravel, soft pink, wide shot. */
const CALL_TO_CHAOS_MEDITATIVE_EDITION_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["red-300", "white"],
  light: { x: 24, y: 34, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 30, y: 62, size: 5, rotate: 0 },
  extras: [{ layer: "mid", x: 70, y: 66, scale: 0.4, pose: "pose-sit", look: meditativeExtraLookOf("meditative-095", 2) }],
  layers: {
    sky: [{ part: "sun", x: 24, y: 34, scale: 0.9 }],
    far: [
      { part: "ink-peaks", x: 50, y: 52, scale: 0.6, paint: "red-300" },
      { part: "pagoda", x: 84, y: 58, scale: 0.6 },
    ],
    mid: [{ part: "raked-gravel", x: 50, y: 60 }],
    subject: [{ part: "jade-die", x: 30, y: 74, scale: 0.5 }],
    near: [
      { part: "blossom-branch", x: 4, y: 26, scale: 1.4 },
      { part: "petals", x: 60, y: 40, scale: 1.2 },
      { part: "stone-lantern", x: 88, y: 94, scale: 1 },
      { part: "pebbles", x: 46, y: 94 },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    light: [
      { part: "halo", x: 24, y: 34, scale: 1.4, paint: "white" },
      { part: "light-shaft", x: 24, y: 20, scale: 0.8, paint: "red-300" },
    ],
    particles: [
      { part: "petals", x: 50, y: 60, scale: 1, paint: "red-300" },
      { part: "motes", x: 50, y: 50, scale: 1, paint: "white" },
    ],
  },
};

/** Meditative Journey (Meditative #96, Rare Spell), Alt art: wide vista: an autumn river delta at sunset, tiny lotus-leaf boats carrying cards out toward a red sea, the pilgrim small on a hill. */
const MEDITATIVE_JOURNEY_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["orange-400", "red-600"],
  light: { x: 40, y: 44, r: 40, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 40, y: 64, size: 5, rotate: 0 },
  figure: { x: 82, y: 64, scale: 0.35, flip: -1, pose: "pose-stride", look: meditativeLookOf("meditative-096") },
  layers: {
    sky: [{ part: "sun", x: 40, y: 44, scale: 1.2 }],
    far: [{ part: "sea", x: 50, y: 48, paint: "red-500" }],
    mid: [
      { part: "field", x: 50, y: 58, paint: "wood-600" },
      { part: "water", x: 46, y: 66, paint: "red-300" },
      { part: "cliff", x: 84, y: 64, scale: 1, flip: -1, paint: "wood-700" },
    ],
    subject: [
      { part: "lotus", x: 40, y: 76, scale: 0.7, count: 4, spread: [16, 6] },
      { part: "playing-card", x: 40, y: 72, scale: 0.3, count: 4, spread: [16, 6] },
    ],
    near: [
      { part: "reed", x: 10, y: 96, scale: 1.6, count: 2, spread: [4, 2] },
      { part: "ink-seal", x: 16, y: 72 },
    ],
  },
};

/** A worker in a gunmetal hard hat. */
const WORKER: FigureLook = { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-work-apron", item: null, cloth: "slate-700" };

/** Jlockeed's Evil Blueprints (Meditative #97, Mythic Spell), Alt art: daytime on a dusty mountain slope: cranes raising the blueprint's buildings out of cyan outlines, workers in gunmetal hard hats, orange earth, wide shot. */
const JLOCKEEDS_EVIL_BLUEPRINTS_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "orange-400"],
  light: { x: 80, y: 20, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 80, size: 5, rotate: 0 },
  extras: [
    { layer: "mid", x: 32, y: 76, scale: 0.35, pose: "pose-lift", look: WORKER },
    { layer: "mid", x: 68, y: 78, scale: 0.35, flip: -1, pose: "pose-point", look: WORKER },
  ],
  layers: {
    sky: [
      { part: "sun", x: 80, y: 18, scale: 0.8 },
      { part: "cloud", x: 24, y: 22, scale: 1, paint: "white" },
    ],
    far: [
      { part: "mountains", x: 50, y: 50, paint: "wood-600" },
      { part: "mesa", x: 82, y: 60, scale: 0.8, paint: "red-900" },
    ],
    mid: [
      { part: "sand", x: 50, y: 62, paint: "orange-400" },
      { part: "tower-crane", x: 22, y: 74, scale: 1 },
      { part: "tower-crane", x: 80, y: 76, scale: 0.9, flip: -1 },
      { part: "schoolhouse", x: 40, y: 72, scale: 0.4, paint: "cyan-400", fade: 0.6 },
      { part: "glass-church", x: 62, y: 72, scale: 0.4, paint: "cyan-400", fade: 0.6 },
    ],
    subject: [{ part: "blueprint", x: 50, y: 92, scale: 0.9, paint: "cyan-400" }],
    near: [
      { part: "pebbles", x: 16, y: 96 },
      { part: "ink-seal", x: 84, y: 74 },
    ],
    light: [
      { part: "god-rays", x: 80, y: 18, scale: 1, paint: "white" },
      { part: "halo", x: 50, y: 72, scale: 1.6, paint: "cyan-400" },
    ],
    particles: [
      { part: "dust", x: 50, y: 70, scale: 1.4, paint: "orange-400" },
      { part: "sparks", x: 24, y: 40, scale: 1, count: 2, spread: [4, 4] },
      { part: "motes", x: 50, y: 50, scale: 1, paint: "white" },
    ],
  },
};

/** Greaser (Meditative #100, Common Unit), Alt art: rainy night under a lone streetlamp, close-up: the greaser's reflection in a puddle as he combs his hair, deep blue and silver. */
const GREASER_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["blue-900", "slate-700"],
  light: { x: 80, y: 34, r: 34, paint: "slate-100", opacity: 0.35 },
  emblem: { x: 46, y: 66, size: 5, rotate: 0 },
  figure: { x: 46, y: 114, scale: 1.7, pose: "pose-raise", look: meditativeLookOf("meditative-100") },
  layers: {
    sky: [{ part: "rain", x: 50, y: 24, scale: 1.4, paint: "slate-400" }],
    far: [
      { part: "skyline", x: 50, y: 62, scale: 0.7, paint: "slate-900" },
      { part: "street", x: 50, y: 62 },
    ],
    mid: [{ part: "streetlamp", x: 80, y: 80, scale: 1.6 }],
    near: [
      { part: "still-water", x: 50, y: 88 },
      { part: "ripples", x: 30, y: 94, scale: 0.8, paint: "slate-100" },
      { part: "ink-seal", x: 16, y: 72 },
    ],
    light: [{ part: "halo", x: 80, y: 34, scale: 1.4, paint: "slate-100" }],
    particles: [{ part: "rain", x: 50, y: 60, scale: 1.4, paint: "slate-100" }],
  },
};

/** R3404000: the alt recipes of Meditative #72 to #100, by card id. */
export const MEDITATIVE_ALT_RECIPES_3: Readonly<Record<string, SceneRecipe>> = {
  "meditative-072": THE_BANISHER_ALT,
  "meditative-073": PLATE_PACKER_ALT,
  "meditative-074": MONTANA_GIANT_ALT,
  "meditative-075": EVER_GROWING_TREE_ALT,
  "meditative-078": OCCIDENTLESS_MANDATE_ALT,
  "meditative-079": TOUCHED_BY_KY_ALT,
  "meditative-080": ALUNETH_ALT,
  "meditative-081": DEADMANS_HAND_ALT,
  "meditative-083": MEDINA_ENFORCER_ALT,
  "meditative-086": MAYOR_MEDINAMOGGER_ALT,
  "meditative-087": TATCHES_THE_TOTEM_ALT,
  "meditative-090": SPELL_BASKET_ALT,
  "meditative-091": WINDFAST_ALT,
  "meditative-094": SHRINKING_FELINOR_ALT,
  "meditative-095": CALL_TO_CHAOS_MEDITATIVE_EDITION_ALT,
  "meditative-096": MEDITATIVE_JOURNEY_ALT,
  "meditative-097": JLOCKEEDS_EVIL_BLUEPRINTS_ALT,
  "meditative-100": GREASER_ALT,
};
