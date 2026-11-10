// R3388000: Classic's alt scene recipes, part 3 of three (#55 to #90), one per card offered Alt art, each drawn from its
// brief (#3388's table; ART.md, "Classic's Alt art restaged"): the card's own subject in another place, time or moment,
// under a sky sharing no colour with its normal picture's and a new camera (a close-up, a wide or distant view, or one
// from above), its figure the normal picture's person (classicAltRecipes1.ts `normalClassicLookOf`), in Classic's
// temple-stone look and at or above its rarity's floor. scene.ts `sceneRecipeOf` reads one under the card's Alt art seed
// alone, at every Alt art level. Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { normalClassicExtraLookOf, normalClassicLookOf } from "./classicAltRecipes1.ts";
import { figureLookFor } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** Book of Wildfire (Classic #55, Epic), Alt art: After the fire: the blackened tome cools among new green shoots on an ash-grey hillside in drizzle; grey and fresh green. */
const BOOK_OF_WILDFIRE_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-400", "slate-100"],
  light: { x: 30, y: 22, r: 40, paint: "white", opacity: 0.3 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 40, y: 22, scale: 1.4, count: 3, spread: [30, 4], paint: "slate-700" }],
    far: [
      { part: "hills", x: 50, y: 48, paint: "slate-700" },
      { part: "dead-tree", x: 14, y: 54, scale: 1, paint: "slate-800" },
      { part: "dead-tree", x: 86, y: 53, scale: 0.8, flip: -1, paint: "slate-800" },
    ],
    mid: [
      { part: "field", x: 50, y: 58, paint: "stone-500" },
      { part: "log", x: 22, y: 70, scale: 1, paint: "slate-900" },
      { part: "smoke", x: 76, y: 62, scale: 0.9, paint: "slate-400" },
      { part: "sprout", x: 78, y: 70, scale: 1.4, count: 3, spread: [10, 2], paint: "green-400" },
    ],
    subject: [
      { part: "tome", x: 50, y: 70, scale: 1.6, paint: "slate-800" },
      { part: "sprout", x: 50, y: 76, scale: 1.8, count: 6, spread: [22, 4], paint: "green-400" },
      { part: "steam", x: 50, y: 62, scale: 0.8, paint: "slate-100" },
    ],
    near: [
      { part: "sprout", x: 14, y: 94, scale: 2.2, count: 3, spread: [8, 2], paint: "green-400" },
      { part: "grass-tuft", x: 86, y: 96, count: 3, spread: [8, 2], paint: "green-500" },
      { part: "pebbles", x: 60, y: 96, paint: "slate-700" },
    ],
    light: [{ part: "rim-light", x: 50, y: 62, scale: 1.2, paint: "white" }],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.4, paint: "slate-100" },
      { part: "rain", x: 50, y: 74, scale: 1.2, paint: "slate-100" },
    ],
  },
};

/** Common Resources (Classic #58, Common), Alt art: Winter commons: two rival farmers share a frozen well, the rope hauling up a frosted card, breath fogging; snowy white and blue. Stand-ins: `steam` for their fogging breath. */
const COMMON_RESOURCES_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-100", "slate-400"],
  light: { x: 50, y: 20, r: 40, paint: "white", opacity: 0.35 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  figure: { x: 32, y: 80, scale: 0.6, pose: "pose-hold", look: { ...normalClassicLookOf("classic-058") } },
  extras: [{ layer: "subject", x: 68, y: 80, scale: 0.6, flip: -1, pose: "pose-hold", look: { ...normalClassicExtraLookOf("classic-058", 0) } }],
  layers: {
    sky: [{ part: "cloud", x: 30, y: 22, scale: 1.2, count: 2, spread: [30, 4], paint: "white" }],
    far: [
      { part: "hills", x: 50, y: 54, paint: "blue-200" },
      { part: "barn", x: 14, y: 60, scale: 0.6, paint: "blue-700" },
      { part: "pine", x: 86, y: 60, scale: 0.9, count: 3, spread: [8, 1], paint: "blue-800" },
    ],
    mid: [
      { part: "snowfield", x: 50, y: 62 },
      { part: "fence", x: 14, y: 70, scale: 0.7, paint: "blue-800" },
    ],
    subject: [
      { part: "rope", x: 50, y: 60, scale: 0.5 },
      { part: "well", x: 50, y: 80, scale: 1.2, paint: "slate-400" },
      { part: "card", x: 50, y: 69, scale: 0.9, paint: "white" },
      { part: "frost", x: 50, y: 72.4, scale: 0.4 },
    ],
    near: [
      { part: "steam", x: 38, y: 50, scale: 0.9, paint: "white" },
      { part: "steam", x: 62, y: 50, scale: 0.9, paint: "white" },
      { part: "bucket", x: 82, y: 94, scale: 1.1, paint: "blue-700" },
      { part: "snow", x: 50, y: 92, scale: 1.2 },
    ],
    light: [{ part: "halo", x: 50, y: 66, scale: 0.8, paint: "white" }],
    particles: [{ part: "snow", x: 50, y: 40, scale: 1.4 }],
  },
};

