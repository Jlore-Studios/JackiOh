// R3390000: Classic+'s scene recipes, part 3 of three (#53 to #78, #65's, #73's, #75's and #76's tokens and
// T-AI-1 to 10), one per entry, each drawn from its brief in Classic's temple-stone look (ART.md, "Classic+ drawn
// as scenes"): its subject within x 30 to 70 and y 25 to 85, its foreground near, its middle in the mid and subject
// layers and its background far, richer the rarer. Presentation only (CLAUDE.md rule 7): a recipe is read by the
// card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** The ink-drawn sprinters of Book of Tokens, glowing indigo off the page. */
const INK_SPRINTER: FigureLook = { ...figureLookFor([]), hair: null, item: null, skin: "violet-300", cloth: "violet-500" };

/** Book of Tokens (Classic+ #53, Epic): two tiny ink sprinters leap off an open tome on a scriptorium lectern and dash across its desk. */
const BOOK_OF_TOKENS: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-900"],
  light: { x: 50, y: 48, r: 34, paint: "violet-500", opacity: 0.45 },
  emblem: { x: 50, y: 27, size: 6, rotate: 0 },
  extras: [
    { layer: "subject", x: 60, y: 45, scale: 0.34, pose: "pose-run", look: INK_SPRINTER },
    { layer: "subject", x: 70, y: 69, scale: 0.3, pose: "pose-run", look: { ...INK_SPRINTER, cloth: "blue-600" } },
  ],
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 64, paint: "slate-800" },
      { part: "bookshelf", x: 14, y: 64, scale: 1.6 },
      { part: "bookshelf", x: 86, y: 64, scale: 1.6 },
      { part: "candle", x: 14, y: 30, count: 2, spread: [6, 0] },
      { part: "candle", x: 86, y: 30, count: 2, spread: [6, 0] },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 64, paint: "wood-800" },
      { part: "table", x: 76, y: 80, scale: 1.4 },
      { part: "scroll", x: 83, y: 70, scale: 0.7 },
    ],
    subject: [
      { part: "lectern", x: 48, y: 88, scale: 3.6 },
      { part: "tome", x: 49, y: 52, scale: 1.5, paint: "blue-900" },
    ],
    near: [
      { part: "quill", x: 18, y: 92, scale: 1.4 },
      { part: "mist", x: 24, y: 94, scale: 0.5, paint: "blue-900" },
      { part: "tome", x: 84, y: 96, scale: 1.1, paint: "red-900" },
    ],
    light: [
      { part: "halo", x: 49, y: 46, scale: 2.6, paint: "violet-500" },
      { part: "halo", x: 14, y: 27, scale: 1, paint: "orange-400", count: 2, spread: [6, 0] },
      { part: "halo", x: 86, y: 27, scale: 1, paint: "orange-400" },
      { part: "rim-light", x: 50, y: 54, scale: 1.2, paint: "violet-300" },
    ],
    particles: [
      { part: "sparks", x: 56, y: 40, scale: 1, count: 3, spread: [12, 6], paint: "violet-300" },
      { part: "motes", x: 50, y: 50, scale: 1.3, paint: "blue-200" },
    ],
  },
};

/** Book of Books (Classic+ #54, Epic): a grand old book on a reading throne signs a smaller book with its bookmark's quill, books fluttering round. */
const BOOK_OF_BOOKS: SceneRecipe = {
  family: "classic",
  sky: ["wood-900", "brass-800"],
  light: { x: 50, y: 46, r: 38, paint: "brass-500", opacity: 0.45 },
  emblem: { x: 50, y: 41, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 66, paint: "wood-700" },
      { part: "bookshelf", x: 10, y: 66, scale: 1.7 },
      { part: "bookshelf", x: 28, y: 58, scale: 1.2 },
      { part: "bookshelf", x: 72, y: 58, scale: 1.2 },
      { part: "bookshelf", x: 90, y: 66, scale: 1.7 },
      { part: "bookshelf", x: 20, y: 34, scale: 0.8 },
      { part: "bookshelf", x: 80, y: 34, scale: 0.8 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "wood-800" },
      { part: "tome", x: 30, y: 32, scale: 0.6, count: 2, spread: [8, 6], paint: "red-900" },
      { part: "tome", x: 70, y: 30, scale: 0.6, count: 2, spread: [8, 6], paint: "blue-800" },
      { part: "throne", x: 50, y: 84, scale: 2.4 },
    ],
    subject: [
      { part: "tome", x: 50, y: 63, scale: 1.5, paint: "red-900" },
      { part: "tome", x: 64, y: 82, scale: 0.8, paint: "green-900" },
      { part: "quill", x: 60, y: 79, scale: 0.9 },
    ],
    near: [
      { part: "tome", x: 14, y: 96, scale: 1.2, paint: "wood-700" },
      { part: "scroll", x: 84, y: 95, scale: 1.1 },
      { part: "candle", x: 90, y: 89, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 50, y: 58, scale: 2.6, paint: "orange-400" },
      { part: "halo", x: 90, y: 85, scale: 0.9, paint: "orange-400" },
      { part: "rim-light", x: 50, y: 60, scale: 1.3, paint: "brass-500" },
    ],
    particles: [
      { part: "motes", x: 50, y: 46, scale: 1.5, paint: "brass-500" },
      { part: "sparks", x: 58, y: 74, scale: 0.8, paint: "white" },
    ],
  },
};

/** Book of Greed (Classic+ #55, Epic): a miser prises open a jewelled crimson tome chained to a vault plinth, gems spilling before it. */
const BOOK_OF_GREED: SceneRecipe = {
  family: "classic",
  sky: ["black", "red-900"],
  light: { x: 44, y: 58, r: 32, paint: "red-600", opacity: 0.5 },
  emblem: { x: 42, y: 75, size: 5, rotate: 0 },
  figure: {
    x: 64,
    y: 87,
    scale: 0.92,
    flip: -1,
    pose: "pose-guard",
    look: { ...figureLookFor(["Human"]), hair: "tonsure", outfit: "outfit-coat", item: null, skin: "red-300", cloth: "violet-700" },
  },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 64, paint: "slate-900" },
      { part: "arch", x: 44, y: 64, scale: 1.7, paint: "slate-800" },
      { part: "chest", x: 14, y: 66, scale: 0.9 },
      { part: "chest", x: 86, y: 64, scale: 0.8, flip: -1 },
      { part: "coins", x: 22, y: 66, scale: 0.8 },
    ],
    mid: [
      { part: "tile-floor", x: 44, y: 64, paint: "slate-800" },
      { part: "altar", x: 42, y: 84, scale: 1.8 },
      { part: "chain", x: 22, y: 76, scale: 0.9 },
    ],
    subject: [
      { part: "tome", x: 42, y: 67, scale: 1.4, paint: "red-600" },
      { part: "gems", x: 42, y: 66, scale: 0.6 },
    ],
    near: [
      { part: "gems", x: 30, y: 93, scale: 1.3, count: 3, spread: [20, 3] },
      { part: "coins", x: 82, y: 95, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 42, y: 60, scale: 2.6, paint: "red-600" },
      { part: "halo", x: 46, y: 56, scale: 1.4, paint: "violet-500" },
      { part: "rim-light", x: 62, y: 62, scale: 1.2, paint: "violet-300", flip: -1 },
    ],
    particles: [
      { part: "sparks", x: 42, y: 56, scale: 1, count: 3, spread: [12, 6], paint: "violet-300" },
      { part: "motes", x: 50, y: 60, scale: 1.3, paint: "red-300" },
    ],
  },
};

/** Book of Pain (Classic+ #56, Epic): a snarling black book bursts open in a torchlit cell, its razor pages slicing a page from a fleeing rival. */
const BOOK_OF_PAIN: SceneRecipe = {
  family: "classic",
  sky: ["black", "red-900"],
  light: { x: 38, y: 54, r: 34, paint: "red-600", opacity: 0.5 },
  emblem: { x: 50, y: 28, size: 5, rotate: 0 },
  extras: [
    {
      layer: "subject",
      x: 72,
      y: 86,
      scale: 0.82,
      pose: "pose-run",
      look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-tabard", item: "item-scroll", cloth: "slate-700" },
    },
  ],
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 64, paint: "slate-800" },
      { part: "bookshelf", x: 14, y: 64, scale: 1.4 },
      { part: "bookshelf", x: 86, y: 64, scale: 1.4 },
      { part: "chain", x: 4, y: 44, scale: 0.9 },
      { part: "chain", x: 74, y: 46, scale: 0.9 },
      { part: "portcullis", x: 50, y: 64, scale: 1.3 },
      { part: "flames", x: 30, y: 38, scale: 0.6 },
      { part: "flames", x: 70, y: 38, scale: 0.6 },
    ],
    mid: [{ part: "rock-floor", x: 50, y: 64, paint: "slate-800" }],
    subject: [
      { part: "tome", x: 38, y: 70, scale: 1.6, paint: "slate-900" },
      { part: "card", x: 48, y: 58, scale: 0.9, paint: "slate-100" },
      { part: "card", x: 56, y: 52, scale: 0.8, paint: "slate-100" },
      { part: "card", x: 30, y: 54, scale: 0.8, paint: "slate-100" },
      { part: "card", x: 62, y: 62, scale: 0.7, paint: "slate-100" },
    ],
    near: [
      { part: "spores", x: 22, y: 92, scale: 1.2, paint: "red-600" },
      { part: "dust", x: 70, y: 96, scale: 1.2, paint: "red-900" },
      { part: "scroll", x: 84, y: 94, scale: 0.9 },
    ],
    light: [
      { part: "halo", x: 38, y: 62, scale: 2.4, paint: "red-600" },
      { part: "halo", x: 30, y: 34, scale: 1, paint: "orange-400" },
      { part: "halo", x: 70, y: 34, scale: 1, paint: "orange-400" },
      { part: "rim-light", x: 70, y: 62, scale: 1.2, paint: "red-300", flip: -1 },
    ],
    particles: [
      { part: "sparks", x: 64, y: 58, scale: 1, count: 3, spread: [10, 6], paint: "red-300" },
      { part: "embers", x: 40, y: 56, scale: 1.4, count: 3, spread: [16, 8] },
    ],
  },
};

