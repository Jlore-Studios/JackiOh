// R3404000: Meditative's alt scene recipes, part 2 of three (#40 to #71), one per card offered Alt art, each drawn
// from its brief as meditativeAltRecipes1.ts's are: the card's own subject in another place, time or moment, under a
// sky sharing no colour with its normal picture's and a new camera, its figure the normal picture's person, one red
// seal in its near layer and at or above its rarity's floor. Presentation only (CLAUDE.md rule 7).

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import { meditativeExtraLookOf, meditativeLookOf } from "./meditativeAltRecipes1.ts";
import type { SceneRecipe } from "./scene.ts";

/** Feng Shui (Meditative #40, Legendary Field Spell), Alt art: top-down into a small high-rise flat: five tiny elemental spirits nudging a sofa into place, a snowy city below, cool white and blue. */
const FENG_SHUI_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["blue-200", "white"],
  light: { x: 50, y: 60, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 50, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "snow", x: 50, y: 16, scale: 1 }],
    far: [
      { part: "skyline", x: 50, y: 30, scale: 0.5, paint: "slate-400" },
      { part: "tile-floor", x: 50, y: 28, paint: "slate-100" },
    ],
    mid: [
      { part: "bamboo", x: 8, y: 50, scale: 0.6 },
      { part: "paper-screen", x: 92, y: 60, scale: 1.2, flip: -1 },
    ],
    subject: [
      { part: "rune-circle", x: 50, y: 66, scale: 2.2, paint: "jade-400" },
      { part: "sofa", x: 50, y: 66, scale: 1.7 },
      { part: "ink-imp", x: 26, y: 70, scale: 0.9, paint: "green-500" },
      { part: "ink-imp", x: 74, y: 70, scale: 0.9, flip: -1, paint: "red-600" },
      { part: "ink-imp", x: 32, y: 56, scale: 0.8, paint: "wood-700" },
      { part: "ink-imp", x: 68, y: 56, scale: 0.8, flip: -1, paint: "slate-400" },
      { part: "ink-imp", x: 50, y: 82, scale: 0.9, paint: "blue-600" },
    ],
    near: [
      { part: "bamboo", x: 10, y: 99, scale: 1 },
      { part: "ink-seal", x: 84, y: 78 },
    ],
    light: [
      { part: "halo", x: 50, y: 62, scale: 2, paint: "white" },
      { part: "light-shaft", x: 78, y: 14, scale: 1, paint: "white" },
    ],
    particles: [
      { part: "snow", x: 50, y: 24, scale: 1.2 },
      { part: "motes", x: 50, y: 62, scale: 1, paint: "jade-400" },
    ],
  },
};

/** CN Smuggler (Meditative #41, Rare Unit), Alt art: close-up of the smuggler's grin under a straw hat as rain lashes a mountain pass, the tarp blowing off to show grey rocks, deep green storm light. */
const CN_SMUGGLER_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["green-900", "green-700"],
  light: { x: 76, y: 24, r: 34, paint: "jade-400", opacity: 0.35 },
  emblem: { x: 44, y: 66, size: 5, rotate: 0 },
  figure: { x: 42, y: 116, scale: 1.8, pose: "pose-cast", look: { ...meditativeLookOf("meditative-041"), hair: "hat-brim" } },
  layers: {
    sky: [{ part: "lightning", x: 78, y: 18, scale: 0.8 }],
    far: [{ part: "ink-peaks", x: 50, y: 62, scale: 0.9, paint: "green-900" }],
    mid: [
      { part: "cart", x: 80, y: 86, scale: 1.5, paint: "wood-800" },
      { part: "jade-boulder", x: 80, y: 74, scale: 0.5, count: 3, spread: [5, 2], paint: "stone-500" },
    ],
    near: [
      { part: "rain", x: 50, y: 50, scale: 1.4, paint: "slate-100" },
      { part: "ink-seal", x: 16, y: 72 },
    ],
    particles: [
      { part: "rain", x: 50, y: 30, scale: 1.4, paint: "slate-100" },
      { part: "gust", x: 74, y: 58, scale: 0.9 },
    ],
  },
};