/** Pile On (Classic #60, Epic), Alt art: Bird's-eye view of the heap of warriors and banners piled on a harbour pier at dawn, boats crowding round; pink sky and sea green. Stand-ins: `boards` for the pier. */
const PILE_ON_ALT: SceneRecipe = {
  family: "classic",
  sky: ["red-300", "violet-300"],
  light: { x: 80, y: 20, r: 36, paint: "red-300", opacity: 0.4 },
  emblem: { x: 84, y: 74, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 40, y: 72, scale: 0.42, pose: "pose-spread", look: { ...normalClassicExtraLookOf("classic-060", 0) } },
    { layer: "subject", x: 60, y: 70, scale: 0.44, flip: -1, pose: "pose-run", look: { ...normalClassicExtraLookOf("classic-060", 1) } },
  ],
  layers: {
    sky: [{ part: "sun-disc", x: 80, y: 18, scale: 0.9 }],
    far: [{ part: "sea", x: 50, y: 28, paint: "jade-400" }],
    mid: [
      { part: "boards", x: 50, y: 62, paint: "wood-600" },
      { part: "boat", x: 14, y: 50, scale: 0.8 },
      { part: "boat", x: 34, y: 40, scale: 0.55, flip: -1 },
      { part: "boat", x: 68, y: 40, scale: 0.55 },
      { part: "boat", x: 88, y: 52, scale: 0.8, flip: -1 },
    ],
    subject: [
      { part: "banner", x: 50, y: 62, scale: 1.6, paint: "red-600" },
      { part: "banner", x: 34, y: 70, scale: 1.1, paint: "blue-700" },
      { part: "banner", x: 66, y: 72, scale: 1.1, flip: -1, paint: "violet-700" },
      { part: "soldier-fallen", x: 50, y: 82, scale: 1.2, count: 4, spread: [16, 2] },
      { part: "soldier-fallen", x: 50, y: 76, scale: 1, count: 3, spread: [12, 2], flip: -1 },
      { part: "soldier", x: 50, y: 72, scale: 0.8, count: 2, spread: [8, 1] },
    ],
    near: [{ part: "crate", x: 12, y: 94, scale: 1.2 }],
    light: [{ part: "rim-light", x: 50, y: 64, scale: 1.4, paint: "red-300" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" }],
  },
};

/** Plague Bringer Goliath (Classic #61, Rare), Alt art: Close-up of the Goliath's helm in a cold mountain pass, a sneeze building, icicles shaking off its visor; ice blue and grey. Stand-ins: `ice-shard`s for the icicles. */
const PLAGUE_BRINGER_GOLIATH_ALT: SceneRecipe = {
  family: "classic",
  sky: ["blue-200", "slate-400"],
  light: { x: 50, y: 30, r: 40, paint: "white", opacity: 0.35 },
  emblem: { x: 82, y: 76, size: 5, rotate: 0 },
  figure: { x: 50, y: 124, scale: 1.8, pose: "pose-stand", look: { ...normalClassicLookOf("classic-061") } },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 22, scale: 1.2, count: 2, spread: [30, 4], paint: "white" }],
    far: [
      { part: "mountains", x: 50, y: 44, paint: "slate-700" },
      { part: "snowfield", x: 50, y: 64 },
    ],
    mid: [
      { part: "cliff", x: 6, y: 30, paint: "slate-700" },
      { part: "cliff", x: 94, y: 30, flip: -1, paint: "slate-700" },
    ],
    subject: [{ part: "mist", x: 50, y: 70, scale: 1.4, paint: "white" }],
    near: [
      { part: "ice-shard", x: 38, y: 34, scale: 0.7, flip: -1, paint: "blue-200" },
      { part: "ice-shard", x: 62, y: 32, scale: 0.7, paint: "blue-200" },
      { part: "ice-shard", x: 32, y: 42, scale: 0.55, flip: -1, paint: "blue-200" },
      { part: "ice-shard", x: 68, y: 44, scale: 0.55, paint: "blue-200" },
      { part: "snow", x: 50, y: 90, scale: 1.4 },
    ],
    light: [{ part: "rim-light", x: 50, y: 40, scale: 1.4, paint: "white" }],
    particles: [
      { part: "snow", x: 50, y: 34, scale: 1.4 },
      { part: "spray", x: 50, y: 40, scale: 0.6, paint: "lime-200" },
    ],
  },
};