/** Book of Stats (Classic+ #57, Epic): in a stone gymnasium a squire reading a training manual by the weight rack swells into a muscled warrior. */
const BOOK_OF_STATS: SceneRecipe = {
  family: "classic",
  sky: ["blue-600", "sky-300"],
  light: { x: 50, y: 50, r: 30, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 26, y: 66, size: 5, rotate: 0 },
  figure: {
    x: 52,
    y: 86,
    scale: 1.02,
    pose: "pose-stand",
    look: { ...figureLookFor(["Human"]), outfit: "outfit-muscle", item: "item-book", cloth: "red-600" },
  },
  layers: {
    sky: [
      { part: "sun", x: 20, y: 22, scale: 0.9 },
      { part: "cloud", x: 70, y: 22, scale: 0.7 },
    ],
    far: [
      { part: "column", x: 50, y: 58, scale: 0.8, count: 5, spread: [44, 0] },
      { part: "statue", x: 12, y: 63, scale: 1.4 },
      { part: "statue", x: 88, y: 63, scale: 1.4, flip: -1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 62, paint: "stone-500" },
      { part: "weapon-rack", x: 26, y: 82, scale: 1.2 },
      { part: "shockwave", x: 52, y: 62, scale: 0.6, paint: "orange-400" },
    ],
    subject: [{ part: "halo", x: 52, y: 58, scale: 3, paint: "orange-400" }],
    near: [
      { part: "weights", x: 18, y: 94, scale: 1.3 },
      { part: "weights", x: 82, y: 95, scale: 1.2 },
      { part: "grass-tuft", x: 50, y: 97, count: 3, spread: [40, 2] },
    ],
    light: [
      { part: "light-shaft", x: 26, y: 0, scale: 1.1, paint: "white" },
      { part: "rim-light", x: 52, y: 56, scale: 1.3, paint: "orange-400" },
    ],
    particles: [
      { part: "sparks", x: 52, y: 40, scale: 1.2, count: 3, spread: [14, 8], paint: "sky-300" },
      { part: "motes", x: 50, y: 74, scale: 1.3, paint: "white" },
    ],
  },
};

/** Fruit Basket (Classic+ #58, Rare): a wicker basket heaped with apples, grapes and pears on an orchard temple's stone wall at harvest. */
const FRUIT_BASKET: SceneRecipe = {
  family: "classic",
  sky: ["blue-800", "orange-400"],
  light: { x: 22, y: 42, r: 32, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 50, y: 78, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 72, y: 24, scale: 0.7, count: 2, spread: [14, 4] }],
    far: [
      { part: "hills", x: 50, y: 62, paint: "green-900" },
      { part: "temple", x: 50, y: 58, scale: 0.7 },
      { part: "fruit-tree", x: 14, y: 64, scale: 0.9 },
      { part: "fruit-tree", x: 86, y: 62, scale: 0.8, flip: -1 },
    ],
    mid: [{ part: "ruin-wall", x: 54, y: 92, scale: 2.6 }],
    subject: [
      { part: "basket", x: 51, y: 68, scale: 2.3 },
      { part: "apple", x: 29, y: 67, scale: 1.4 },
      { part: "pear", x: 70, y: 73, scale: 1.4 },
      { part: "grapes", x: 63, y: 58, scale: 1 },
    ],
    near: [
      { part: "grass-tuft", x: 50, y: 96, count: 4, spread: [44, 2] },
      { part: "flowers", x: 14, y: 94, scale: 1.2 },
      { part: "apple", x: 84, y: 96, scale: 1.1 },
    ],
    light: [
      { part: "halo", x: 50, y: 58, scale: 2.4, paint: "orange-400" },
      { part: "rim-light", x: 50, y: 58, scale: 1.1, paint: "orange-400", flip: -1 },
    ],
    particles: [
      { part: "sparks", x: 50, y: 56, scale: 0.9, count: 2, spread: [14, 4], paint: "white" },
      { part: "motes", x: 50, y: 40, scale: 1.3, paint: "sky-300" },
    ],
  },
};

/** All Purpose Apple (Classic+ #59, Rare): a shiny red apple flies across a village square, healing a knight with green sparkles and bonking a goblin. */
const ALL_PURPOSE_APPLE: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 50, y: 36, r: 30, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 63, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 28, y: 86, scale: 0.78, pose: "pose-raise", look: { ...figureLookFor(["Human"]), outfit: "outfit-armor", item: "item-sword", cloth: "slate-400" } },
    { layer: "subject", x: 74, y: 86, scale: 0.66, flip: -1, pose: "pose-spread", look: { ...figureLookFor([]), head: "head-goblin", hair: null, item: "item-dagger", skin: "green-500", cloth: "wood-700" } },
  ],
  layers: {
    far: [
      { part: "house", x: 12, y: 64, scale: 1.1 },
      { part: "house", x: 88, y: 62, scale: 1, flip: -1 },
      { part: "tower", x: 70, y: 58, scale: 0.6 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 62, paint: "stone-500" },
      { part: "fountain", x: 50, y: 70, scale: 0.9 },
    ],
    subject: [{ part: "apple", x: 52, y: 44, scale: 3.2 }],
    near: [
      { part: "crate", x: 10, y: 96, scale: 1.4 },
      { part: "grass-tuft", x: 50, y: 97, count: 3, spread: [40, 2] },
    ],
    light: [
      { part: "halo", x: 28, y: 60, scale: 2, paint: "green-400" },
      { part: "halo", x: 52, y: 38, scale: 1.6, paint: "red-300" },
    ],
    particles: [
      { part: "sparks", x: 30, y: 56, scale: 1, count: 3, spread: [10, 10], paint: "green-400" },
      { part: "motes", x: 40, y: 44, scale: 1.2, paint: "green-200" },
      { part: "sparks", x: 70, y: 54, scale: 0.8, paint: "white" },
    ],
  },
};

/** Doctors Orders (Classic+ #60, Rare): a whitewashed temple infirmary, stone cots, an apple bowl on the physician's desk and herbs drying from the rafters. */
const DOCTORS_ORDERS: SceneRecipe = {
  family: "classic",
  sky: ["blue-200", "slate-100"],
  light: { x: 30, y: 32, r: 34, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 42, size: 6, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 64, paint: "slate-100" },
      { part: "fence", x: 50, y: 21, scale: 2, paint: "wood-700" },
      { part: "grapes", x: 22, y: 20, scale: 1, count: 2, spread: [10, 1], paint: "green-700" },
      { part: "grapes", x: 76, y: 20, scale: 1, paint: "green-700" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 64, paint: "slate-400" },
      { part: "cot", x: 18, y: 76, scale: 1.6 },
      { part: "cot", x: 84, y: 74, scale: 1.4, flip: -1 },
      { part: "cabinet", x: 90, y: 64, scale: 1 },
    ],
    subject: [
      { part: "table", x: 50, y: 84, scale: 2 },
      { part: "basket", x: 50, y: 69, scale: 1.4 },
      { part: "flask", x: 36, y: 69, scale: 1 },
      { part: "scroll", x: 64, y: 69, scale: 0.8 },
    ],
    near: [
      { part: "flask", x: 12, y: 94, scale: 1.2, paint: "jade-400" },
      { part: "tome", x: 86, y: 96, scale: 1, paint: "blue-800" },
    ],
    light: [
      { part: "light-shaft", x: 32, y: 10, scale: 1.2, paint: "white" },
      { part: "halo", x: 66, y: 30, scale: 1.6, paint: "white" },
    ],
    particles: [{ part: "motes", x: 36, y: 50, scale: 1.4, paint: "white" }],
  },
};

/** Bauble Bubble (Classic+ #61, Rare): a giant soap-bubble dome resting in a fountain plaza, trinkets, baubles and tiny crates floating inside. */
const BAUBLE_BUBBLE: SceneRecipe = {
  family: "classic",
  sky: ["blue-800", "violet-300"],
  light: { x: 50, y: 56, r: 36, paint: "violet-300", opacity: 0.45 },
  emblem: { x: 50, y: 80, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "house", x: 12, y: 62, scale: 1 },
      { part: "temple", x: 50, y: 56, scale: 0.6 },
      { part: "house", x: 88, y: 62, scale: 1, flip: -1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 62, paint: "stone-500" },
      { part: "fountain", x: 14, y: 80, scale: 0.9 },
      { part: "fountain", x: 86, y: 80, scale: 0.9 },
    ],
    subject: [
      { part: "crate", x: 44, y: 78, scale: 1.2, count: 3, spread: [6, 1] },
      { part: "crate", x: 45, y: 70, scale: 1 },
      { part: "gift-box", x: 60, y: 68, scale: 1.1 },
      { part: "orb", x: 36, y: 62, scale: 0.5, paint: "red-600" },
      { part: "orb", x: 62, y: 56, scale: 0.45, paint: "violet-500" },
      { part: "gems", x: 50, y: 58, scale: 0.9 },
      { part: "dome", x: 50, y: 84, scale: 1.55 },
    ],
    near: [
      { part: "bubbles", x: 16, y: 90, scale: 1.2 },
      { part: "bubbles", x: 84, y: 88, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 50, y: 66, scale: 2.6, paint: "violet-300" },
      { part: "rim-light", x: 50, y: 70, scale: 1.4, paint: "white", flip: -1 },
    ],
    particles: [
      { part: "sparks", x: 50, y: 56, scale: 1, count: 3, spread: [18, 8], paint: "white" },
      { part: "bubbles", x: 50, y: 40, scale: 1, count: 2, spread: [24, 6] },
    ],
  },
};