/** CN Flea Market (Meditative #42, Legendary Spell), Alt art: the morning after: the riverside alley at sunrise, stalls folded, one vendor counting cards on an upturned crate, pale pink sky over empty cobbles. */
const CN_FLEA_MARKET_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["red-300", "white"],
  light: { x: 80, y: 36, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 48, y: 60, size: 5, rotate: 0 },
  figure: { x: 36, y: 80, scale: 0.55, pose: "pose-kneel", look: { ...meditativeLookOf("meditative-042"), item: "item-card" } },
  layers: {
    sky: [
      { part: "sun", x: 80, y: 36, scale: 0.9 },
      { part: "cloud", x: 26, y: 24, scale: 1, paint: "white" },
    ],
    far: [
      { part: "water", x: 50, y: 50, paint: "blue-200" },
      { part: "tiled-house", x: 80, y: 54, scale: 0.8 },
      { part: "tiled-house", x: 14, y: 52, scale: 0.7 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 60 },
      { part: "awning", x: 76, y: 74, scale: 0.9, paint: "slate-400" },
      { part: "crate", x: 88, y: 80, scale: 0.8, count: 2, spread: [4, 1] },
    ],
    subject: [
      { part: "crate", x: 50, y: 82, scale: 1.1 },
      { part: "playing-card", x: 50, y: 72, scale: 0.5, count: 3, spread: [4, 1] },
    ],
    near: [
      { part: "pebbles", x: 20, y: 94, scale: 1.2 },
      { part: "playing-card", x: 70, y: 94, scale: 0.6 },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    light: [
      { part: "halo", x: 80, y: 36, scale: 1.6, paint: "white" },
      { part: "light-shaft", x: 80, y: 30, scale: 0.8, paint: "red-300" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" },
      { part: "mist", x: 50, y: 54, scale: 0.8 },
    ],
  },
};

/** A buyer at the night market, robed against the cold. */
const BUYER: FigureLook = { ...figureLookFor(["Human"]), hair: "hair-bun", outfit: "outfit-robe", item: null, cloth: "slate-700" };

/** CN Jade Market (Meditative #43, Rare Spell), Alt art: night, close-up: the jade market's covered hall under hanging lamps, a buyer cradling one grey stone like a newborn, deep blue shadows. */
const CN_JADE_MARKET_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["blue-900", "blue-800"],
  light: { x: 50, y: 30, r: 34, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  figure: { x: 50, y: 116, scale: 1.7, pose: "pose-hold", look: BUYER },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 66, paint: "blue-900" },
      { part: "column", x: 10, y: 84, scale: 1.4, paint: "slate-800" },
      { part: "column", x: 90, y: 84, scale: 1.4, paint: "slate-800" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "slate-800" },
      { part: "lantern-string", x: 50, y: 30, scale: 1 },
    ],
    subject: [
      { part: "jade-boulder", x: 16, y: 90, scale: 0.5, count: 2, spread: [3, 1], paint: "slate-400" },
      { part: "awning", x: 24, y: 76, scale: 1, paint: "jade-400" },
      { part: "awning", x: 80, y: 76, scale: 1, flip: -1, paint: "red-600" },
    ],
    near: [
      { part: "jade-boulder", x: 50, y: 76, scale: 0.6, paint: "slate-400" },
      { part: "paper-lantern", x: 18, y: 22, scale: 1.4 },
      { part: "paper-lantern", x: 82, y: 24, scale: 1.3 },
      { part: "ink-seal", x: 84, y: 72 },
    ],
    light: [
      { part: "halo", x: 18, y: 28, scale: 1.2, paint: "orange-400" },
      { part: "halo", x: 82, y: 30, scale: 1.2, paint: "orange-400" },
    ],
  },
};