/** Living Bomb (Classic #62, Rare), Alt art: A river barge at night carries the throbbing fever-bomb past sleeping villages, its veins glowing on the water; deep blue and green. Stand-ins: a `boat` for the barge. */
const LIVING_BOMB_ALT: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "slate-900"],
  light: { x: 50, y: 60, r: 34, paint: "green-400", opacity: 0.35 },
  emblem: { x: 20, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 16, scale: 1.2 },
      { part: "moon", x: 80, y: 22, scale: 0.6 },
    ],
    far: [
      { part: "hills", x: 50, y: 50, paint: "slate-800" },
      { part: "house", x: 14, y: 58, scale: 0.5, paint: "slate-800" },
      { part: "house", x: 28, y: 57, scale: 0.4, paint: "slate-800" },
      { part: "house", x: 84, y: 58, scale: 0.5, flip: -1, paint: "slate-800" },
    ],
    mid: [
      { part: "water", x: 50, y: 60, paint: "blue-800" },
      { part: "boat", x: 50, y: 74, scale: 1.4, paint: "wood-700" },
    ],
    subject: [
      { part: "orb", x: 50, y: 64, scale: 1.4, paint: "green-700" },
      { part: "halo", x: 50, y: 84, scale: 0.8, paint: "green-400" },
    ],
    near: [
      { part: "ripples", x: 50, y: 84, scale: 1, paint: "green-400" },
      { part: "reed", x: 8, y: 94, scale: 1.6, count: 3, spread: [5, 1] },
    ],
    light: [
      { part: "halo", x: 50, y: 62, scale: 1.4, paint: "green-400" },
      { part: "rim-light", x: 50, y: 60, scale: 1, paint: "lime-400" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "green-200" }],
  },
};

/** Malzahar's Recycler (Classic #64, Rare), Alt art: The recycler half-buried in a sunny scrapyard of broken stone golems, cards whirling in its hopper; rust orange and sky blue. Stand-ins: a `statue` and a `boulder` for the broken golems, the `glow-chute` for its hopper. */
const MALZAHARS_RECYCLER_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "sky-300"],
  light: { x: 80, y: 18, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 80, y: 40, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun-disc", x: 82, y: 18, scale: 1 },
      { part: "cloud", x: 24, y: 22, scale: 1, paint: "white" },
    ],
    far: [{ part: "hills", x: 50, y: 58, paint: "wood-700" }],
    mid: [
      { part: "sand", x: 50, y: 64, paint: "wood-600" },
      { part: "statue", x: 72, y: 72, scale: 1.2, paint: "stone-500" },
      { part: "boulder", x: 86, y: 78, scale: 0.6, paint: "stone-500" },
      { part: "fallen-column", x: 16, y: 74, scale: 0.8, paint: "stone-500" },
    ],
    subject: [
      { part: "forge", x: 40, y: 70, scale: 1.2, paint: "red-900" },
      { part: "glow-chute", x: 40, y: 38, scale: 1, paint: "orange-400" },
      { part: "card-ring", x: 40, y: 34, scale: 0.8 },
    ],
    near: [
      { part: "scrap-heap", x: 40, y: 72, paint: "ember-500" },
      { part: "cog", x: 80, y: 92, scale: 1.4, paint: "red-900" },
    ],
    light: [{ part: "rim-light", x: 40, y: 50, scale: 1.2, paint: "orange-400" }],
    particles: [{ part: "sparks", x: 40, y: 36, scale: 1.2, count: 2, spread: [12, 6], paint: "orange-400" }],
  },
};