/** KY's Papaya (Classic+ #62, Epic, full art): a giant papaya slice traces a glowing S across a chalked flagstone grid as each statue along it topples. */
const KYS_PAPAYA: SceneRecipe = {
  family: "classic",
  sky: ["slate-800", "green-900"],
  light: { x: 50, y: 50, r: 36, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 50, y: 76, size: 6, rotate: 0 },
  layers: {
    far: [
      { part: "arch", x: 18, y: 60, scale: 1.1 },
      { part: "arch", x: 50, y: 58, scale: 1 },
      { part: "arch", x: 82, y: 60, scale: 1.1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 60, paint: "stone-500" },
      { part: "statue", x: 26, y: 70, scale: 0.85, props: true },
      { part: "fallen-column", x: 28, y: 84, scale: 0.6 },
      { part: "statue", x: 76, y: 66, scale: 0.75, flip: -1, props: true },
      { part: "fallen-column", x: 76, y: 74, scale: 0.55, flip: -1 },
    ],
    subject: [
      { part: "light-ribbon", x: 46, y: 66, scale: 1.15, paint: "orange-400" },
      { part: "papaya", x: 63, y: 55, scale: 2.4 },
    ],
    near: [
      { part: "slate-board", x: 14, y: 96, scale: 1 },
      { part: "pebbles", x: 80, y: 95, scale: 1.1, paint: "white", props: true },
    ],
    light: [
      { part: "halo", x: 64, y: 52, scale: 2.2, paint: "orange-400" },
      { part: "halo", x: 40, y: 66, scale: 2, paint: "green-400" },
    ],
    particles: [
      { part: "dust", x: 22, y: 92, scale: 1, paint: "white" },
      { part: "sparks", x: 50, y: 62, scale: 1.1, count: 3, spread: [18, 8], paint: "lime-200" },
      { part: "motes", x: 50, y: 40, scale: 1.4, paint: "green-200" },
    ],
  },
};

/** Fruit Tree (Classic+ #63, Rare): an ancient fruit tree grows through a ruined temple's roof, fruit dropping into a stone basin at a misty sunrise. */
const FRUIT_TREE: SceneRecipe = {
  family: "classic",
  sky: ["blue-800", "orange-400"],
  light: { x: 22, y: 50, r: 32, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 50, y: 50, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 18, y: 44, scale: 0.8 }],
    far: [
      { part: "hills", x: 50, y: 64, paint: "green-900" },
      { part: "mist", x: 50, y: 64, scale: 1, count: 2, spread: [30, 2] },
    ],
    mid: [
      { part: "fallen-column", x: 16, y: 82, scale: 0.8 },
      { part: "ruin-wall", x: 86, y: 80, scale: 0.9, flip: -1 },
    ],
    subject: [
      { part: "fruit-tree", x: 50, y: 74, scale: 2.1 },
      { part: "temple", x: 50, y: 82, scale: 1.25 },
      { part: "grapes", x: 34, y: 30, scale: 1 },
      { part: "grapes", x: 66, y: 28, scale: 0.9 },
      { part: "apple", x: 58, y: 62, scale: 1 },
    ],
    near: [
      { part: "fountain", x: 50, y: 96, scale: 1.1 },
      { part: "grass-tuft", x: 50, y: 97, count: 3, spread: [40, 2] },
      { part: "fern", x: 12, y: 96, scale: 1.2 },
    ],
    light: [
      { part: "light-shaft", x: 24, y: 20, scale: 1, paint: "orange-400" },
      { part: "halo", x: 50, y: 34, scale: 2.4, paint: "orange-400" },
    ],
    particles: [
      { part: "petals", x: 50, y: 50, scale: 1.2, paint: "green-200" },
      { part: "motes", x: 40, y: 60, scale: 1.3, paint: "white" },
    ],
  },
};

/** Mulch Muncher (Classic+ #64, Rare): a hulking boar-like compost beast chomps a heap of fruit peels in a temple garden's mulch pit. */
const MULCH_MUNCHER: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "brass-500"],
  light: { x: 50, y: 56, r: 32, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 82, y: 28, scale: 0.8 }],
    far: [
      { part: "hills", x: 50, y: 62, paint: "green-900" },
      { part: "temple", x: 26, y: 60, scale: 0.6 },
      { part: "tree", x: 80, y: 62, scale: 1 },
    ],
    mid: [
      { part: "field", x: 50, y: 64, paint: "green-700" },
      { part: "grave-mound", x: 50, y: 84, scale: 4.4, paint: "wood-900" },
      { part: "fence", x: 50, y: 66, scale: 0.8, paint: "wood-800" },
    ],
    subject: [
      { part: "boar", x: 45, y: 80, scale: 2.1 },
      { part: "papaya", x: 70, y: 82, scale: 0.8 },
      { part: "apple", x: 76, y: 80, scale: 0.9 },
      { part: "pear", x: 64, y: 84, scale: 0.9 },
    ],
    near: [
      { part: "grass-tuft", x: 50, y: 96, count: 4, spread: [44, 2] },
      { part: "mushroom", x: 12, y: 94, scale: 1.2 },
      { part: "worm", x: 86, y: 94, scale: 1.2 },
    ],
    light: [{ part: "rim-light", x: 46, y: 70, scale: 1.4, paint: "orange-400", flip: -1 }],
    particles: [
      { part: "dust", x: 50, y: 86, scale: 1.4, paint: "wood-600" },
      { part: "bee", x: 70, y: 62, scale: 1, count: 3, spread: [10, 6], paint: "slate-900" },
    ],
  },
};

/** Two Grapes (Classic+ #65, Rare): a green and a purple grape roll like dice across a stone gaming table in a candlelit wine cellar. */
const TWO_GRAPES: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-900"],
  light: { x: 50, y: 56, r: 30, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 50, y: 76, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 62, paint: "slate-800" },
      { part: "arch", x: 18, y: 62, scale: 1.3, paint: "slate-700" },
      { part: "arch", x: 82, y: 62, scale: 1.3, paint: "slate-700" },
      { part: "drum", x: 16, y: 62, scale: 1, paint: "wood-700" },
      { part: "drum", x: 84, y: 62, scale: 1, paint: "wood-700" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 62, paint: "slate-800" },
      { part: "table", x: 50, y: 86, scale: 2.6 },
    ],
    subject: [
      { part: "orb", x: 42, y: 62, scale: 0.75, paint: "green-500" },
      { part: "orb", x: 58, y: 60, scale: 0.8, paint: "violet-700" },
      { part: "dice", x: 50, y: 66, scale: 1 },
    ],
    near: [
      { part: "candle", x: 14, y: 90, scale: 1.4 },
      { part: "coins", x: 84, y: 94, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 14, y: 84, scale: 1.4, paint: "orange-400" },
      { part: "halo", x: 50, y: 60, scale: 2, paint: "violet-500" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 56, scale: 0.9, count: 2, spread: [12, 4], paint: "white" },
      { part: "motes", x: 50, y: 44, scale: 1.2, paint: "orange-400" },
    ],
  },
};

/** Rotten Grape (Classic+ #65.1, Token): a shrivelled mouldy grape oozing green fumes on a cracked stone plate, a fly circling it. */
const ROTTEN_GRAPE: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "green-900"],
  light: { x: 50, y: 58, r: 30, paint: "lime-400", opacity: 0.35 },
  emblem: { x: 50, y: 78, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "room-wall", x: 50, y: 60, paint: "slate-800" }],
    mid: [
      { part: "table", x: 50, y: 88, scale: 2.6 },
      { part: "rune-circle", x: 50, y: 70, scale: 0.9, paint: "stone-500" },
    ],
    subject: [
      { part: "orb", x: 50, y: 64, scale: 0.8, paint: "green-900" },
      { part: "smoke", x: 50, y: 58, scale: 1.2, paint: "green-400" },
      { part: "bee", x: 64, y: 50, scale: 1.4, paint: "slate-900" },
    ],
    near: [{ part: "spores", x: 50, y: 88, scale: 1.4, paint: "lime-400" }],
    particles: [{ part: "spores", x: 50, y: 50, scale: 1.4, paint: "green-400" }],
  },
};

/** Normal Grape (Classic+ #65.2, Token): a single plain purple grape on a vine leaf, resting on a sunny temple windowsill. */
const NORMAL_GRAPE: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 30, y: 32, r: 30, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 80, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 30, scale: 0.8, count: 2, spread: [20, 4] }],
    far: [
      { part: "hills", x: 50, y: 62, paint: "green-700" },
      { part: "temple", x: 72, y: 60, scale: 0.5 },
    ],
    mid: [
      { part: "arch", x: 50, y: 74, scale: 2.8, paint: "stone-500" },
      { part: "ruin-wall", x: 50, y: 90, scale: 2.8 },
    ],
    subject: [
      { part: "fern", x: 50, y: 67, scale: 1.8, paint: "green-500" },
      { part: "orb", x: 50, y: 61, scale: 1, paint: "violet-700" },
    ],
    near: [{ part: "flowers", x: 16, y: 94, scale: 1.2 }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.3, paint: "white" }],
  },
};

/** Large Grape (Classic+ #65.3, Token): a boulder-sized purple grape carted by two farmers along a vineyard road in a warm afternoon. */
const LARGE_GRAPE: SceneRecipe = {
  family: "classic",
  sky: ["blue-800", "orange-400"],
  light: { x: 24, y: 40, r: 30, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 20, y: 30, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 24, y: 86, scale: 0.62, pose: "pose-run", look: { ...figureLookFor(["Human"]), hair: "hat-brim", item: null, cloth: "green-700" } },
    { layer: "subject", x: 80, y: 86, scale: 0.6, flip: -1, pose: "pose-guard", look: { ...figureLookFor(["Human"]), hair: "kerchief", outfit: "outfit-apron", item: null, cloth: "wood-700" } },
  ],
  layers: {
    sky: [{ part: "sun", x: 20, y: 30, scale: 0.8 }],
    far: [
      { part: "hills", x: 50, y: 58, paint: "green-900" },
      { part: "vine", x: 50, y: 62, scale: 0.6, count: 6, spread: [44, 1] },
    ],
    mid: [{ part: "street", x: 50, y: 62, paint: "wood-700" }],
    subject: [
      { part: "cart", x: 50, y: 86, scale: 2.4 },
      { part: "orb", x: 50, y: 53, scale: 2.2, paint: "violet-700" },
    ],
    near: [{ part: "grass-tuft", x: 50, y: 97, count: 4, spread: [44, 2] }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "orange-400" }],
  },
};