/** CN Jade Well (Meditative #44, Rare Field Spell), Alt art: winter, wide shot: the well capped in snow on a bare hillside, a child in a red scarf hauling the bucket as a rock rises in it. */
const CN_JADE_WELL_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["white", "blue-200"],
  light: { x: 44, y: 56, r: 30, paint: "jade-400", opacity: 0.35 },
  emblem: { x: 44, y: 66, size: 5, rotate: 0 },
  figure: { x: 60, y: 76, scale: 0.4, pose: "pose-lift", look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-coat", item: null, cloth: "red-600" } },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 22, scale: 1.2, paint: "slate-100" }],
    far: [
      { part: "hills", x: 50, y: 54, paint: "slate-400" },
      { part: "dead-tree", x: 16, y: 58, scale: 0.9 },
      { part: "dead-tree", x: 86, y: 60, scale: 0.7, flip: -1 },
    ],
    mid: [{ part: "snowfield", x: 50, y: 62 }],
    subject: [
      { part: "stone-well", x: 44, y: 78, scale: 1 },
      { part: "jade-boulder", x: 44, y: 62, scale: 0.3, paint: "jade-400" },
      { part: "bucket", x: 52, y: 66, scale: 0.6 },
    ],
    near: [
      { part: "dead-tree", x: 90, y: 99, scale: 1.4, flip: -1 },
      { part: "ink-seal", x: 16, y: 74 },
    ],
    particles: [{ part: "snow", x: 50, y: 40, scale: 1.4 }],
  },
};

/** Knowledge Breaker (Meditative #45, Legendary Unit), Alt art: close-up portrait: the cracked porcelain mask reflected in a rain-streaked tram window, thoughts escaping the passengers as faint wisps, deep teal city behind. */
const KNOWLEDGE_BREAKER_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["green-900", "cyan-400"],
  light: { x: 50, y: 36, r: 36, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  figure: { x: 50, y: 116, scale: 1.8, pose: "pose-spread", look: meditativeLookOf("meditative-045"), fade: 0.75 },
  layers: {
    sky: [{ part: "rain", x: 50, y: 24, scale: 1.4, paint: "slate-100" }],
    far: [
      { part: "skyline", x: 50, y: 66, scale: 1, paint: "green-900" },
      { part: "streetlamp", x: 14, y: 72, scale: 0.8 },
      { part: "streetlamp", x: 86, y: 70, scale: 0.7, flip: -1 },
    ],
    mid: [
      { part: "glass-wall", x: 50, y: 100, scale: 3.4, paint: "cyan-400" },
      { part: "ink-person", x: 16, y: 92, scale: 2, count: 2, spread: [6, 2] },
      { part: "ink-person", x: 84, y: 90, scale: 1.8, flip: -1, count: 2, spread: [6, 2] },
    ],
    near: [
      { part: "wisp", x: 20, y: 60, scale: 1, count: 3, spread: [10, 8] },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    light: [
      { part: "halo", x: 50, y: 30, scale: 1.6, paint: "cyan-400" },
      { part: "rim-light", x: 50, y: 50, scale: 1.4, paint: "slate-100" },
    ],
    particles: [
      { part: "rain", x: 50, y: 62, scale: 1.4, paint: "slate-100" },
      { part: "motes", x: 40, y: 50, scale: 1, paint: "cyan-400" },
    ],
  },
};

/** 饕餮 (Meditative #47, Legendary Unit), Alt art: wide vista: the taotie as a vast storm cloud over a river plain at noon, its jawless maw inhaling a whole enemy column, bruised purple sky. */
const TAOTIE_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "purple-400"],
  light: { x: 50, y: 40, r: 40, paint: "violet-300", opacity: 0.35 },
  emblem: { x: 50, y: 44, size: 6, rotate: 0 },
  layers: {
    sky: [
      { part: "ink-cloud", x: 18, y: 26, scale: 1.4, paint: "violet-500" },
      { part: "ink-cloud", x: 84, y: 22, scale: 1.2, paint: "violet-500" },
    ],
    far: [
      { part: "hills", x: 50, y: 64, paint: "slate-700" },
      { part: "water", x: 50, y: 68, paint: "violet-300" },
    ],
    mid: [
      { part: "field", x: 50, y: 74, paint: "green-900" },
      { part: "soldier", x: 34, y: 82, scale: 0.5, count: 6, spread: [18, 3] },
    ],
    subject: [
      { part: "taotie", x: 50, y: 62, scale: 2.2 },
      { part: "ink-cloud", x: 50, y: 30, scale: 2, paint: "slate-800" },
      { part: "gust", x: 50, y: 70, scale: 1.4 },
    ],
    near: [
      { part: "soldier", x: 76, y: 94, scale: 0.8, flip: -1 },
      { part: "ink-seal", x: 16, y: 76 },
    ],
    light: [
      { part: "lightning", x: 22, y: 30, scale: 0.8 },
      { part: "halo", x: 50, y: 50, scale: 2, paint: "violet-300" },
    ],
    particles: [
      { part: "dust", x: 50, y: 74, scale: 1.2, paint: "slate-400" },
      { part: "motes", x: 50, y: 60, scale: 1.2, paint: "violet-300" },
    ],
  },
};