/** Book of Plague (Classic #70, Epic), Alt art: The tome lies open in a sunny wildflower meadow as its plague moths rise in a cloud over a monk's picnic; sky blue and pale green. */
const BOOK_OF_PLAGUE_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "green-200"],
  light: { x: 78, y: 18, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 68, y: 78, scale: 0.5, flip: -1, pose: "pose-sit", look: { ...normalClassicExtraLookOf("classic-070", 0), hair: "tonsure" } },
  ],
  layers: {
    sky: [
      { part: "sun", x: 80, y: 18, scale: 0.8 },
      { part: "cloud", x: 34, y: 22, scale: 1, count: 2, spread: [20, 3], paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 54, paint: "green-500" },
      { part: "tree", x: 90, y: 60, scale: 1, paint: "green-700" },
      { part: "tree", x: 10, y: 58, scale: 0.8, paint: "green-700" },
    ],
    mid: [
      { part: "field", x: 50, y: 62, paint: "green-400" },
      { part: "flowers", x: 22, y: 70, count: 2, spread: [10, 2] },
      { part: "picnic-blanket", x: 68, y: 82, scale: 1 },
    ],
    subject: [
      { part: "tome", x: 40, y: 76, scale: 1.6, paint: "wood-700" },
      { part: "moth", x: 54, y: 52, scale: 1, count: 6, spread: [20, 12], paint: "green-200" },
      { part: "basket", x: 80, y: 80, scale: 0.8 },
    ],
    near: [
      { part: "flowers", x: 12, y: 96, scale: 2, count: 2, spread: [5, 1] },
      { part: "tall-grass", x: 90, y: 98, scale: 1.4 },
    ],
    light: [
      { part: "halo", x: 40, y: 70, scale: 1, paint: "green-200" },
      { part: "rim-light", x: 54, y: 50, scale: 1.2, paint: "white" },
    ],
    particles: [
      { part: "spore-motes", x: 54, y: 46, scale: 1.4, paint: "green-200" },
      { part: "petals", x: 30, y: 40, scale: 1.2 },
    ],
  },
};