/** Golden Grape (Classic+ #65.4, Token): one small glowing grape on a navy velvet cushion, the only warm spot in a shadowed jeweller's workshop. */
const GOLDEN_GRAPE: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-900"],
  light: { x: 50, y: 60, r: 24, paint: "orange-400", opacity: 0.5 },
  emblem: { x: 18, y: 46, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 62, paint: "slate-900" },
      { part: "cabinet", x: 16, y: 62, scale: 1.4 },
      { part: "bookshelf", x: 86, y: 62, scale: 1.3 },
    ],
    mid: [
      { part: "table", x: 50, y: 88, scale: 2.6 },
      { part: "grave-mound", x: 50, y: 70, scale: 2.6, paint: "blue-900" },
    ],
    subject: [{ part: "orb", x: 50, y: 62, scale: 0.75, paint: "brass-500" }],
    near: [
      { part: "gems", x: 22, y: 92, scale: 1.1 },
      { part: "hourglass", x: 82, y: 94, scale: 1 },
    ],
    light: [{ part: "halo", x: 50, y: 63, scale: 1.6, paint: "orange-400" }],
    particles: [{ part: "sparks", x: 50, y: 60, scale: 0.9, count: 2, spread: [6, 4], paint: "orange-400" }],
  },
};

/** Mythic Grape (Classic+ #65.5, Token): a luminous violet grape hovers over an ancient vineyard altar, terraced hills of vines swirling into it. */
const MYTHIC_GRAPE: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 42, r: 36, paint: "violet-500", opacity: 0.5 },
  emblem: { x: 50, y: 76, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 40, y: 22, scale: 1.4 }],
    far: [
      { part: "hills", x: 50, y: 56, paint: "green-900" },
      { part: "vine", x: 50, y: 60, scale: 0.5, count: 6, spread: [44, 1] },
      { part: "fence", x: 50, y: 70, scale: 2, paint: "stone-500" },
      { part: "vine", x: 50, y: 70, scale: 0.6, count: 5, spread: [44, 1] },
    ],
    mid: [{ part: "altar", x: 50, y: 84, scale: 1.8 }],
    subject: [
      { part: "void-swirl", x: 50, y: 46, scale: 1.1, paint: "violet-700" },
      { part: "light-ribbon", x: 50, y: 46, scale: 0.9, paint: "violet-300" },
      { part: "orb", x: 50, y: 46, scale: 0.9, paint: "violet-500" },
    ],
    light: [{ part: "halo", x: 50, y: 46, scale: 2.6, paint: "purple-400" }],
    particles: [{ part: "sparks", x: 50, y: 46, scale: 1, count: 2, spread: [16, 8], paint: "violet-300" }],
  },
};

/** Vine of Grapes (Classic+ #66, Rare): a grapevine of grapes of every colour and size climbs a crumbling temple pillar, a gambler reaching up to pick one. */
const VINE_OF_GRAPES: SceneRecipe = {
  family: "classic",
  sky: ["blue-800", "orange-400"],
  light: { x: 30, y: 36, r: 30, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 38, y: 78, size: 5, rotate: 0 },
  figure: {
    x: 62,
    y: 87,
    scale: 0.86,
    flip: -1,
    pose: "pose-raise",
    look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-coat", item: null, cloth: "red-900" },
  },
  layers: {
    far: [
      { part: "hills", x: 50, y: 64, paint: "green-900" },
      { part: "ruin-wall", x: 82, y: 66, scale: 1 },
      { part: "temple", x: 80, y: 60, scale: 0.5 },
    ],
    mid: [{ part: "field", x: 50, y: 66, paint: "green-700" }],
    subject: [
      { part: "column", x: 38, y: 86, scale: 2.5 },
      { part: "vine", x: 38, y: 86, scale: 2.4 },
      { part: "grapes", x: 32, y: 30, scale: 1.3, paint: "violet-500" },
      { part: "grapes", x: 46, y: 36, scale: 0.8, paint: "green-400" },
      { part: "grapes", x: 30, y: 50, scale: 1, paint: "red-600" },
      { part: "grapes", x: 44, y: 58, scale: 0.7, paint: "orange-400" },
    ],
    near: [
      { part: "dice", x: 80, y: 94, scale: 1.2 },
      { part: "coins", x: 18, y: 95, scale: 1.1 },
      { part: "grass-tuft", x: 50, y: 97, count: 3, spread: [40, 2] },
    ],
    light: [
      { part: "light-shaft", x: 30, y: 0, scale: 1, paint: "orange-400", count: 2, spread: [20, 0] },
      { part: "halo", x: 46, y: 40, scale: 1.6, paint: "orange-400" },
    ],
    particles: [
      { part: "petals", x: 50, y: 40, scale: 1.2, paint: "green-700" },
      { part: "motes", x: 50, y: 50, scale: 1.3, paint: "orange-400" },
    ],
  },
};

/** Pear (Classic+ #67, Rare): a green pear split open on a herb-garden bench at a hillside hermitage, two tiny friendly creatures climbing out and waving. */
const PEAR: SceneRecipe = {
  family: "classic",
  sky: ["blue-800", "green-200"],
  light: { x: 30, y: 34, r: 30, paint: "green-200", opacity: 0.4 },
  emblem: { x: 50, y: 78, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 42, y: 62, scale: 0.22, pose: "pose-spread", look: { ...figureLookFor(["Fruit"]), skin: "lime-400", cloth: "green-500" } },
    { layer: "subject", x: 58, y: 61, scale: 0.2, flip: -1, pose: "pose-raise", look: { ...figureLookFor(["Fruit"]), skin: "lime-200", cloth: "green-400" } },
  ],
  layers: {
    sky: [{ part: "cloud", x: 26, y: 24, scale: 0.7 }],
    far: [
      { part: "hills", x: 50, y: 58, paint: "green-700" },
      { part: "house", x: 82, y: 58, scale: 0.7 },
      { part: "tree", x: 14, y: 60, scale: 0.9 },
    ],
    mid: [
      { part: "field", x: 50, y: 62, paint: "green-700" },
      { part: "flowers", x: 20, y: 74, scale: 1.1, count: 3, spread: [10, 2] },
      { part: "fern", x: 84, y: 74, scale: 1.1 },
      { part: "table", x: 50, y: 84, scale: 1.8 },
    ],
    subject: [
      { part: "pear", x: 44, y: 70, scale: 2.6 },
      { part: "pear", x: 56, y: 70, scale: 2.6, flip: -1 },
    ],
    near: [
      { part: "flowers", x: 12, y: 95, scale: 1.2 },
      { part: "grass-tuft", x: 50, y: 97, count: 4, spread: [44, 2] },
    ],
    light: [{ part: "halo", x: 50, y: 62, scale: 2, paint: "green-200" }],
    particles: [
      { part: "petals", x: 50, y: 40, scale: 1.2, paint: "red-300" },
      { part: "bee", x: 76, y: 60, scale: 1 },
    ],
  },
};

/** A robed farmer of Organic Produce's terraces. */
const TERRACE_FARMER: FigureLook = { ...figureLookFor([]), outfit: "outfit-robe", cloth: "wood-600" };

/** Organic Produce (Classic+ #68, Epic): a terraced temple farm market, crates of glistening fruit before robed farmers tending berries and melons among bees. */
const ORGANIC_PRODUCE: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 26, y: 30, r: 30, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 60, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 30, y: 82, scale: 0.55, pose: "pose-kneel", look: TERRACE_FARMER },
    { layer: "subject", x: 72, y: 80, scale: 0.55, flip: -1, pose: "pose-stand", look: { ...TERRACE_FARMER, hair: "hat-brim", item: "item-sack", cloth: "green-900" } },
  ],
  layers: {
    sky: [{ part: "cloud", x: 70, y: 22, scale: 0.7 }],
    far: [
      { part: "hills", x: 50, y: 44, paint: "green-700" },
      { part: "fence", x: 50, y: 46, scale: 1.2, paint: "stone-500" },
      { part: "fence", x: 50, y: 54, scale: 1.4, paint: "stone-500" },
      { part: "temple", x: 70, y: 38, scale: 0.4 },
      { part: "bush", x: 30, y: 50, scale: 0.6, count: 2, spread: [24, 2] },
    ],
    mid: [
      { part: "field", x: 50, y: 60, paint: "green-700" },
      { part: "bush", x: 20, y: 72, scale: 1, count: 2, spread: [8, 2] },
      { part: "bush", x: 82, y: 70, scale: 1 },
    ],
    subject: [
      { part: "awning", x: 50, y: 78, scale: 1.4 },
      { part: "orb", x: 38, y: 82, scale: 0.4, paint: "green-500" },
      { part: "orb", x: 62, y: 83, scale: 0.4, paint: "green-500" },
    ],
    near: [
      { part: "crate", x: 16, y: 98, scale: 2.4 },
      { part: "apple", x: 13, y: 82, scale: 1.2, count: 3, spread: [4, 1] },
      { part: "crate", x: 84, y: 98, scale: 2.4 },
      { part: "pear", x: 84, y: 82, scale: 1.2, count: 2, spread: [4, 1] },
    ],
    light: [
      { part: "halo", x: 50, y: 66, scale: 2.2, paint: "white" },
      { part: "light-shaft", x: 24, y: 0, scale: 1, paint: "white" },
    ],
    particles: [
      { part: "bee", x: 50, y: 64, scale: 1, count: 3, spread: [30, 10] },
      { part: "sparks", x: 50, y: 80, scale: 0.8, count: 2, spread: [34, 1], paint: "white" },
    ],
  },
};

/** Buff Billy (Classic+ #69, Rare, full art): a hugely muscled man on skinny legs flexing on a riverside quarry ledge beside cracked stone weights. */
const BUFF_BILLY: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "orange-400"],
  light: { x: 50, y: 42, r: 34, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 84, y: 60, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 1.05,
    pose: "pose-flex",
    look: { ...figureLookFor(["Human"]), hair: "kerchief", outfit: "outfit-muscle", item: null, cloth: "red-600" },
  },
  layers: {
    sky: [{ part: "sun", x: 80, y: 24, scale: 0.9 }],
    far: [
      { part: "cliff", x: 26, y: 50, scale: 0.8 },
      { part: "cliff", x: 76, y: 52, scale: 0.7, flip: -1 },
      { part: "water", x: 50, y: 66, paint: "blue-800" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 76, paint: "stone-500" },
      { part: "standing-stone", x: 84, y: 74, scale: 1 },
      { part: "weights", x: 24, y: 82, scale: 1.2, props: true },
      { part: "weights", x: 76, y: 84, scale: 1.1, props: true },
    ],
    subject: [{ part: "halo", x: 50, y: 50, scale: 3, paint: "orange-400" }],
    near: [
      { part: "pebbles", x: 30, y: 96, scale: 1.2 },
      { part: "pebbles", x: 74, y: 97, scale: 1 },
    ],
    light: [{ part: "rim-light", x: 50, y: 58, scale: 1.4, paint: "orange-400" }],
    particles: [
      { part: "dust", x: 50, y: 86, scale: 1.4, paint: "brass-500" },
      { part: "sparks", x: 50, y: 36, scale: 1, count: 2, spread: [12, 4], paint: "sky-300" },
    ],
  },
};