/** YileGPT (Meditative #49, Mythic Unit), Alt art: close-up: the ink-and-light scholar reading by a snowy hermitage window at dawn, a tiny glitching virus peeking from its sleeve, soft lilac and white. */
const YILE_GPT_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["violet-300", "white"],
  light: { x: 64, y: 34, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 44, y: 66, size: 6, rotate: 0 },
  figure: { x: 44, y: 114, scale: 1.7, pose: "pose-hold", look: { ...meditativeLookOf("meditative-049"), item: "item-book" } },
  layers: {
    sky: [
      { part: "snow", x: 50, y: 24, scale: 1.2 },
      { part: "sun", x: 70, y: 40, scale: 0.7 },
    ],
    far: [
      { part: "ink-peaks", x: 50, y: 58, scale: 0.7, paint: "violet-500" },
      { part: "snow", x: 50, y: 40, scale: 1.2 },
      { part: "ink-round-window", x: 50, y: 44, scale: 3.2 },
    ],
    mid: [
      { part: "closed-tome", x: 18, y: 82, scale: 1.2, count: 2, spread: [3, 2] },
      { part: "incense", x: 84, y: 84, scale: 1 },
    ],
    subject: [{ part: "light-ribbon", x: 40, y: 56, scale: 0.8, paint: "cyan-400" }],
    near: [
      { part: "desk", x: 50, y: 104, scale: 3.6, paint: "wood-800" },
      { part: "glitch-critter", x: 66, y: 80, scale: 1 },
      { part: "brush", x: 26, y: 88, scale: 0.6 },
      { part: "teapot", x: 84, y: 94, scale: 0.8 },
      { part: "ink-seal", x: 16, y: 76 },
    ],
    light: [
      { part: "halo", x: 50, y: 40, scale: 2.4, paint: "white" },
      { part: "light-shaft", x: 70, y: 16, scale: 1, paint: "violet-300" },
      { part: "rim-light", x: 44, y: 60, scale: 1.4, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" },
      { part: "squares", x: 66, y: 72, scale: 0.4, paint: "violet-500" },
      { part: "sparks", x: 66, y: 76, scale: 0.8, paint: "cyan-400" },
    ],
  },
};