/** Corpse Plantation (Classic #74, Epic), Alt art: Harvest noon at the plantation: farmhands in straw hats scythe stalks among grave-mounds under a white-hot sky, sacks of bones on a cart; parched tan and blue. Stand-ins: `hat-brim`s for the straw hats, `item-rake`s for the scythes. */
const CORPSE_PLANTATION_ALT: SceneRecipe = {
  family: "classic",
  sky: ["white", "sky-300"],
  light: { x: 50, y: 10, r: 46, paint: "white", opacity: 0.55 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 22, y: 80, scale: 0.46, pose: "pose-hold", look: { ...figureLookFor([]), hair: "hat-brim", outfit: "outfit-overalls", item: "item-rake", cloth: "blue-700" } },
    { layer: "subject", x: 72, y: 76, scale: 0.42, flip: -1, pose: "pose-hold", look: { ...figureLookFor([]), hair: "hat-brim", outfit: "outfit-tunic", item: "item-rake", cloth: "wood-600" } },
    { layer: "subject", x: 88, y: 84, scale: 0.5, flip: -1, pose: "pose-hold", look: { ...figureLookFor([]), hair: "hat-brim", outfit: "outfit-overalls", item: "item-rake", cloth: "blue-600" } },
  ],
  layers: {
    sky: [{ part: "sun", x: 50, y: 12, scale: 1 }],
    far: [
      { part: "hills", x: 50, y: 56, paint: "brass-500" },
    ],
    mid: [
      { part: "furrows", x: 50, y: 60, paint: "brass-600" },
      { part: "grave-mound", x: 30, y: 66, scale: 0.42 },
      { part: "grave-mound", x: 64, y: 66, scale: 0.4 },
      { part: "grave-mound", x: 14, y: 74, scale: 0.6 },
      { part: "wheat", x: 60, y: 70, count: 3, spread: [30, 2] },
    ],
    subject: [
      { part: "bone-scarecrow", x: 50, y: 74, scale: 0.8 },
      { part: "cart", x: 34, y: 80, scale: 1.1, paint: "wood-600" },
      { part: "sack", x: 30, y: 74, scale: 0.6, paint: "brass-500" },
      { part: "bones", x: 37, y: 74, scale: 0.6 },
    ],
    near: [
      { part: "wheat", x: 8, y: 98, scale: 1.8, count: 2, spread: [5, 1] },
      { part: "pebbles", x: 70, y: 96, paint: "brass-600" },
    ],
    light: [{ part: "god-rays", x: 50, y: 6, scale: 1, paint: "white" }],
  },
};

/** Argusland (Classic #75, Rare), Alt art: Close-up of one great stone eye in the fortress wall, an arrow splintering against its lid at sunset; deep orange and violet. */
const ARGUSLAND_ALT: SceneRecipe = {
  family: "classic",
  sky: ["orange-400", "violet-500"],
  light: { x: 50, y: 50, r: 36, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 80, y: 76, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun-disc", x: 82, y: 26, scale: 1 }],
    far: [
      { part: "tower", x: 14, y: 40, scale: 0.8, paint: "violet-700" },
      { part: "tower", x: 88, y: 38, scale: 0.6, flip: -1, paint: "violet-700" },
    ],
    mid: [
      { part: "stone-wall", x: 50, y: 52, scale: 2.4, paint: "stone-500" },
      { part: "stone-wall", x: 50, y: 64, scale: 2.4, paint: "slate-400" },
      { part: "stone-wall", x: 50, y: 76, scale: 2.4, paint: "stone-500" },
      { part: "stone-wall", x: 50, y: 88, scale: 2.4, paint: "slate-400" },
      { part: "stone-wall", x: 50, y: 100, scale: 2.4, paint: "stone-500" },
    ],
    subject: [
      { part: "eye", x: 52, y: 62, scale: 3 },
      { part: "splinters", x: 44, y: 55, scale: 0.8, paint: "wood-600" },
    ],
    near: [
      { part: "flying-arrow", x: 44, y: 55, scale: 1.2 },
      { part: "vine", x: 90, y: 50, scale: 1.2 },
    ],
    light: [{ part: "halo", x: 52, y: 62, scale: 1.4, paint: "cyan-400" }],
    particles: [{ part: "dust", x: 44, y: 56, scale: 1, paint: "stone-500" }],
  },
};