/** Chaos Machine (Classic+ #70, Rare): a clanking contraption of gears and pistons in a temple undercroft swells the crates on one side and shrinks the other's. */
const CHAOS_MACHINE: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 50, r: 32, paint: "violet-500", opacity: 0.5 },
  emblem: { x: 50, y: 26, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 62, paint: "slate-800" },
      { part: "arch", x: 16, y: 62, scale: 1.3, paint: "slate-700" },
      { part: "arch", x: 84, y: 62, scale: 1.3, paint: "slate-700" },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 62, paint: "slate-700" }],
    subject: [
      { part: "pipes", x: 50, y: 80, scale: 1.2 },
      { part: "gear", x: 44, y: 54, scale: 1.2 },
      { part: "gear", x: 58, y: 46, scale: 0.8 },
      { part: "gear", x: 56, y: 62, scale: 0.6 },
      { part: "bellows", x: 34, y: 70, scale: 0.8, flip: -1 },
      { part: "bellows", x: 66, y: 70, scale: 0.8 },
    ],
    near: [
      { part: "crate", x: 14, y: 96, scale: 2.6 },
      { part: "crate", x: 84, y: 92, scale: 0.6, count: 3, spread: [4, 1] },
    ],
    light: [
      { part: "lightning", x: 30, y: 16, scale: 0.7, paint: "violet-300" },
      { part: "halo", x: 50, y: 56, scale: 2.2, paint: "violet-500" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 50, scale: 1, count: 3, spread: [16, 10], paint: "violet-300" },
      { part: "smoke", x: 64, y: 40, scale: 0.8, paint: "slate-400" },
    ],
  },
};

/** Book of Buff (Classic+ #71, Epic): an iron-bound tome thuds onto a cliffside pedestal, its beam swelling a stone watchtower and its guard to twice their size. */
const BOOK_OF_BUFF: SceneRecipe = {
  family: "classic",
  sky: ["red-600", "orange-400"],
  light: { x: 28, y: 52, r: 34, paint: "orange-400", opacity: 0.5 },
  emblem: { x: 30, y: 78, size: 5, rotate: 0 },
  extras: [
    {
      layer: "subject",
      x: 56,
      y: 86,
      scale: 1,
      pose: "pose-guard",
      look: { ...figureLookFor(["Human"]), hair: "helm", outfit: "outfit-armor", item: "item-spear", cloth: "slate-400" },
    },
  ],
  layers: {
    sky: [{ part: "sun", x: 18, y: 54, scale: 1 }],
    far: [
      { part: "mountains", x: 50, y: 60, paint: "slate-700" },
      { part: "cloud", x: 70, y: 26, scale: 0.8, paint: "red-300" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 72, paint: "stone-500" },
      { part: "tower", x: 72, y: 82, scale: 1.6 },
      { part: "altar", x: 30, y: 84, scale: 1.4 },
    ],
    subject: [
      { part: "tome", x: 30, y: 72, scale: 1.3, paint: "slate-700" },
      { part: "light-ribbon", x: 46, y: 62, scale: 0.8, paint: "orange-400" },
      { part: "shockwave", x: 30, y: 72, scale: 0.6, paint: "orange-400" },
    ],
    near: [
      { part: "petals", x: 20, y: 92, scale: 1.2, paint: "red-600" },
      { part: "scroll", x: 82, y: 95, scale: 1 },
      { part: "pebbles", x: 50, y: 97, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 30, y: 66, scale: 2.2, paint: "orange-400" },
      { part: "rim-light", x: 56, y: 58, scale: 1.3, paint: "orange-400", flip: -1 },
      { part: "god-rays", x: 18, y: 54, scale: 0.8, paint: "orange-400" },
    ],
    particles: [
      { part: "sparks", x: 44, y: 58, scale: 1.1, count: 3, spread: [16, 8], paint: "orange-400" },
      { part: "embers", x: 30, y: 66, scale: 1.4, count: 2, spread: [8, 4] },
    ],
  },
};

/** Book of Nerf (Classic+ #72, Epic): a dusty grey tome opens on a sunken crypt lectern and its downward beam shrinks a proud stone giant to a tiny figure. */
const BOOK_OF_NERF: SceneRecipe = {
  family: "classic",
  sky: ["slate-800", "slate-700"],
  light: { x: 58, y: 40, r: 32, paint: "blue-200", opacity: 0.35 },
  emblem: { x: 32, y: 74, size: 5, rotate: 0 },
  extras: [
    { layer: "mid", x: 60, y: 84, scale: 1.05, pose: "pose-spread", look: { ...figureLookFor([]), head: "head-golem", hair: null, outfit: "outfit-stone", skin: "slate-700", cloth: "slate-700" } },
    { layer: "subject", x: 60, y: 84, scale: 0.34, pose: "pose-spread", look: { ...figureLookFor([]), head: "head-golem", hair: null, outfit: "outfit-stone", skin: "stone-500", cloth: "slate-400" } },
  ],
  layers: {
    far: [
      { part: "arch", x: 16, y: 62, scale: 1.4, paint: "slate-700" },
      { part: "arch", x: 84, y: 62, scale: 1.4, paint: "slate-700" },
      { part: "ruin-wall", x: 50, y: 62, scale: 1.6, paint: "slate-700" },
      { part: "column", x: 32, y: 62, scale: 0.9 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 62, paint: "slate-700" }],
    subject: [
      { part: "lectern", x: 32, y: 88, scale: 3 },
      { part: "tome", x: 33, y: 59, scale: 1.2, paint: "slate-400" },
      { part: "shockwave", x: 60, y: 84, scale: 0.5, paint: "blue-200" },
    ],
    near: [
      { part: "banner", x: 8, y: 98, scale: 1.2, paint: "slate-700" },
      { part: "banner", x: 92, y: 98, scale: 1.2, flip: -1, paint: "slate-700" },
      { part: "pebbles", x: 50, y: 97, scale: 1.2 },
    ],
    light: [
      { part: "light-shaft", x: 62, y: 14, scale: 1.1, paint: "blue-200" },
      { part: "halo", x: 34, y: 54, scale: 1.6, paint: "blue-200" },
    ],
    particles: [
      { part: "dust", x: 50, y: 50, scale: 1.4, paint: "slate-400" },
      { part: "motes", x: 60, y: 60, scale: 1.3, paint: "blue-200" },
    ],
  },
};

/** Call to Chaos (Classic+ Edition) (Classic+ #73, Legendary): a hooded gambler yanks the lever of a towering stone wheel of fortune that spews fruit, books, a fist, lightning and cards. */
const CALL_TO_CHAOS: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 44, y: 42, r: 40, paint: "violet-500", opacity: 0.5 },
  emblem: { x: 44, y: 44, size: 5, rotate: 0 },
  figure: {
    x: 80,
    y: 86,
    scale: 0.78,
    flip: -1,
    pose: "pose-raise",
    look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-robe", item: null, cloth: "violet-700" },
  },
  layers: {
    sky: [{ part: "stars", x: 40, y: 20, scale: 1.4 }],
    far: [
      { part: "room-wall", x: 50, y: 64, paint: "slate-800" },
      { part: "column", x: 10, y: 70, scale: 1.6 },
      { part: "column", x: 90, y: 70, scale: 1.6 },
      { part: "ruin-wall", x: 50, y: 66, scale: 1.4, paint: "slate-700" },
    ],
    mid: [
      { part: "tile-floor", x: 44, y: 64, paint: "slate-800" },
      { part: "lightning", x: 26, y: 18, scale: 0.8, paint: "violet-300" },
      { part: "lightning", x: 64, y: 16, scale: 0.7, paint: "jade-400" },
    ],
    subject: [
      { part: "fortune-wheel", x: 44, y: 44, scale: 1.6 },
      { part: "apple", x: 20, y: 34, scale: 1.4 },
      { part: "pear", x: 68, y: 26, scale: 1.3 },
      { part: "tome", x: 18, y: 48, scale: 0.6, paint: "blue-800" },
      { part: "tome", x: 70, y: 40, scale: 0.6, paint: "red-900" },
      { part: "knuckles", x: 30, y: 70, scale: 1.6, paint: "stone-500" },
      { part: "card", x: 56, y: 70, scale: 0.8, count: 3, spread: [10, 6] },
    ],
    near: [
      { part: "dice", x: 24, y: 92, scale: 1.3, count: 2, spread: [16, 3] },
      { part: "fallen-column", x: 64, y: 96, scale: 0.8 },
      { part: "pebbles", x: 46, y: 97, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 44, y: 44, scale: 3, paint: "violet-500" },
      { part: "halo", x: 44, y: 44, scale: 1.6, paint: "jade-400" },
      { part: "god-rays", x: 44, y: 10, scale: 0.8, paint: "violet-300" },
      { part: "rim-light", x: 84, y: 60, scale: 1.1, paint: "jade-400", flip: -1 },
    ],
    particles: [
      { part: "sparks", x: 44, y: 44, scale: 1.2, count: 4, spread: [24, 18], paint: "violet-300" },
      { part: "sparks", x: 44, y: 44, scale: 1, count: 3, spread: [24, 18], paint: "jade-400" },
      { part: "motes", x: 50, y: 60, scale: 1.4, paint: "violet-300" },
    ],
  },
};