/** CN Tech (Meditative #50, Rare Spell), Alt art: low angle in a bustling noon street market: a pocket gadget beams one card out of a whirling cloud above the crowd, warm red awnings. */
const CN_TECH_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-400", "red-400"],
  light: { x: 50, y: 34, r: 36, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 50, y: 46, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 84, y: 18, scale: 0.7 }],
    far: [
      { part: "tiled-house", x: 14, y: 64, scale: 1.4 },
      { part: "tiled-house", x: 86, y: 62, scale: 1.3, flip: -1 },
    ],
    mid: [
      { part: "awning", x: 20, y: 84, scale: 1.4, paint: "red-600" },
      { part: "awning", x: 80, y: 84, scale: 1.4, flip: -1, paint: "red-500" },
      { part: "crowd", x: 50, y: 99, scale: 0.9 },
    ],
    subject: [
      { part: "card-ring", x: 50, y: 32, scale: 1.6 },
      { part: "ink-cloud", x: 50, y: 30, scale: 0.9, paint: "slate-400" },
      { part: "playing-card", x: 50, y: 44, scale: 1 },
      { part: "hologram", x: 50, y: 66, scale: 0.6 },
    ],
    near: [
      { part: "gust", x: 50, y: 28, scale: 1.2 },
      { part: "ink-seal", x: 16, y: 74 },
    ],
    light: [{ part: "halo", x: 50, y: 44, scale: 1.4, paint: "cyan-400" }],
  },
};

/** Devin Bot (Meditative #51, Legendary Unit), Alt art: wide shot: the robot meditating on a beach at sunset, tiny copies of itself streaming out to fill the sand in rows, orange and violet sky. */
const DEVIN_BOT_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["orange-400", "violet-500"],
  light: { x: 50, y: 50, r: 40, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 50, y: 64, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 50, y: 50, scale: 1.4 },
      { part: "cloud", x: 22, y: 24, scale: 1.2, paint: "violet-300" },
    ],
    far: [{ part: "sea", x: 50, y: 54 }],
    mid: [
      { part: "sand", x: 50, y: 66 },
      { part: "round-bot", x: 28, y: 76, scale: 0.3, count: 4, spread: [16, 2] },
      { part: "round-bot", x: 72, y: 80, scale: 0.35, count: 3, spread: [16, 2] },
    ],
    subject: [{ part: "round-bot", x: 50, y: 72, scale: 0.8 }],
    near: [
      { part: "round-bot", x: 20, y: 94, scale: 0.45 },
      { part: "pebbles", x: 76, y: 96, scale: 1.2 },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    light: [
      { part: "halo", x: 50, y: 50, scale: 2, paint: "orange-400" },
      { part: "rim-light", x: 50, y: 64, scale: 1, paint: "orange-400" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 64, scale: 1, count: 2, spread: [16, 6] },
      { part: "motes", x: 50, y: 56, scale: 1.2, paint: "white" },
    ],
  },
};

/** Dragon Fruit (Meditative #55, Rare Spell), Alt art: wide angle: the dragon fruit still on its cactus vine on a sun-blasted cliff, a little dragon curled smugly around it, deep blue sky. */
const DRAGON_FRUIT_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["blue-700", "sky-400"],
  light: { x: 78, y: 22, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 46, y: 70, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 78, y: 22, scale: 1.1 }],
    far: [
      { part: "mesa", x: 22, y: 68, scale: 1, paint: "red-900" },
      { part: "mesa", x: 82, y: 70, scale: 0.8, paint: "wood-700" },
    ],
    mid: [
      { part: "cliff", x: 66, y: 62, scale: 1.4, paint: "wood-600" },
      { part: "cactus", x: 44, y: 62, scale: 1.2 },
    ],
    subject: [
      { part: "dragon-fruit", x: 50, y: 46, scale: 0.9 },
      { part: "hatchling", x: 56, y: 62, scale: 1.2, flip: -1 },
    ],
    near: [
      { part: "pebbles", x: 30, y: 92, scale: 1.2 },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    light: [{ part: "halo", x: 50, y: 42, scale: 0.8, paint: "red-300" }],
  },
};