/** Plague Bringer (Classic #76, Rare), Alt art: The rider leaving at dawn over a frosty bridge, mule mid-sneeze, souvenir clutched to his chest; pale pink and frost white. Stand-ins: an `item-sack` for the souvenir. */
const PLAGUE_BRINGER_ALT: SceneRecipe = {
  family: "classic",
  sky: ["red-300", "slate-100"],
  light: { x: 20, y: 40, r: 40, paint: "red-300", opacity: 0.4 },
  emblem: { x: 80, y: 30, size: 5, rotate: 0 },
  figure: { x: 49, y: 58.6, scale: 0.4, pose: "pose-sit", look: { ...normalClassicLookOf("classic-076"), item: "item-sack" } },
  layers: {
    sky: [{ part: "sun-disc", x: 18, y: 46, scale: 1 }],
    far: [
      { part: "mountains", x: 50, y: 48, paint: "slate-400" },
      { part: "pine", x: 86, y: 60, scale: 0.8, count: 3, spread: [8, 1], paint: "slate-700" },
      { part: "mist", x: 50, y: 58, scale: 1.4, paint: "white" },
    ],
    mid: [
      { part: "water", x: 50, y: 68, paint: "blue-200" },
      { part: "bridge", x: 50, y: 66, scale: 1.2, paint: "slate-400" },
    ],
    subject: [
      { part: "mule", x: 50, y: 66, scale: 1, paint: "slate-400" },
      { part: "frost", x: 50, y: 66, scale: 1.6 },
    ],
    near: [
      { part: "snow", x: 50, y: 88, scale: 1.4 },
      { part: "reed", x: 10, y: 96, scale: 1.6, count: 2, spread: [5, 1], paint: "slate-700" },
    ],
    light: [{ part: "rim-light", x: 50, y: 54, scale: 1, paint: "white" }],
    particles: [{ part: "snow", x: 50, y: 40, scale: 1.4 }],
  },
};

/** BOOM! Big Max (Classic #80, Legendary), Alt art: Before the drop: Big Max crouched at the open hatch of a siege tower over a misty valley, fists clenched; cold steel blue. */
const BOOM_BIG_MAX_ALT: SceneRecipe = {
  family: "classic",
  sky: ["blue-800", "slate-400"],
  light: { x: 50, y: 50, r: 40, paint: "blue-200", opacity: 0.35 },
  emblem: { x: 20, y: 30, size: 6, rotate: 0 },
  figure: { x: 50, y: 80, scale: 0.9, pose: "pose-crouch", look: { ...normalClassicLookOf("classic-080") } },
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 22, scale: 1.2, count: 3, spread: [30, 4], paint: "slate-700" },
      { part: "moon", x: 80, y: 18, scale: 0.5 },
    ],
    far: [
      { part: "mountains", x: 50, y: 56, paint: "slate-700" },
      { part: "mist", x: 50, y: 62, scale: 2, count: 2, spread: [30, 3], paint: "slate-100" },
    ],
    mid: [
      { part: "hills", x: 50, y: 70, paint: "blue-900" },
      { part: "fortress", x: 22, y: 74, scale: 0.4, paint: "slate-700" },
      { part: "mist", x: 50, y: 76, scale: 1.6, paint: "slate-100" },
    ],
    subject: [
      { part: "siege-tower", x: 50, y: 206, scale: 4, paint: "wood-700" },
      { part: "hatch", x: 50, y: 86, scale: 1.3 },
    ],
    near: [
      { part: "banner", x: 31, y: 60, scale: 1, paint: "blue-700" },
      { part: "banner", x: 69, y: 60, scale: 1, flip: -1, paint: "slate-400" },
    ],
    light: [
      { part: "rim-light", x: 50, y: 56, scale: 1.4, paint: "blue-200" },
      { part: "halo", x: 50, y: 78, scale: 0.8, paint: "blue-200" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "blue-200" },
      { part: "gust", x: 70, y: 40, scale: 1.2 },
    ],
  },
};

/** R3388000: Classic's alt recipes #55 to #90, by card id. */
export const CLASSIC_ALT_RECIPES_3: Readonly<Record<string, SceneRecipe>> = {
  "classic-055": BOOK_OF_WILDFIRE_ALT,
  "classic-058": COMMON_RESOURCES_ALT,
  "classic-060": PILE_ON_ALT,
  "classic-061": PLAGUE_BRINGER_GOLIATH_ALT,
  "classic-062": LIVING_BOMB_ALT,
  "classic-064": MALZAHARS_RECYCLER_ALT,
  "classic-070": BOOK_OF_PLAGUE_ALT,
  "classic-074": CORPSE_PLANTATION_ALT,
  "classic-075": ARGUSLAND_ALT,
  "classic-076": PLAGUE_BRINGER_ALT,
  "classic-080": BOOM_BIG_MAX_ALT,
};