/** Classic Golem (Classic+ #73.1, Token): a massive temple-stone golem mid-punch in a ruined plaza, carved faces shifting across its chest, in a dust storm. */
const CLASSIC_GOLEM: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "stone-500"],
  light: { x: 50, y: 50, r: 32, paint: "slate-100", opacity: 0.3 },
  emblem: { x: 16, y: 66, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 88,
    scale: 1.15,
    pose: "pose-run",
    look: { ...figureLookFor([]), head: "head-golem", hair: null, outfit: "outfit-stone", item: null, skin: "stone-500", cloth: "slate-700" },
  },
  layers: {
    far: [
      { part: "ruin-wall", x: 16, y: 66, scale: 1 },
      { part: "column", x: 84, y: 66, scale: 1.2 },
      { part: "tornado", x: 82, y: 62, scale: 1, paint: "stone-500" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 64, paint: "stone-500" },
      { part: "fallen-column", x: 76, y: 80, scale: 0.9 },
    ],
    subject: [{ part: "dust", x: 50, y: 88, scale: 1.6, paint: "stone-500" }],
    near: [{ part: "pebbles", x: 50, y: 96, scale: 1.4, count: 2, spread: [30, 2] }],
    particles: [{ part: "dust", x: 50, y: 50, scale: 1.4, count: 2, spread: [30, 20], paint: "slate-400" }],
  },
};

/** The enemy soldiers marching up Twice Forward One Step Backwards' stair. */
const STAIR_SOLDIER: FigureLook = { ...figureLookFor(["Human"]), hair: "visor", outfit: "outfit-armor", item: "item-spear", skin: "red-300", cloth: "red-900" };

/** Twice Forward One Step Backwards (Classic+ #74, Mythic): an army marches up a clockwork stair that grinds back, its spells glued into a cracking monolith over the void. */
const TWICE_FORWARD: SceneRecipe = {
  family: "classic",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 70, r: 44, paint: "violet-500", opacity: 0.55 },
  emblem: { x: 50, y: 40, size: 6, rotate: 0 },
  extras: [
    { layer: "subject", x: 30, y: 84, scale: 0.42, pose: "pose-run", look: STAIR_SOLDIER },
    { layer: "subject", x: 40, y: 76, scale: 0.36, pose: "pose-run", look: { ...STAIR_SOLDIER, item: "item-sword" } },
    { layer: "subject", x: 64, y: 80, scale: 0.4, flip: -1, pose: "pose-run", look: STAIR_SOLDIER },
    { layer: "subject", x: 70, y: 90, scale: 0.46, flip: -1, pose: "pose-raise", look: { ...STAIR_SOLDIER, item: "item-staff", hair: "hood" } },
  ],
  layers: {
    sky: [{ part: "void-swirl", x: 50, y: 70, scale: 3 }],
    far: [
      { part: "gear", x: 14, y: 30, scale: 2.4, paint: "slate-700" },
      { part: "gear", x: 86, y: 26, scale: 2, paint: "slate-700" },
      { part: "clock-face", x: 50, y: 22, scale: 0.9 },
    ],
    mid: [
      { part: "stairs", x: 50, y: 50, paint: "stone-500" },
      { part: "gear", x: 18, y: 66, scale: 1.4, paint: "brass-600" },
      { part: "gear", x: 84, y: 70, scale: 1.6, paint: "brass-600" },
    ],
    subject: [
      { part: "standing-stone", x: 50, y: 64, scale: 2, paint: "slate-400" },
      { part: "lightning", x: 48, y: 36, scale: 0.6, paint: "cyan-400" },
      { part: "fireball", x: 56, y: 48, scale: 0.5 },
      { part: "lightning", x: 54, y: 52, scale: 0.4, paint: "slate-100" },
    ],
    near: [
      { part: "gear", x: 90, y: 94, scale: 1.6, paint: "slate-400" },
    ],
    light: [
      { part: "halo", x: 50, y: 48, scale: 2.4, paint: "cyan-400" },
      { part: "halo", x: 50, y: 80, scale: 3, paint: "violet-500" },
      { part: "rim-light", x: 50, y: 50, scale: 1.4, paint: "cyan-400" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 48, scale: 1.2, paint: "cyan-400" },
      { part: "embers", x: 50, y: 60, scale: 1.4, paint: "cyan-400" },
    ],
  },
};

/** J-lease J-Jungle EX-plorer (Classic+ #75, Legendary, full art): J-lease swings on a vine through a collapsed jungle temple, tossing her glinting pack to a companion below. */
const J_LEASE: SceneRecipe = {
  family: "classic",
  sky: ["green-900", "jade-400"],
  light: { x: 40, y: 36, r: 36, paint: "green-400", opacity: 0.45 },
  emblem: { x: 84, y: 54, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 64,
    scale: 0.8,
    pose: "pose-spread",
    look: { ...figureLookFor([]), hair: "hat-brim", outfit: "outfit-patched", item: "item-sack", skin: "red-300", cloth: "brass-600" },
  },
  extras: [{ layer: "mid", x: 76, y: 88, scale: 0.55, flip: -1, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "kerchief", outfit: "outfit-jacket", item: null, cloth: "wood-600" } }],
  layers: {
    sky: [{ part: "tree", x: 50, y: 30, scale: 2.4, count: 3, spread: [40, 4], paint: "green-700" }],
    far: [
      { part: "temple", x: 50, y: 74, scale: 1.3 },
      { part: "statue", x: 14, y: 76, scale: 1.5 },
      { part: "statue", x: 86, y: 74, scale: 1.4, flip: -1 },
      { part: "monkey", x: 18, y: 42, scale: 1.2 },
      { part: "monkey", x: 82, y: 36, scale: 1.1, flip: -1 },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 76, paint: "green-900" },
      { part: "gems", x: 62, y: 78, scale: 0.8, props: true },
      { part: "coins", x: 68, y: 66, scale: 0.6, props: true },
    ],
    subject: [{ part: "vine", x: 36, y: 26, scale: 2.4, paint: "green-500" }],
    near: [
      { part: "fern", x: 8, y: 98, scale: 2 },
      { part: "grass-tuft", x: 88, y: 98, scale: 1.4, count: 2, spread: [6, 1] },
      { part: "vine", x: 94, y: 90, scale: 1.4 },
    ],
    light: [
      { part: "light-shaft", x: 30, y: 0, scale: 1, paint: "green-400", count: 3, spread: [24, 0] },
      { part: "halo", x: 50, y: 50, scale: 2.4, paint: "green-400" },
      { part: "rim-light", x: 50, y: 48, scale: 1.2, paint: "lime-200", flip: -1 },
    ],
    particles: [
      { part: "petals", x: 50, y: 40, scale: 1.4, paint: "green-400" },
      { part: "sparks", x: 64, y: 30, scale: 1, paint: "white" },
      { part: "motes", x: 40, y: 30, scale: 1.4, paint: "lime-200" },
    ],
  },
};

/** J-lease J-Jungle EX-plorer Pack (Classic+ #75.1, Token): a battered explorer's pack spilling shiny relics onto mossy temple steps in dappled jungle light. */
const J_LEASE_PACK: SceneRecipe = {
  family: "classic",
  sky: ["green-900", "jade-400"],
  light: { x: 40, y: 40, r: 30, paint: "lime-200", opacity: 0.4 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "tree", x: 50, y: 40, scale: 2, count: 3, spread: [40, 4], paint: "green-700" }],
    mid: [{ part: "stairs", x: 50, y: 46, paint: "stone-500" }],
    subject: [
      { part: "chest", x: 46, y: 70, scale: 1.6, paint: "brass-800" },
      { part: "gems", x: 52, y: 76, scale: 1 },
      { part: "coins", x: 62, y: 80, scale: 0.9 },
    ],
    near: [{ part: "fern", x: 12, y: 96, scale: 1.6 }],
    light: [{ part: "light-shaft", x: 30, y: 0, scale: 1, paint: "lime-200", count: 2, spread: [20, 0] }],
    particles: [{ part: "sparks", x: 52, y: 64, scale: 1, count: 2, spread: [10, 6], paint: "white" }],
  },
};

/** Brother Lar (Classic+ #76, Rare): a young monk in grey robes waves goodbye at a misty mountain temple gate as his brother's shadow steps up behind. */
const BROTHER_LAR: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "slate-400"],
  light: { x: 30, y: 40, r: 32, paint: "slate-100", opacity: 0.35 },
  emblem: { x: 30, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 46,
    y: 87,
    scale: 1,
    pose: "pose-raise",
    look: { ...figureLookFor(["CN", "Human"]), hair: "tonsure", outfit: "outfit-robe", item: null, cloth: "slate-400" },
  },
  extras: [{ layer: "mid", x: 64, y: 84, scale: 1, pose: "pose-stand", look: { ...figureLookFor([]), hair: "tonsure", outfit: "outfit-robe", skin: "slate-800", cloth: "slate-800" } }],
  layers: {
    far: [
      { part: "mountains", x: 50, y: 56, paint: "slate-700" },
      { part: "temple", x: 70, y: 48, scale: 0.5 },
      { part: "mist", x: 50, y: 56, scale: 1, count: 2, spread: [30, 2] },
    ],
    mid: [
      { part: "stairs", x: 50, y: 64, paint: "stone-500" },
      { part: "gate", x: 50, y: 70, scale: 2.2 },
    ],
    near: [
      { part: "mist", x: 50, y: 90, scale: 1.2, count: 2, spread: [30, 3] },
      { part: "grass-tuft", x: 50, y: 97, count: 3, spread: [40, 2] },
    ],
    light: [{ part: "halo", x: 30, y: 40, scale: 2, paint: "slate-100" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.3, paint: "white" }],
  },
};

/** Brother Ping (Classic+ #76.1, Token): a lean monk flicks pebbles from a sling at targets across a bamboo-fenced temple yard. */
const BROTHER_PING: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 30, y: 40, r: 30, paint: "white", opacity: 0.35 },
  emblem: { x: 76, y: 56, size: 5, rotate: 0 },
  figure: {
    x: 40,
    y: 87,
    scale: 1,
    pose: "pose-guard",
    look: { ...figureLookFor(["CN", "Human"]), hair: "hair-short", outfit: "outfit-wraps", item: "item-sling", cloth: "wood-600" },
  },
  layers: {
    far: [
      { part: "temple", x: 30, y: 60, scale: 0.6 },
      { part: "fence", x: 50, y: 66, scale: 1.4, paint: "green-700" },
    ],
    mid: [
      { part: "field", x: 50, y: 66, paint: "stone-500" },
      { part: "drum", x: 76, y: 68, scale: 0.8, paint: "red-600" },
      { part: "drum", x: 88, y: 70, scale: 0.6, paint: "red-600" },
    ],
    subject: [{ part: "pebbles", x: 64, y: 54, scale: 0.6 }],
    near: [{ part: "grass-tuft", x: 50, y: 97, count: 4, spread: [44, 2] }],
    particles: [{ part: "sparks", x: 66, y: 54, scale: 0.8, count: 2, spread: [8, 2], paint: "white" }],
  },
};