/** House Party (Meditative #56, Common Spell), Alt art: the morning after: the defenders asleep in heaps on the timber house's lawn as sunrise breaks over misty fields, soft green and pink. */
const HOUSE_PARTY_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "green-200"],
  light: { x: 22, y: 42, r: 40, paint: "red-300", opacity: 0.4 },
  emblem: { x: 58, y: 54, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 22, y: 42, scale: 1 }],
    far: [
      { part: "hills", x: 50, y: 50, paint: "green-700" },
      { part: "mist", x: 50, y: 52, scale: 1.2 },
    ],
    mid: [
      { part: "field", x: 50, y: 58, paint: "green-400" },
      { part: "fence", x: 50, y: 66, scale: 0.8, paint: "wood-700" },
    ],
    subject: [
      { part: "tiled-house", x: 60, y: 62, scale: 1 },
      { part: "soldier-fallen", x: 30, y: 74, scale: 0.9, count: 4, spread: [14, 3] },
    ],
    near: [
      { part: "soldier-fallen", x: 72, y: 80, scale: 1.1, flip: -1 },
      { part: "grass-tuft", x: 14, y: 98, count: 2, spread: [5, 1], paint: "green-700" },
      { part: "ink-seal", x: 84, y: 74 },
    ],
    particles: [{ part: "mist", x: 50, y: 72, scale: 1 }],
  },
};

/** Eschews (Meditative #60, Epic Field Spell), Alt art: low angle: a hammock slung between two cliff pines, a sleeper ignoring the random spirits juggling chores around him, bright blue sky. */
const ESCHEWS_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-400", "blue-200"],
  light: { x: 70, y: 28, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 64, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 24, y: 24, scale: 1.2, paint: "white" }],
    far: [
      { part: "cliff", x: 22, y: 76, scale: 1.4, paint: "slate-700" },
      { part: "cliff", x: 80, y: 78, scale: 1.3, flip: -1, paint: "slate-700" },
    ],
    mid: [
      { part: "pine", x: 18, y: 80, scale: 2 },
      { part: "pine", x: 82, y: 82, scale: 2, flip: -1 },
      { part: "hammock", x: 50, y: 58, scale: 1.6 },
      { part: "ink-person", x: 50, y: 62, scale: 1.2 },
    ],
    subject: [
      { part: "parcel", x: 30, y: 36, scale: 0.7 },
      { part: "envelope-drift", x: 68, y: 34, scale: 0.7 },
      { part: "ink-imp", x: 36, y: 46, scale: 1, count: 3, spread: [18, 6] },
    ],
    near: [
      { part: "parcel", x: 86, y: 98, scale: 1 },
      { part: "ink-seal", x: 16, y: 74 },
    ],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1, paint: "white" }],
  },
};

/** Skull of J'Nari (Meditative #63, Legendary Field Spell), Alt art: the skull half-buried on a sunny beach at low tide, jaw chattering at gulls as a warrior claws up out of the wet sand, sea-green and white. */
const SKULL_OF_JNARI_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["jade-400", "white"],
  light: { x: 80, y: 20, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 42, y: 76, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 74, y: 98, scale: 0.6, pose: "pose-raise", look: meditativeExtraLookOf("meditative-063", 0) }],
  layers: {
    sky: [
      { part: "sun", x: 80, y: 20, scale: 0.9 },
      { part: "gull", x: 30, y: 30, scale: 1.2, count: 3, spread: [14, 6] },
    ],
    far: [
      { part: "sea", x: 50, y: 52 },
      { part: "wave", x: 82, y: 58, scale: 0.4, flip: -1 },
    ],
    mid: [
      { part: "sand", x: 50, y: 62 },
      { part: "pebbles", x: 22, y: 76, scale: 1 },
    ],
    subject: [{ part: "horned-skull", x: 42, y: 86, scale: 1.2, paint: "slate-100" }],
    near: [
      { part: "drops", x: 74, y: 80, scale: 0.8, paint: "blue-200" },
      { part: "gull", x: 62, y: 66, scale: 1 },
      { part: "ink-seal", x: 16, y: 72 },
    ],
    light: [
      { part: "halo", x: 80, y: 20, scale: 1.6, paint: "white" },
      { part: "god-rays", x: 80, y: 18, scale: 0.9, paint: "white" },
    ],
    particles: [
      { part: "spray", x: 80, y: 60, scale: 1 },
      { part: "motes", x: 50, y: 50, scale: 1, paint: "white" },
    ],
  },
};