/** Anti-Softlock (Classic+ #77, Epic): a key of white light turns in a gatehouse of a thousand padlocks, chains bursting, doors flung open down a corridor of gates. */
const ANTI_SOFTLOCK: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "sky-300"],
  light: { x: 50, y: 48, r: 40, paint: "blue-200", opacity: 0.55 },
  emblem: { x: 76, y: 64, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 60, paint: "slate-700" },
      { part: "arch", x: 50, y: 56, scale: 0.6 },
      { part: "arch", x: 50, y: 58, scale: 0.9 },
      { part: "arch", x: 50, y: 62, scale: 1.3 },
      { part: "padlock", x: 20, y: 36, scale: 0.8, count: 5, spread: [12, 12] },
      { part: "padlock", x: 80, y: 36, scale: 0.8, count: 5, spread: [12, 12] },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 60, paint: "stone-500" },
      { part: "doorway", x: 22, y: 78, scale: 2 },
      { part: "doorway", x: 78, y: 78, scale: 2, flip: -1 },
      { part: "chain", x: 26, y: 46, scale: 1 },
      { part: "chain", x: 52, y: 42, scale: 1 },
    ],
    subject: [{ part: "key", x: 50, y: 52, scale: 2.4, paint: "white" }],
    near: [
      { part: "padlock", x: 50, y: 94, scale: 1.1, count: 5, spread: [38, 3] },
      { part: "chain", x: 4, y: 92, scale: 1 },
      { part: "pebbles", x: 70, y: 98, scale: 1.2 },
    ],
    light: [
      { part: "god-rays", x: 50, y: 30, scale: 0.9, paint: "white" },
      { part: "halo", x: 50, y: 52, scale: 2.8, paint: "blue-200" },
      { part: "halo", x: 50, y: 52, scale: 1.4, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 52, scale: 1.2, count: 4, spread: [20, 14], paint: "white" },
      { part: "motes", x: 50, y: 70, scale: 1.4, paint: "blue-200" },
    ],
  },
};

/** Claude's Datacenter (Classic+ #78, Legendary): a vaulted temple turned datacenter, crystal server obelisks under arches, a baker pulling a loaf from a vent-warmed rack, a card forming at the aisle's end. */
const CLAUDES_DATACENTER: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "brass-800"],
  light: { x: 50, y: 44, r: 36, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 50, y: 44, size: 5, rotate: 0 },
  extras: [
    {
      layer: "subject",
      x: 38,
      y: 87,
      scale: 0.82,
      pose: "pose-stand",
      look: { ...figureLookFor(["Human"]), hair: "toque", outfit: "outfit-apron", item: "item-tray", cloth: "red-600" },
    },
  ],
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 52, paint: "wood-700" }],
    far: [
      { part: "arch", x: 50, y: 52, scale: 0.7 },
      { part: "arch", x: 50, y: 56, scale: 1.1 },
      { part: "arch", x: 50, y: 64, scale: 1.8 },
      { part: "card", x: 50, y: 50, scale: 0.9, paint: "orange-400" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 52, paint: "stone-500" },
      { part: "server-rack", x: 34, y: 60, scale: 0.7 },
      { part: "server-rack", x: 26, y: 68, scale: 1 },
      { part: "server-rack", x: 14, y: 80, scale: 1.4 },
      { part: "server-rack", x: 66, y: 60, scale: 0.7 },
      { part: "server-rack", x: 74, y: 68, scale: 1 },
      { part: "server-rack", x: 86, y: 80, scale: 1.4 },
    ],
    subject: [
      { part: "cabinet", x: 60, y: 86, scale: 1.2 },
      { part: "bread", x: 60, y: 69, scale: 1 },
      { part: "chain", x: 30, y: 82, scale: 1, paint: "slate-900" },
    ],
    near: [
      { part: "chain", x: 4, y: 94, scale: 1.2, paint: "slate-900" },
      { part: "chain", x: 62, y: 96, scale: 1.2, paint: "slate-900" },
      { part: "vine", x: 92, y: 98, scale: 1.2, paint: "slate-800" },
    ],
    light: [
      { part: "halo", x: 50, y: 46, scale: 2.4, paint: "orange-400" },
      { part: "halo", x: 20, y: 64, scale: 2, paint: "cyan-400" },
      { part: "halo", x: 80, y: 64, scale: 2, paint: "cyan-400" },
      { part: "rim-light", x: 38, y: 60, scale: 1.1, paint: "orange-400" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 46, scale: 1, count: 3, spread: [10, 8], paint: "orange-400" },
      { part: "motes", x: 50, y: 70, scale: 1.5, count: 2, spread: [20, 6], paint: "cyan-400" },
      { part: "embers", x: 60, y: 64, scale: 1.2, count: 2, spread: [6, 4] },
    ],
  },
};

/** Helpful Assistant (Classic+ T-AI-1, Token): a smiling porcelain automaton butler bowing with a tray of cards in a library antechamber, a heart-shaped glow on its chest. */
const HELPFUL_ASSISTANT: SceneRecipe = {
  family: "classic",
  sky: ["violet-300", "blue-200"],
  light: { x: 50, y: 56, r: 30, paint: "red-300", opacity: 0.35 },
  emblem: { x: 80, y: 34, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 87,
    scale: 1.02,
    pose: "pose-guard",
    look: { ...figureLookFor([]), head: "head-automaton", hair: null, outfit: "outfit-suit", item: "item-tray", skin: "slate-100", cloth: "blue-200" },
  },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 64, paint: "violet-300" },
      { part: "bookshelf", x: 14, y: 64, scale: 1.4 },
      { part: "bookshelf", x: 86, y: 64, scale: 1.4 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 64, paint: "slate-400" }],
    subject: [{ part: "halo", x: 50, y: 52, scale: 1.2, paint: "red-400" }],
    near: [{ part: "card", x: 84, y: 94, scale: 1, count: 2, spread: [4, 1] }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.3, paint: "white" }],
  },
};

/** The crystal construct of Scaling Law, in cyan glass. */
const CRYSTAL_CONSTRUCT: FigureLook = { ...figureLookFor([]), head: "head-golem", hair: null, outfit: "outfit-stone", item: null, skin: "sky-300", cloth: "cyan-400" };

/** Scaling Law (Classic+ T-AI-2, Token): a crystal construct growing visibly taller beside a ladder of its own smaller copies in a stone atrium. */
const SCALING_LAW: SceneRecipe = {
  family: "classic",
  sky: ["slate-800", "blue-800"],
  light: { x: 56, y: 50, r: 32, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 84, y: 36, size: 5, rotate: 0 },
  figure: { x: 60, y: 88, scale: 1.1, pose: "pose-spread", look: CRYSTAL_CONSTRUCT },
  extras: [
    { layer: "mid", x: 16, y: 84, scale: 0.3, pose: "pose-stand", look: CRYSTAL_CONSTRUCT },
    { layer: "mid", x: 25, y: 84, scale: 0.45, pose: "pose-stand", look: CRYSTAL_CONSTRUCT },
    { layer: "mid", x: 36, y: 85, scale: 0.65, pose: "pose-stand", look: CRYSTAL_CONSTRUCT },
  ],
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 64, paint: "stone-500" },
      { part: "column", x: 10, y: 66, scale: 1.4 },
      { part: "column", x: 90, y: 66, scale: 1.4 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 64, paint: "slate-700" }],
    subject: [{ part: "halo", x: 60, y: 54, scale: 2.4, paint: "cyan-400" }],
    light: [{ part: "light-shaft", x: 60, y: 0, scale: 1.1, paint: "cyan-400" }],
    particles: [{ part: "sparks", x: 60, y: 40, scale: 1, count: 2, spread: [12, 10], paint: "sky-300" }],
  },
};

/** Hallucination (Classic+ T-AI-3, Token): a shimmering, cracked mirage of a knight with too many eyes on its shield forms over a desert temple. */
const HALLUCINATION: SceneRecipe = {
  family: "classic",
  sky: ["violet-700", "purple-400"],
  light: { x: 50, y: 40, r: 34, paint: "purple-400", opacity: 0.45 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 66,
    scale: 0.8,
    pose: "pose-guard",
    look: { ...figureLookFor(["Human"]), hair: "helm", outfit: "outfit-armor", item: "item-shield", skin: "violet-300", cloth: "purple-400" },
  },
  layers: {
    far: [{ part: "hills", x: 50, y: 66, paint: "brass-600" }],
    mid: [
      { part: "field", x: 50, y: 74, paint: "brass-500" },
      { part: "temple", x: 50, y: 86, scale: 0.8 },
    ],
    subject: [{ part: "halo", x: 50, y: 40, scale: 2.6, paint: "purple-400" }],
    near: [{ part: "glitch", x: 50, y: 44, scale: 1.2 }],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1.4, paint: "violet-300" }],
  },
};

/** Chain of Thought (Classic+ T-AI-4, Token): a glowing chain of linked thought bubbles spirals up a stone stairwell, each link lighting the next. */
const CHAIN_OF_THOUGHT: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 44, r: 32, paint: "violet-300", opacity: 0.45 },
  emblem: { x: 50, y: 80, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "room-wall", x: 50, y: 40, paint: "slate-800" }],
    mid: [{ part: "stairs", x: 50, y: 36, paint: "stone-500" }],
    subject: [
      { part: "thought-bubble", x: 36, y: 74, scale: 1 },
      { part: "thought-bubble", x: 60, y: 62, scale: 1.1 },
      { part: "thought-bubble", x: 40, y: 48, scale: 1.2 },
      { part: "thought-bubble", x: 62, y: 34, scale: 1.3 },
    ],
    light: [
      { part: "halo", x: 62, y: 34, scale: 1.6, paint: "violet-300" },
      { part: "halo", x: 40, y: 48, scale: 1.2, paint: "violet-300" },
    ],
    particles: [{ part: "sparks", x: 50, y: 50, scale: 1, count: 2, spread: [14, 14], paint: "violet-300" }],
  },
};