/** Traitorous Blood (Meditative #64, Common Trap), Alt art: night ambush on a rope bridge: lantern light catches one soldier's sword turning into his neighbour's shield, the red thread taut, deep indigo gorge below. */
const TRAITOROUS_BLOOD_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["blue-900", "violet-700"],
  light: { x: 22, y: 36, r: 30, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  figure: { x: 58, y: 60, scale: 0.5, flip: -1, pose: "pose-lunge", look: meditativeLookOf("meditative-064") },
  extras: [{ layer: "subject", x: 40, y: 61, scale: 0.48, pose: "pose-block", look: meditativeExtraLookOf("meditative-064", 0) }],
  layers: {
    sky: [
      { part: "moon", x: 80, y: 22, scale: 0.6 },
      { part: "stars", x: 34, y: 22, scale: 1 },
    ],
    far: [
      { part: "cliff", x: 14, y: 58, scale: 1.4, paint: "slate-800" },
      { part: "cliff", x: 86, y: 58, scale: 1.4, flip: -1, paint: "slate-800" },
    ],
    mid: [
      { part: "mist", x: 50, y: 82, scale: 1.2, paint: "violet-500" },
      { part: "rope-bridge", x: 50, y: 60, scale: 1.4 },
    ],
    subject: [
      { part: "red-thread", x: 40, y: 48, scale: 0.6 },
      { part: "shield", x: 46, y: 50, scale: 0.6 },
    ],
    near: [
      { part: "lantern", x: 22, y: 30, scale: 1 },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    light: [{ part: "halo", x: 22, y: 36, scale: 1.2, paint: "orange-400" }],
  },
};

/** Keymaster Keenus (Meditative #65, Legendary Unit), Alt art: close-up portrait: the old gatekeeper on a sunny harbour quay holding one key up to his eye like a lens, every key on his robe glinting, sea blue. */
const KEYMASTER_KEENUS_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-400", "blue-600"],
  light: { x: 80, y: 20, r: 36, paint: "white", opacity: 0.45 },
  emblem: { x: 46, y: 66, size: 5, rotate: 0 },
  figure: { x: 46, y: 116, scale: 1.8, pose: "pose-raise", look: meditativeLookOf("meditative-065") },
  layers: {
    sky: [
      { part: "sun", x: 82, y: 20, scale: 0.8 },
      { part: "gull", x: 24, y: 26, scale: 1, count: 2, spread: [8, 4] },
    ],
    far: [
      { part: "sea", x: 50, y: 58 },
      { part: "boat", x: 82, y: 62, scale: 0.6 },
      { part: "boat", x: 14, y: 60, scale: 0.5, flip: -1 },
    ],
    mid: [{ part: "boards", x: 50, y: 72 }],
    near: [
      { part: "key", x: 22, y: 84, scale: 0.8 },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    light: [
      { part: "halo", x: 62, y: 30, scale: 1, paint: "white" },
      { part: "god-rays", x: 82, y: 18, scale: 0.8, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 60, scale: 1.2, count: 3, spread: [12, 10] },
      { part: "motes", x: 50, y: 40, scale: 1, paint: "white" },
    ],
  },
};

/** The Maestro (Meditative #69, Rare Unit), Alt art: open-air concert on a mountain meadow at noon: the baton raised before the coughing choir, green plague clouds drifting across a bright blue sky, wide shot. */
const THE_MAESTRO_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-400", "blue-200"],
  light: { x: 50, y: 20, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 62, y: 56, size: 5, rotate: 0 },
  figure: { x: 64, y: 72, scale: 0.5, flip: -1, pose: "pose-raise", look: meditativeLookOf("meditative-069") },
  layers: {
    sky: [
      { part: "ink-cloud", x: 26, y: 24, scale: 0.9, paint: "green-400" },
      { part: "ink-cloud", x: 76, y: 20, scale: 0.7, paint: "green-400" },
    ],
    far: [{ part: "mountains", x: 50, y: 50, paint: "slate-700" }],
    mid: [
      { part: "field", x: 50, y: 60, paint: "green-500" },
      { part: "flowers", x: 14, y: 80, count: 3, spread: [8, 3] },
    ],
    subject: [{ part: "ink-person", x: 34, y: 72, scale: 1, count: 4, spread: [10, 3] }],
    near: [
      { part: "grass-tuft", x: 50, y: 98, count: 3, spread: [40, 1], paint: "green-700" },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    particles: [{ part: "spore-motes", x: 34, y: 60, scale: 1, paint: "green-400" }],
  },
};

/** Pareto Optimality (Meditative #71, Mythic Field Spell), Alt art: before class, from the back row: the empty study hall at copper-red sunset, the cane at rest on the lectern, the curve scroll glowing across the desks. */
const PARETO_OPTIMALITY_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["red-500", "orange-400"],
  light: { x: 76, y: 36, r: 40, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 50, y: 40, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 76, y: 40, scale: 1.2 }],
    far: [
      { part: "room-wall", x: 50, y: 58, paint: "wood-700" },
      { part: "window", x: 24, y: 46, scale: 1.4 },
      { part: "window", x: 76, y: 46, scale: 1.4 },
      { part: "bell-curve", x: 50, y: 44, scale: 1.2, paint: "orange-400" },
    ],
    mid: [
      { part: "boards", x: 50, y: 58 },
      { part: "paper-screen", x: 8, y: 68, scale: 1.4 },
      { part: "paper-screen", x: 92, y: 68, scale: 1.4, flip: -1 },
      { part: "desk", x: 30, y: 68, scale: 1, count: 3, spread: [6, 1] },
      { part: "desk", x: 70, y: 68, scale: 1, count: 3, spread: [6, 1] },
    ],
    subject: [
      { part: "cane", x: 54, y: 50, scale: 0.5 },
      { part: "lectern", x: 50, y: 62, scale: 1.4 },
      { part: "desk", x: 24, y: 80, scale: 1.8 },
      { part: "desk", x: 76, y: 80, scale: 1.8, flip: -1 },
    ],
    near: [
      { part: "desk", x: 14, y: 99, scale: 2.6 },
      { part: "desk", x: 86, y: 99, scale: 2.6, flip: -1 },
      { part: "closed-tome", x: 14, y: 78, scale: 1 },
      { part: "ink-seal", x: 50, y: 82 },
    ],
    light: [
      { part: "god-rays", x: 76, y: 30, scale: 1, paint: "orange-400" },
      { part: "halo", x: 50, y: 46, scale: 1.4, paint: "orange-400" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "orange-400" },
      { part: "sparks", x: 50, y: 40, scale: 1, count: 2, spread: [16, 4] },
    ],
  },
};