/** Autocomplete (Classic+ T-AI-5, Token): a translucent echo finishes a rival mage's spell gesture a heartbeat before he does in a temple duelling hall. */
const AUTOCOMPLETE: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-800"],
  light: { x: 36, y: 50, r: 30, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 84, y: 40, size: 5, rotate: 0 },
  figure: {
    x: 38,
    y: 87,
    scale: 0.95,
    pose: "pose-raise",
    look: { ...figureLookFor(["KY"]), outfit: "outfit-robe", skin: "cyan-400", cloth: "sky-300" },
  },
  extras: [{ layer: "subject", x: 66, y: 87, scale: 0.95, flip: -1, pose: "pose-raise", look: { ...figureLookFor(["KY"]), outfit: "outfit-robe", cloth: "violet-700" } }],
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 64, paint: "slate-800" },
      { part: "banner", x: 82, y: 56, scale: 1, paint: "violet-700" },
      { part: "banner", x: 14, y: 56, scale: 1, paint: "blue-800" },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 64, paint: "slate-700" }],
    light: [{ part: "halo", x: 46, y: 28, scale: 1.4, paint: "cyan-400" }],
    particles: [{ part: "sparks", x: 46, y: 28, scale: 1, count: 2, spread: [6, 4], paint: "cyan-400" }],
  },
};

/** Datacenter Fire (Classic+ T-AI-6, Token): flames roar out of rows of crystal server racks in a stone vault, smoke pouring from the overheated vents. */
const DATACENTER_FIRE: SceneRecipe = {
  family: "classic",
  sky: ["black", "red-900"],
  light: { x: 50, y: 52, r: 34, paint: "ember-500", opacity: 0.5 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 60, paint: "slate-800" },
      { part: "arch", x: 50, y: 60, scale: 1.6, paint: "slate-700" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 60, paint: "slate-800" },
      { part: "server-rack", x: 24, y: 76, scale: 1.1 },
      { part: "server-rack", x: 76, y: 76, scale: 1.1 },
    ],
    subject: [
      { part: "server-rack", x: 50, y: 84, scale: 1.4 },
      { part: "flames", x: 50, y: 50, scale: 1.6 },
      { part: "flames", x: 24, y: 48, scale: 1 },
      { part: "flames", x: 76, y: 48, scale: 1 },
    ],
    near: [{ part: "smoke", x: 50, y: 40, scale: 1.4 }],
    particles: [{ part: "embers", x: 50, y: 40, scale: 1.4, count: 3, spread: [24, 10] }],
  },
};

/** Alignment Tax (Classic+ T-AI-7, Token): a robed toll collector at a mountain bridge gate takes one extra silver coin from a grumbling traveller. */
const ALIGNMENT_TAX: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "slate-400"],
  light: { x: 50, y: 40, r: 30, paint: "slate-100", opacity: 0.3 },
  emblem: { x: 50, y: 34, size: 5, rotate: 0 },
  figure: {
    x: 38,
    y: 87,
    scale: 0.82,
    pose: "pose-guard",
    look: { ...figureLookFor([]), outfit: "outfit-robe", item: "item-coin", cloth: "slate-700" },
  },
  extras: [{ layer: "subject", x: 64, y: 87, scale: 0.8, flip: -1, pose: "pose-stand", look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-poncho", item: "item-sack", cloth: "red-600" } }],
  layers: {
    far: [
      { part: "mountains", x: 50, y: 56, paint: "slate-700" },
      { part: "cloud", x: 30, y: 26, scale: 1, count: 2, spread: [30, 4] },
    ],
    mid: [
      { part: "bridge", x: 50, y: 66, scale: 1.4 },
      { part: "gate", x: 50, y: 66, scale: 1.6 },
    ],
    subject: [{ part: "coins", x: 50, y: 84, scale: 1 }],
    near: [{ part: "grass-tuft", x: 50, y: 97, count: 3, spread: [44, 2] }],
    particles: [{ part: "sparks", x: 48, y: 60, scale: 0.8, paint: "white" }],
  },
};

/** Rate Limit (Classic+ T-AI-8, Token): a heavy iron portcullis slams down before a hurrying messenger at a temple gate, his third parcel still in hand. */
const RATE_LIMIT: SceneRecipe = {
  family: "classic",
  sky: ["slate-800", "slate-400"],
  light: { x: 40, y: 44, r: 30, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 40, y: 34, size: 5, rotate: 0 },
  figure: {
    x: 70,
    y: 88,
    scale: 0.72,
    flip: -1,
    pose: "pose-run",
    look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-jacket", item: "item-sack", cloth: "blue-800" },
  },
  layers: {
    far: [{ part: "ruin-wall", x: 40, y: 82, scale: 3, paint: "stone-500" }],
    mid: [
      { part: "street", x: 40, y: 72, paint: "slate-700" },
      { part: "portcullis", x: 40, y: 80, scale: 2.2 },
    ],
    subject: [
      { part: "gift-box", x: 46, y: 86, scale: 1, paint: "wood-600" },
      { part: "gift-box", x: 54, y: 87, scale: 0.9, paint: "wood-700" },
    ],
    near: [{ part: "dust", x: 40, y: 86, scale: 1.4, paint: "stone-500" }],
    particles: [{ part: "dust", x: 60, y: 84, scale: 1, paint: "slate-400" }],
  },
};

/** Refusal (Classic+ T-AI-9, Token): a calm automaton raises a palm before a guarded shrine, a fireball fizzling against its shimmering barrier. */
const REFUSAL: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-800"],
  light: { x: 50, y: 50, r: 32, paint: "sky-300", opacity: 0.45 },
  emblem: { x: 22, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 46,
    y: 87,
    scale: 0.95,
    pose: "pose-raise",
    look: { ...figureLookFor([]), head: "head-automaton", hair: null, outfit: "outfit-plate", item: null, skin: "slate-400", cloth: "blue-800" },
  },
  layers: {
    far: [
      { part: "temple", x: 22, y: 76, scale: 0.9 },
      { part: "hills", x: 50, y: 66, paint: "slate-800" },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 70, paint: "slate-700" }],
    subject: [
      { part: "shockwave", x: 62, y: 46, scale: 0.6, paint: "sky-300" },
      { part: "fireball", x: 76, y: 42, scale: 1, flip: -1 },
    ],
    light: [{ part: "rim-light", x: 60, y: 46, scale: 1.6, paint: "cyan-400", flip: -1 }],
    particles: [{ part: "smoke", x: 74, y: 40, scale: 0.8, paint: "slate-400" }],
  },
};

/** Fine-Tuning (Classic+ T-AI-10, Token): in a monastery tower's automaton workshop tiny tools adjust a half-built crystal figure on a bench among gears. */
const FINE_TUNING: SceneRecipe = {
  family: "classic",
  sky: ["wood-900", "wood-700"],
  light: { x: 50, y: 56, r: 30, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 20, y: 40, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 50, y: 70, scale: 0.42, pose: "pose-spread", look: { ...CRYSTAL_CONSTRUCT, head: "head-automaton", skin: "cyan-400", cloth: "sky-300" } }],
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 62, paint: "wood-700" },
      { part: "lantern", x: 70, y: 18, scale: 1.4 },
    ],
    mid: [{ part: "table", x: 50, y: 88, scale: 2.4 }],
    subject: [
      { part: "gear", x: 30, y: 62, scale: 0.6 },
      { part: "gear", x: 70, y: 64, scale: 0.5 },
      { part: "key", x: 64, y: 54, scale: 0.6 },
      { part: "quill", x: 36, y: 70, scale: 0.7 },
    ],
    near: [{ part: "gear", x: 12, y: 92, scale: 1 }],
    particles: [{ part: "sparks", x: 50, y: 56, scale: 0.8, count: 2, spread: [8, 6], paint: "cyan-400" }],
  },
};

export const CLASSIC_PLUS_RECIPES_3: Readonly<Record<string, SceneRecipe>> = {
  "classicplus-053": BOOK_OF_TOKENS,
  "classicplus-054": BOOK_OF_BOOKS,
  "classicplus-055": BOOK_OF_GREED,
  "classicplus-056": BOOK_OF_PAIN,
  "classicplus-057": BOOK_OF_STATS,
  "classicplus-058": FRUIT_BASKET,
  "classicplus-059": ALL_PURPOSE_APPLE,
  "classicplus-060": DOCTORS_ORDERS,
  "classicplus-061": BAUBLE_BUBBLE,
  "classicplus-062": KYS_PAPAYA,
  "classicplus-063": FRUIT_TREE,
  "classicplus-064": MULCH_MUNCHER,
  "classicplus-065": TWO_GRAPES,
  "classicplus-065-1": ROTTEN_GRAPE,
  "classicplus-065-2": NORMAL_GRAPE,
  "classicplus-065-3": LARGE_GRAPE,
  "classicplus-065-4": GOLDEN_GRAPE,
  "classicplus-065-5": MYTHIC_GRAPE,
  "classicplus-066": VINE_OF_GRAPES,
  "classicplus-067": PEAR,
  "classicplus-068": ORGANIC_PRODUCE,
  "classicplus-069": BUFF_BILLY,
  "classicplus-070": CHAOS_MACHINE,
  "classicplus-071": BOOK_OF_BUFF,
  "classicplus-072": BOOK_OF_NERF,
  "classicplus-073": CALL_TO_CHAOS,
  "classicplus-073-1": CLASSIC_GOLEM,
  "classicplus-074": TWICE_FORWARD,
  "classicplus-075": J_LEASE,
  "classicplus-075-1": J_LEASE_PACK,
  "classicplus-076": BROTHER_LAR,
  "classicplus-076-1": BROTHER_PING,
  "classicplus-077": ANTI_SOFTLOCK,
  "classicplus-078": CLAUDES_DATACENTER,
  "classicplus-t-ai-01": HELPFUL_ASSISTANT,
  "classicplus-t-ai-02": SCALING_LAW,
  "classicplus-t-ai-03": HALLUCINATION,
  "classicplus-t-ai-04": CHAIN_OF_THOUGHT,
  "classicplus-t-ai-05": AUTOCOMPLETE,
  "classicplus-t-ai-06": DATACENTER_FIRE,
  "classicplus-t-ai-07": ALIGNMENT_TAX,
  "classicplus-t-ai-08": RATE_LIMIT,
  "classicplus-t-ai-09": REFUSAL,
  "classicplus-t-ai-10": FINE_TUNING,
};