/** R3404000: the alt recipes of Meditative #40 to #71, by card id. */
export const MEDITATIVE_ALT_RECIPES_2: Readonly<Record<string, SceneRecipe>> = {
  "meditative-040": FENG_SHUI_ALT,
  "meditative-041": CN_SMUGGLER_ALT,
  "meditative-042": CN_FLEA_MARKET_ALT,
  "meditative-043": CN_JADE_MARKET_ALT,
  "meditative-044": CN_JADE_WELL_ALT,
  "meditative-045": KNOWLEDGE_BREAKER_ALT,
  "meditative-047": TAOTIE_ALT,
  "meditative-049": YILE_GPT_ALT,
  "meditative-050": CN_TECH_ALT,
  "meditative-051": DEVIN_BOT_ALT,
  "meditative-055": DRAGON_FRUIT_ALT,
  "meditative-056": HOUSE_PARTY_ALT,
  "meditative-060": ESCHEWS_ALT,
  "meditative-063": SKULL_OF_JNARI_ALT,
  "meditative-064": TRAITOROUS_BLOOD_ALT,
  "meditative-065": KEYMASTER_KEENUS_ALT,
  "meditative-069": THE_MAESTRO_ALT,
  "meditative-071": PARETO_OPTIMALITY_ALT,
};
