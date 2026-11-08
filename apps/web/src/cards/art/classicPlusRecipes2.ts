// R3390000: Classic+'s scene recipes, part 2 of three (#27 to #52 with #32's, #36's, #38's, #42's and #46's
// tokens), data per card (ART.md, "Classic+ drawn as scenes"): each entry's brief from #3390's table drawn in
// Classic's temple-stone look from the scene kit's parts and the figure kit's looks, its subject within x 30 to
// 70 and y 25 to 85, its foreground near, its middle in the mid and subject layers and its background far, at or
// above its rarity's detail floor (richness.ts `ART_DETAIL_MIN`). Presentation only (CLAUDE.md rule 7): a
// recipe is read by the card's id, never by anything hidden.

import { figureLookFor } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** Zephrys Zealotism (Classic+ #27, Mythic): a zealot of living wind looms from a cloud lectern on a mountaintop temple, his cyclone trading a kneeling player's old cards for a glowing fan as mana crystals refill. */
const ZEPHRYS_ZEALOTISM: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "orange-400"],
  light: { x: 50, y: 40, r: 40, paint: "sky-300", opacity: 0.45 },
  emblem: { x: 50, y: 82, size: 6, rotate: 0 },
  figure: {
    x: 50,
    y: 80,
    scale: 1.05,
    pose: "pose-raise",
    look: { head: "head-human", hair: "hood", outfit: "outfit-robe", item: null, skin: "sky-300", cloth: "violet-700" },
  },
  extras: [
    {
      layer: "mid",
      x: 78,
      y: 84,
      scale: 0.55,
      flip: -1,
      pose: "pose-kneel",
      look: { head: "head-human", hair: "hair-short", outfit: "outfit-tunic", item: null, skin: "red-300", cloth: "blue-800" },
    },
  ],
  layers: {
    sky: [
      { part: "cloud", x: 22, y: 22, scale: 1.4, count: 4, spread: [34, 5], paint: "slate-700" },
      { part: "lightning", x: 84, y: 14, scale: 0.6 },
      { part: "sun", x: 14, y: 60, scale: 0.9 },
    ],
    far: [
      { part: "mountains", x: 50, y: 72, paint: "slate-800" },
      { part: "cloud", x: 50, y: 70, scale: 1.7, count: 6, spread: [46, 3], paint: "slate-100" },
      { part: "temple", x: 50, y: 70, scale: 1.05, paint: "stone-500" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 76, paint: "slate-700" },
      { part: "column", x: 14, y: 84, scale: 1.1 },
      { part: "column", x: 88, y: 82, scale: 0.9 },
      { part: "tornado", x: 70, y: 76, scale: 1.1, paint: "sky-300" },
      { part: "card", x: 70, y: 50, scale: 0.6, count: 4, spread: [9, 10], paint: "slate-700" },
    ],
    subject: [{ part: "card", x: 30, y: 62, scale: 0.85, count: 5, spread: [8, 3], paint: "cyan-400" }],
    near: [
      { part: "rune-circle", x: 50, y: 93, scale: 1.3, paint: "cyan-400" },
      { part: "cloud", x: 50, y: 92, scale: 1.4, count: 3, spread: [8, 2], paint: "white" },
      { part: "crystal", x: 50, y: 96, scale: 1.2, count: 6, spread: [30, 2], paint: "sky-400" },
      { part: "banner", x: 6, y: 100, scale: 1.5, paint: "slate-100" },
      { part: "banner", x: 94, y: 100, scale: 1.5, flip: -1, paint: "slate-100" },
    ],
    light: [
      { part: "god-rays", x: 50, y: -4, paint: "orange-400" },
      { part: "halo", x: 50, y: 36, scale: 2.6, paint: "sky-300" },
      { part: "halo", x: 30, y: 58, scale: 1.4, paint: "cyan-400" },
      { part: "rim-light", x: 50, y: 52, scale: 1.4, paint: "orange-400" },
    ],
    particles: [
      { part: "sparks", x: 30, y: 56, scale: 1.1, count: 3, spread: [10, 6], paint: "cyan-400" },
      { part: "motes", x: 60, y: 44, scale: 1.4, count: 2, spread: [20, 10], paint: "slate-100" },
      { part: "dust", x: 70, y: 70, scale: 1.1, paint: "slate-100" },
    ],
  },
};

/** Nuestro hogar, nuestras tumbas (Classic+ #28, Common): a skeletal guardian in a tattered poncho keeps the doorway of a family crypt-house, marigolds and candles at its feet, at dusk. */
const NUESTRO_HOGAR: SceneRecipe = {
  family: "classic",
  sky: ["violet-700", "orange-400"],
  light: { x: 50, y: 58, r: 34, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 50, y: 25, size: 6, rotate: 0 },
  figure: {
    x: 50,
    y: 87,
    scale: 0.85,
    pose: "pose-stand",
    look: { ...figureLookFor([]), head: "head-skull", hair: null, outfit: "outfit-poncho", item: "item-spear", skin: "slate-100", cloth: "red-600" },
  },
  layers: {
    sky: [
      { part: "stars", x: 26, y: 20, scale: 1.1 },
      { part: "moon", x: 86, y: 22, scale: 0.45 },
    ],
    mid: [
      { part: "house", x: 50, y: 86, scale: 3.2, paint: "stone-500" },
      { part: "field", x: 50, y: 85, paint: "wood-800" },
      { part: "tombstone", x: 12, y: 88, scale: 1.4 },
      { part: "tombstone", x: 88, y: 88, scale: 1.3 },
    ],
    subject: [{ part: "doorway", x: 50, y: 86, scale: 4.4 }],
    near: [
      { part: "flowers", x: 50, y: 95, scale: 1.4, count: 5, spread: [26, 2], paint: "orange-400" },
      { part: "candle", x: 32, y: 93, scale: 1.2, count: 3, spread: [5, 1] },
      { part: "candle", x: 68, y: 93, scale: 1.2, count: 3, spread: [5, 1] },
    ],
    particles: [
      { part: "petals", x: 50, y: 70, scale: 1.4, paint: "orange-400" },
      { part: "motes", x: 50, y: 50, scale: 1.2, paint: "orange-400" },
    ],
  },
};

/** Portal to the Past (Classic+ #29, Mythic): in a sunken temple a carved stone ring tears open on a sepia vortex where a past battle hangs mid-fall, a hand reaching in over the flooded steps. */
const PORTAL_TO_THE_PAST: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "slate-800"],
  light: { x: 50, y: 42, r: 36, paint: "brass-500", opacity: 0.5 },
  emblem: { x: 50, y: 15.5, size: 5, rotate: 0 },
  extras: [
    {
      layer: "mid",
      x: 40,
      y: 50,
      scale: 0.32,
      pose: "pose-spread",
      look: { head: "head-human", hair: "helm", outfit: "outfit-armor", item: "item-spear", skin: "brass-500", cloth: "wood-700" },
    },
    {
      layer: "mid",
      x: 62,
      y: 44,
      scale: 0.28,
      flip: -1,
      pose: "pose-strike",
      look: { head: "head-human", hair: "helm", outfit: "outfit-tabard", item: "item-sword", skin: "brass-500", cloth: "red-900" },
    },
  ],
  layers: {
    sky: [
      { part: "room-wall", x: 50, y: 64, paint: "slate-800" },
      { part: "arch", x: 50, y: 66, scale: 2.6, paint: "slate-700" },
    ],
    far: [
      { part: "column", x: 10, y: 84, scale: 2.4, paint: "slate-700" },
      { part: "column", x: 90, y: 84, scale: 2.4, paint: "slate-700" },
      { part: "vine", x: 18, y: 50, scale: 1.2, paint: "jade-400" },
      { part: "vine", x: 82, y: 46, scale: 1.1, flip: -1, paint: "jade-400" },
    ],
    mid: [
      { part: "stairs", x: 50, y: 64, scale: 1.2, paint: "stone-500" },
      { part: "void-swirl", x: 50, y: 42, scale: 1.75, paint: "brass-600" },
      { part: "ruin-wall", x: 52, y: 58, scale: 0.7, paint: "wood-700" },
      { part: "fallen-column", x: 46, y: 34, scale: 0.45, paint: "brass-500" },
      { part: "banner", x: 34, y: 40, scale: 0.55, paint: "red-600" },
      { part: "banner", x: 60, y: 34, scale: 0.45, flip: -1, paint: "red-900" },
    ],
    subject: [{ part: "portal-ring", x: 50, y: 42, scale: 1.45 }],
    near: [
      { part: "water", x: 50, y: 84, paint: "blue-800" },
      { part: "pale-hand", x: 52, y: 98, scale: 1.7 },
      { part: "fallen-column", x: 16, y: 92, scale: 1, flip: -1 },
      { part: "crystal", x: 84, y: 90, scale: 1.4, count: 3, spread: [8, 2], paint: "cyan-400" },
    ],
    light: [
      { part: "halo", x: 50, y: 42, scale: 3.2, paint: "brass-500" },
      { part: "light-shaft", x: 52, y: 44, scale: 0.8, paint: "brass-500" },
      { part: "rim-light", x: 50, y: 42, scale: 1.8, paint: "cyan-400" },
      { part: "halo", x: 52, y: 82, scale: 1.6, paint: "cyan-400" },
    ],
    particles: [
      { part: "dust", x: 50, y: 30, scale: 1, count: 2, spread: [18, 10], paint: "brass-500" },
      { part: "hourglass", x: 24, y: 44, scale: 0.5 },
      { part: "crystal", x: 50, y: 40, scale: 0.8, count: 5, spread: [30, 22], paint: "brass-500" },
      { part: "motes", x: 50, y: 50, scale: 1.6, count: 2, spread: [16, 10], paint: "cyan-400" },
      { part: "sparks", x: 76, y: 30, scale: 1, count: 2, spread: [6, 10], paint: "brass-500" },
    ],
  },
};

/** Felinor Fuser (Classic+ #30, Epic): a goggled cat-folk alchemist pours two kittens' glowing essences into one bubbling flask in a vaulted stone laboratory. */
const FELINOR_FUSER: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "slate-800"],
  light: { x: 62, y: 58, r: 30, paint: "violet-500", opacity: 0.45 },
  emblem: { x: 64, y: 76, size: 5, rotate: 0 },
  figure: {
    x: 42,
    y: 86,
    scale: 1.05,
    pose: "pose-stand",
    look: { ...figureLookFor(["Felinor"]), hair: "goggles", outfit: "outfit-coat", item: "item-flask", cloth: "wood-700" },
  },
  layers: {
    sky: [
      { part: "room-wall", x: 50, y: 66, paint: "slate-800" },
      { part: "arch", x: 50, y: 68, scale: 2.8, paint: "stone-500" },
    ],
    far: [
      { part: "bookshelf", x: 14, y: 70, scale: 1.3, paint: "wood-800" },
      { part: "bookshelf", x: 86, y: 70, scale: 1.3, paint: "wood-800" },
      { part: "flask", x: 14, y: 52, scale: 0.4, count: 3, spread: [5, 0], paint: "green-400" },
      { part: "flask", x: 86, y: 52, scale: 0.4, count: 3, spread: [5, 0], paint: "violet-500" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 70, paint: "stone-500" },
      { part: "table", x: 64, y: 82, scale: 1.4, paint: "wood-700" },
    ],
    subject: [
      { part: "flask", x: 64, y: 71, scale: 1.4, paint: "violet-500" },
      { part: "bubbles", x: 64, y: 50, scale: 1 },
    ],
    near: [
      { part: "cat", x: 22, y: 94, scale: 1.4 },
      { part: "cat", x: 82, y: 95, scale: 1.3, flip: -1, paint: "slate-400" },
      { part: "wisp", x: 28, y: 82, scale: 0.9, paint: "violet-300" },
      { part: "wisp", x: 76, y: 82, scale: 0.9, paint: "violet-300" },
      { part: "dust", x: 50, y: 96, scale: 1.2, paint: "orange-400" },
    ],
    light: [
      { part: "halo", x: 64, y: 62, scale: 1.8, paint: "violet-300" },
      { part: "light-ribbon", x: 52, y: 70, scale: 0.7, paint: "violet-300" },
    ],
    particles: [{ part: "sparks", x: 64, y: 52, scale: 1.2, count: 4, spread: [12, 8], paint: "violet-300" }],
  },
};

/** Fusion Lab (Classic+ #31, Epic): an arcane foundry under temple vaults, a welding arc joining two halves of a stone figurine on the workbench among cauldrons, copper pipes and chained machine arms. */
const FUSION_LAB: SceneRecipe = {
  family: "classic",
  sky: ["green-900", "slate-900"],
  light: { x: 50, y: 62, r: 30, paint: "sky-300", opacity: 0.45 },
  emblem: { x: 50, y: 30, size: 6, rotate: 0 },
  layers: {
    sky: [
      { part: "room-wall", x: 50, y: 62, paint: "slate-800" },
      { part: "arch", x: 24, y: 62, scale: 1.6, paint: "slate-700" },
      { part: "arch", x: 76, y: 62, scale: 1.6, paint: "slate-700" },
    ],
    far: [
      { part: "chain", x: 8, y: 22, scale: 1.2 },
      { part: "chain", x: 66, y: 20, scale: 1.1 },
      { part: "gear", x: 16, y: 40, scale: 1.1 },
      { part: "gear", x: 86, y: 36, scale: 0.9 },
      { part: "pipes", x: 50, y: 60, scale: 1.4 },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 64, paint: "slate-800" },
      { part: "cauldron", x: 22, y: 76, scale: 1.1 },
      { part: "cauldron", x: 80, y: 74, scale: 0.9 },
      { part: "bubbles", x: 22, y: 60, scale: 0.9, paint: "green-400" },
      { part: "smoke", x: 80, y: 62, scale: 1, paint: "green-200" },
    ],
    subject: [
      { part: "table", x: 50, y: 88, scale: 1.7, paint: "wood-700" },
      { part: "statue", x: 43, y: 75, scale: 0.95, paint: "stone-500" },
      { part: "statue", x: 57, y: 75, scale: 0.95, flip: -1, paint: "stone-500" },
      { part: "lightning", x: 50, y: 48, scale: 0.5 },
    ],
    near: [
      { part: "pipes", x: 6, y: 100, scale: 1.2, paint: "brass-600" },
      { part: "crate", x: 90, y: 98, scale: 1.5 },
      { part: "gear", x: 88, y: 86, scale: 0.55 },
      { part: "pebbles", x: 50, y: 98, scale: 1.4 },
    ],
    light: [
      { part: "halo", x: 50, y: 62, scale: 2, paint: "sky-300" },
      { part: "halo", x: 50, y: 62, scale: 0.9, paint: "white" },
      { part: "halo", x: 22, y: 66, scale: 1.4, paint: "green-400" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 62, scale: 1.2, count: 5, spread: [12, 8], paint: "slate-100" },
      { part: "embers", x: 50, y: 70, scale: 1.2, count: 2, spread: [10, 4], paint: "sky-300" },
    ],
  },
};

/** Otherworldly Removal (Classic+ #32, Epic): a rift over a mountain shrine's altar drops three foreign weapons, an axe before, knuckles between and a ring of blades behind, under an alien green sky. */
const OTHERWORLDLY_REMOVAL: SceneRecipe = {
  family: "classic",
  sky: ["green-900", "jade-400"],
  light: { x: 50, y: 28, r: 34, paint: "lime-200", opacity: 0.45 },
  emblem: { x: 50, y: 76, size: 6, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 30, y: 18, scale: 1.3 },
      { part: "nebula", x: 50, y: 26, scale: 1.1, paint: "green-400" },
    ],
    far: [
      { part: "mountains", x: 50, y: 70, paint: "slate-800" },
      { part: "standing-stone", x: 12, y: 74, scale: 1.1 },
      { part: "standing-stone", x: 88, y: 74, scale: 1, flip: -1 },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 74, paint: "slate-700" },
      { part: "column", x: 28, y: 84, scale: 1.2 },
      { part: "column", x: 72, y: 84, scale: 1.2 },
      { part: "void-swirl", x: 50, y: 24, scale: 1.3, paint: "green-400" },
      { part: "blade-ring", x: 50, y: 40, scale: 1 },
    ],
    subject: [
      { part: "altar", x: 50, y: 84, scale: 1.7 },
      { part: "candle", x: 37, y: 69, scale: 1.1, count: 2, spread: [2, 0] },
      { part: "candle", x: 63, y: 69, scale: 1.1, count: 2, spread: [2, 0] },
      { part: "knuckles", x: 50, y: 58, scale: 1.7 },
    ],
    near: [
      { part: "axe", x: 16, y: 100, scale: 1.8 },
      { part: "pebbles", x: 70, y: 96, scale: 1.4 },
      { part: "grass-tuft", x: 80, y: 98, count: 3, spread: [12, 2], paint: "green-700" },
    ],
    light: [
      { part: "light-shaft", x: 52, y: 24, scale: 0.9, paint: "lime-200" },
      { part: "halo", x: 50, y: 24, scale: 2.2, paint: "green-400" },
      { part: "halo", x: 50, y: 58, scale: 1.2, paint: "lime-200" },
    ],
    particles: [
      { part: "motes", x: 50, y: 46, scale: 1.4, paint: "green-200" },
      { part: "sparks", x: 50, y: 30, scale: 1.1, count: 3, spread: [12, 4], paint: "lime-200" },
    ],
  },
};

/** Execute (Classic+ #32.1, Token): an executioner's broad axe falls on a limping, bandaged stone knight in a grim castle yard, in flat ash-grey light. */
const EXECUTE: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "stone-500"],
  light: { x: 50, y: 40, r: 30, paint: "slate-100", opacity: 0.3 },
  emblem: { x: 16, y: 42, size: 5, rotate: 0 },
  figure: {
    x: 56,
    y: 86,
    scale: 1,
    flip: -1,
    pose: "pose-guard",
    look: { head: "head-golem", hair: "helm", outfit: "outfit-wraps", item: "item-sword", skin: "stone-500", cloth: "slate-100" },
  },
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 22, scale: 1.2, count: 3, spread: [30, 4], paint: "slate-400" },
      { part: "crow", x: 74, y: 28, scale: 0.7, count: 2, spread: [8, 4] },
    ],
    far: [
      { part: "tower", x: 16, y: 70, scale: 1.3 },
      { part: "portcullis", x: 80, y: 70, scale: 1.3 },
    ],
    mid: [{ part: "rock-floor", x: 50, y: 70, paint: "stone-500" }],
    subject: [{ part: "axe", x: 22, y: 70, scale: 1.7 }],
    near: [
      { part: "pebbles", x: 30, y: 96, scale: 1.3 },
      { part: "bones", x: 84, y: 96, scale: 1 },
    ],
    particles: [{ part: "dust", x: 56, y: 88, scale: 1.3, paint: "slate-400" }],
  },
};

/** Brawl (Classic+ #32.2, Token): stools and fists fly in a stone cellar inn as one bruised dwarf strolls out the door alone, in smoky orange lamplight. */
const BRAWL: SceneRecipe = {
  family: "classic",
  sky: ["wood-800", "brass-800"],
  light: { x: 50, y: 30, r: 34, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 50, y: 27, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 0.82,
    pose: "pose-stand",
    look: { head: "head-human", hair: "hair-long", outfit: "outfit-armor", item: "item-mallet", skin: "red-400", cloth: "wood-700" },
  },
  extras: [
    {
      layer: "mid",
      x: 22,
      y: 80,
      scale: 0.55,
      pose: "pose-strike",
      look: { head: "head-human", hair: "hair-short", outfit: "outfit-tunic", item: null, skin: "red-300", cloth: "blue-800" },
    },
    {
      layer: "mid",
      x: 80,
      y: 80,
      scale: 0.55,
      flip: -1,
      pose: "pose-flex",
      look: { head: "head-goblin", hair: null, outfit: "outfit-patched", item: null, skin: "green-700", cloth: "red-600" },
    },
  ],
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 72, paint: "stone-500" }],
    far: [
      { part: "doorway", x: 50, y: 72, scale: 3.2 },
      { part: "lantern", x: 22, y: 20, scale: 1.1 },
      { part: "lantern", x: 78, y: 22, scale: 1.1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 72, paint: "wood-700" },
      { part: "crate", x: 30, y: 42, scale: 0.8 },
      { part: "knuckles", x: 70, y: 44, scale: 1 },
    ],
    near: [
      { part: "table", x: 14, y: 96, scale: 1.2 },
      { part: "bread", x: 12, y: 87, scale: 0.9 },
      { part: "dice", x: 86, y: 96, scale: 1.2 },
    ],
    particles: [
      { part: "smoke", x: 30, y: 40, scale: 1.2, count: 2, spread: [30, 4], paint: "stone-500" },
      { part: "dust", x: 50, y: 84, scale: 1.2, paint: "brass-500" },
    ],
  },
};

/** Blade Storm (Classic+ #32.3, Token): a storm of spinning swords whirls over a sandy arena floor until a single fighter drops, in a dust-red haze. */
const BLADE_STORM: SceneRecipe = {
  family: "classic",
  sky: ["red-900", "orange-400"],
  light: { x: 50, y: 40, r: 34, paint: "red-400", opacity: 0.4 },
  emblem: { x: 14, y: 50, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 0.95,
    pose: "pose-kneel",
    look: { head: "head-human", hair: "hair-short", outfit: "outfit-tunic", item: "item-shield", skin: "red-300", cloth: "red-600" },
  },
  layers: {
    far: [
      { part: "column", x: 50, y: 64, scale: 0.75, count: 5, spread: [44, 0.5], paint: "stone-500" },
      { part: "banner", x: 12, y: 66, scale: 1.2 },
    ],
    mid: [
      { part: "field", x: 50, y: 64, paint: "brass-600" },
      { part: "blade-ring", x: 50, y: 32, scale: 1.2 },
      { part: "blade-ring", x: 24, y: 52, scale: 0.75 },
      { part: "blade-ring", x: 78, y: 48, scale: 0.85 },
    ],
    near: [
      { part: "dust", x: 50, y: 92, scale: 1.6, paint: "red-300" },
      { part: "pebbles", x: 80, y: 96, scale: 1.2 },
    ],
    particles: [
      { part: "dust", x: 50, y: 50, scale: 1.4, paint: "red-400" },
      { part: "sparks", x: 50, y: 40, scale: 1.2, count: 2, spread: [20, 8] },
    ],
  },
};

/** Ivory Tower (Classic+ #33, Rare): a slender white tower soars above the clouds, a scholar's face set in its high window and winged figures circling, under a pale serene sky. */
const IVORY_TOWER: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 50, y: 34, r: 34, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  extras: [
    {
      layer: "subject",
      x: 50,
      y: 46,
      scale: 0.16,
      pose: "pose-stand",
      look: { head: "head-human", hair: "hat-pointed", outfit: "outfit-coat", item: null, skin: "red-300", cloth: "blue-800" },
    },
  ],
  layers: {
    sky: [
      { part: "sun", x: 18, y: 24, scale: 0.8 },
      { part: "cloud", x: 72, y: 22, scale: 1, count: 2, spread: [14, 3], paint: "white" },
    ],
    far: [{ part: "cloud", x: 50, y: 78, scale: 1.8, count: 5, spread: [46, 3], paint: "slate-100" }],
    mid: [{ part: "bird", x: 50, y: 40, scale: 0.6, count: 4, spread: [30, 14] }],
    subject: [{ part: "tower", x: 50, y: 88, scale: 2, paint: "white" }],
    near: [{ part: "cloud", x: 50, y: 96, scale: 2.2, count: 4, spread: [44, 2], paint: "white" }],
    light: [
      { part: "rim-light", x: 50, y: 56, scale: 2, paint: "white" },
      { part: "halo", x: 50, y: 44, scale: 1.4, paint: "orange-400" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "white" }],
  },
};

/** Memory Leak (Classic+ #34, Epic): a forgotten temple archive whose floor frosts into blank ice, its plinths padlocked in chains and its shelves dissolving into pale fog. */
const MEMORY_LEAK: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-800"],
  light: { x: 50, y: 50, r: 34, paint: "blue-200", opacity: 0.35 },
  emblem: { x: 50, y: 74, size: 6, rotate: 0 },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 66, paint: "slate-700" }],
    far: [
      { part: "bookshelf", x: 14, y: 66, scale: 1.4 },
      { part: "bookshelf", x: 38, y: 64, scale: 1.1 },
      { part: "bookshelf", x: 62, y: 64, scale: 1.1 },
      { part: "bookshelf", x: 86, y: 66, scale: 1.4 },
      { part: "mist", x: 50, y: 48, scale: 1.4, count: 3, spread: [30, 8], paint: "slate-100" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "stone-500" },
      { part: "altar", x: 22, y: 78, scale: 1, paint: "slate-400" },
      { part: "altar", x: 78, y: 78, scale: 1, paint: "slate-400" },
      { part: "padlock", x: 22, y: 74, scale: 0.9 },
      { part: "padlock", x: 78, y: 74, scale: 0.9 },
    ],
    subject: [
      { part: "altar", x: 50, y: 82, scale: 1.7, paint: "slate-400" },
      { part: "tome", x: 50, y: 67, scale: 1 },
      { part: "chain", x: 36, y: 70, scale: 1.3 },
      { part: "padlock", x: 50, y: 66, scale: 1.6 },
    ],
    near: [
      { part: "ice-floor", x: 50, y: 88 },
      { part: "crystal", x: 14, y: 96, scale: 1.3, count: 3, spread: [6, 2], paint: "blue-200" },
      { part: "crystal", x: 86, y: 96, scale: 1.3, count: 3, spread: [6, 2], paint: "blue-200" },
    ],
    light: [
      { part: "halo", x: 50, y: 66, scale: 2, paint: "blue-200" },
      { part: "light-shaft", x: 50, y: 0, scale: 1.2, paint: "blue-200" },
    ],
    particles: [
      { part: "snow", x: 50, y: 50, scale: 1.2 },
      { part: "motes", x: 50, y: 70, scale: 1.4, paint: "sky-300" },
    ],
  },
};

/** Rollback (Classic+ #35, Legendary): before a great temple a robed chronomancer drags a huge clock hand backwards as the battle runs in reverse, echoes of it layered behind in violet dusk. */
const ROLLBACK: SceneRecipe = {
  family: "classic",
  sky: ["violet-700", "orange-400"],
  light: { x: 62, y: 34, r: 40, paint: "violet-300", opacity: 0.45 },
  emblem: { x: 64, y: 32, size: 6, rotate: 0 },
  figure: {
    x: 44,
    y: 86,
    scale: 1,
    flip: -1,
    pose: "pose-run",
    look: { head: "head-human", hair: "hair-long", outfit: "outfit-robe", item: "item-spear", skin: "red-300", cloth: "violet-700" },
  },
  extras: [
    {
      layer: "mid",
      x: 18,
      y: 82,
      scale: 0.45,
      pose: "pose-kneel",
      look: { head: "head-human", hair: "helm", outfit: "outfit-armor", item: "item-shield", skin: "red-300", cloth: "slate-400" },
    },
    {
      layer: "mid",
      x: 84,
      y: 80,
      scale: 0.42,
      flip: -1,
      pose: "pose-guard",
      look: { head: "head-human", hair: "helm", outfit: "outfit-armor", item: "item-bow", skin: "red-300", cloth: "slate-400" },
    },
  ],
  layers: {
    sky: [{ part: "clock-face", x: 64, y: 32, scale: 2.3 }],
    far: [
      { part: "temple", x: 22, y: 58, scale: 0.8, paint: "violet-500" },
      { part: "temple", x: 80, y: 56, scale: 0.75, paint: "violet-500" },
      { part: "temple", x: 50, y: 64, scale: 1.2, paint: "stone-500" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 66, paint: "wood-800" },
      { part: "fallen-column", x: 26, y: 72, scale: 0.8 },
      { part: "column", x: 74, y: 76, scale: 0.9 },
      { part: "arrow", x: 70, y: 56, scale: 0.6, flip: -1, count: 3, spread: [10, 6] },
    ],
    subject: [{ part: "light-ribbon", x: 46, y: 60, scale: 0.9, paint: "violet-300" }],
    near: [
      { part: "fallen-column", x: 82, y: 96, scale: 1.2, flip: -1 },
      { part: "hourglass", x: 14, y: 98, scale: 1.3 },
      { part: "pebbles", x: 46, y: 98, scale: 1.5 },
    ],
    light: [
      { part: "god-rays", x: 64, y: 4, paint: "violet-300" },
      { part: "halo", x: 64, y: 32, scale: 2.6, paint: "violet-300" },
      { part: "rim-light", x: 44, y: 60, scale: 1.4, paint: "orange-400" },
    ],
    particles: [
      { part: "dust", x: 50, y: 80, scale: 1.4, count: 2, spread: [24, 4], paint: "violet-300" },
      { part: "motes", x: 50, y: 50, scale: 1.6, paint: "violet-300" },
      { part: "sparks", x: 64, y: 32, scale: 1.4, count: 3, spread: [14, 10], paint: "white" },
    ],
  },
};

/** Conjure Bones (Classic+ #36, Rare): a sly necromancer slips a rattling handful of tiny bones into a deck of cards on an ossuary table, in bone-white candlelight. */
const CONJURE_BONES: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 62, r: 30, paint: "slate-100", opacity: 0.35 },
  emblem: { x: 50, y: 25, size: 6, rotate: 0 },
  figure: {
    x: 46,
    y: 94,
    scale: 1.05,
    pose: "pose-stand",
    look: { head: "head-human", hair: "hood", outfit: "outfit-robe", item: null, skin: "slate-400", cloth: "violet-700" },
  },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 76, paint: "slate-800" }],
    far: [
      { part: "bones", x: 50, y: 40, scale: 0.9, count: 4, spread: [40, 0] },
      { part: "bones", x: 50, y: 58, scale: 0.9, count: 4, spread: [40, 0] },
    ],
    near: [
      { part: "table", x: 50, y: 100, scale: 2.2, paint: "wood-800" },
      { part: "card", x: 60, y: 82, scale: 0.9, count: 3, spread: [2, 0.5], paint: "violet-700" },
      { part: "bones", x: 68, y: 82, scale: 0.6 },
      { part: "candle", x: 28, y: 82, scale: 1.4, count: 2, spread: [3, 0] },
      { part: "candle", x: 78, y: 82, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 28, y: 72, scale: 1.4, paint: "slate-100" },
      { part: "halo", x: 78, y: 72, scale: 1.2, paint: "slate-100" },
    ],
    particles: [
      { part: "motes", x: 50, y: 60, scale: 1.4, paint: "slate-100" },
      { part: "sparks", x: 58, y: 74, scale: 0.8, count: 2, spread: [3, 2], paint: "slate-100" },
    ],
  },
};

/** Bone Storm (Classic+ #36.1, Token): a storm of whirling bones erupts from a freshly drawn card over a catacomb and batters a line of raiders, bone-white and bruised violet. */
const BONE_STORM: SceneRecipe = {
  family: "classic",
  sky: ["violet-700", "slate-800"],
  light: { x: 40, y: 50, r: 30, paint: "violet-300", opacity: 0.4 },
  emblem: { x: 40, y: 68, size: 5, rotate: 0 },
  extras: [
    {
      layer: "mid",
      x: 74,
      y: 84,
      scale: 0.55,
      flip: -1,
      pose: "pose-guard",
      look: { head: "head-human", hair: "helm", outfit: "outfit-fur", item: "item-shield", skin: "red-300", cloth: "wood-700" },
    },
    {
      layer: "mid",
      x: 86,
      y: 82,
      scale: 0.5,
      flip: -1,
      pose: "pose-guard",
      look: { head: "head-human", hair: "hood", outfit: "outfit-fur", item: "item-sword", skin: "red-300", cloth: "wood-800" },
    },
  ],
  layers: {
    sky: [{ part: "cave", x: 50, y: 70, paint: "slate-800" }],
    far: [{ part: "tombstone", x: 50, y: 72, scale: 1, count: 4, spread: [36, 1] }],
    mid: [{ part: "rock-floor", x: 50, y: 74, paint: "slate-700" }],
    subject: [
      { part: "card", x: 40, y: 78, scale: 2, paint: "violet-700" },
      { part: "tornado", x: 42, y: 60, scale: 1.1, paint: "slate-100" },
    ],
    near: [{ part: "bones", x: 18, y: 96, scale: 1.4 }],
    particles: [
      { part: "bones", x: 54, y: 40, scale: 0.5, count: 4, spread: [18, 12] },
      { part: "dust", x: 60, y: 60, scale: 1.2, paint: "slate-100" },
    ],
  },
};

/** Wardrum (Classic+ #37, Legendary): a drummer-golem whose stone chest is a war drum bursts from a torchlit amphitheatre's crowd, pounding with thigh-bone mallets inside a shimmering shield of spell-light rings. */
const WARDRUM: SceneRecipe = {
  family: "classic",
  sky: ["red-900", "orange-400"],
  light: { x: 50, y: 50, r: 40, paint: "ember-500", opacity: 0.45 },
  emblem: { x: 50, y: 52, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1.1,
    pose: "pose-strike",
    look: { ...figureLookFor(["Quickdraw", "Acclaimed"]), head: "head-golem", hair: null, outfit: "outfit-drum", item: "item-mallet", skin: "stone-500", cloth: "red-600" },
  },
  extras: [
    {
      layer: "far",
      x: 18,
      y: 66,
      scale: 0.38,
      pose: "pose-raise",
      look: { head: "head-human", hair: "hair-short", outfit: "outfit-tunic", item: null, skin: "red-300", cloth: "blue-800" },
    },
    {
      layer: "far",
      x: 82,
      y: 66,
      scale: 0.38,
      flip: -1,
      pose: "pose-spread",
      look: { head: "head-human", hair: "hood", outfit: "outfit-tunic", item: null, skin: "red-300", cloth: "green-700" },
    },
  ],
  layers: {
    sky: [{ part: "smoke", x: 50, y: 30, scale: 1.4, count: 2, spread: [30, 4], paint: "wood-800" }],
    far: [
      { part: "hills", x: 50, y: 52, paint: "slate-800" },
      { part: "column", x: 50, y: 56, scale: 0.6, count: 6, spread: [46, 1], paint: "stone-500" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 72, paint: "wood-800" },
      { part: "fence-post", x: 12, y: 80, scale: 1.2 },
      { part: "fence-post", x: 86, y: 80, scale: 1.2, flip: -1 },
      { part: "flames", x: 13, y: 62, scale: 1 },
      { part: "flames", x: 87, y: 62, scale: 1 },
      { part: "shockwave", x: 50, y: 78, scale: 1.6, paint: "orange-400" },
    ],
    subject: [
      { part: "orb", x: 22, y: 40, scale: 0.5 },
      { part: "fireball", x: 80, y: 38, scale: 0.7, flip: -1 },
      { part: "crystal", x: 74, y: 60, scale: 1.4, paint: "sky-300" },
    ],
    near: [
      { part: "drum", x: 14, y: 98, scale: 1.2 },
      { part: "drum", x: 86, y: 98, scale: 1.1 },
      { part: "flames", x: 30, y: 98, scale: 0.9 },
    ],
    light: [
      { part: "dome", x: 50, y: 88, scale: 1.9, paint: "violet-300" },
      { part: "light-ribbon", x: 50, y: 46, scale: 1.3, paint: "orange-400" },
      { part: "halo", x: 50, y: 52, scale: 2.4, paint: "ember-500" },
      { part: "rim-light", x: 50, y: 56, scale: 1.6, paint: "orange-400" },
    ],
    particles: [
      { part: "embers", x: 50, y: 60, scale: 1.6, count: 4, spread: [30, 14] },
      { part: "sparks", x: 50, y: 30, scale: 1.2, count: 3, spread: [20, 6], paint: "orange-400" },
    ],
  },
};

/** Solarius (Classic+ #38, Epic): a small smiling sun-spirit in scholar's robes schemes over a star chart on a mountain observatory terrace as a larger sun rises behind the peaks. */
const SOLARIUS: SceneRecipe = {
  family: "classic",
  sky: ["blue-800", "orange-400"],
  light: { x: 68, y: 56, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 72, y: 76, size: 5, rotate: 0 },
  figure: {
    x: 48,
    y: 86,
    scale: 0.9,
    pose: "pose-stand",
    look: { ...figureLookFor(["Catalyst"]), head: "head-sun", hair: null, outfit: "outfit-robe", item: "item-scroll", skin: "orange-400", cloth: "blue-800" },
  },
  layers: {
    sky: [
      { part: "stars", x: 30, y: 18, scale: 1.1 },
      { part: "sun", x: 72, y: 56, scale: 2.4 },
    ],
    far: [{ part: "mountains", x: 50, y: 70, paint: "slate-700" }],
    mid: [
      { part: "tile-floor", x: 50, y: 74, paint: "stone-500" },
      { part: "column", x: 12, y: 86, scale: 1.4 },
      { part: "column", x: 88, y: 86, scale: 1.4 },
      { part: "table", x: 72, y: 84, scale: 1 },
      { part: "scroll", x: 72, y: 76, scale: 0.9, paint: "blue-200" },
    ],
    near: [
      { part: "astrolabe", x: 22, y: 98, scale: 1.8 },
      { part: "fence", x: 50, y: 100, scale: 2, paint: "stone-500" },
    ],
    light: [
      { part: "god-rays", x: 72, y: 40, paint: "white" },
      { part: "halo", x: 48, y: 42, scale: 1.4, paint: "orange-400" },
    ],
    particles: [
      { part: "sparks", x: 48, y: 40, scale: 1.2, count: 3, spread: [10, 6], paint: "orange-400" },
      { part: "motes", x: 50, y: 60, scale: 1.4, paint: "white" },
    ],
  },
};

/** Solarius Prime (Classic+ #38.1, Token): a blazing white sun-titan rises over a temple summit flinging spell sparks from both hands, in searing white-orange glare. */
const SOLARIUS_PRIME: SceneRecipe = {
  family: "classic",
  sky: ["red-900", "orange-400"],
  light: { x: 50, y: 40, r: 44, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 68, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 1.1,
    pose: "pose-spread",
    look: { ...figureLookFor(["Prime", "Token"]), head: "head-sun", hair: null, outfit: "outfit-muscle", item: null, skin: "white", cloth: "orange-400" },
  },
  layers: {
    far: [{ part: "mountains", x: 50, y: 78, paint: "slate-800" }],
    near: [{ part: "temple", x: 50, y: 100, scale: 1.3 }],
    light: [
      { part: "god-rays", x: 50, y: 30, paint: "white" },
      { part: "halo", x: 50, y: 34, scale: 2.6, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 28, y: 36, scale: 1.3, count: 3, spread: [6, 6], paint: "orange-400" },
      { part: "sparks", x: 72, y: 36, scale: 1.3, count: 3, spread: [6, 6], paint: "white" },
      { part: "embers", x: 50, y: 60, scale: 1.6, count: 2, spread: [24, 10] },
    ],
  },
};

/** Book Worm (Classic+ #39, Common): a fat spectacled worm with plague-green spots burrows through a stack of old leather tomes in a monastery library, in dusty lamplight. */
const BOOK_WORM: SceneRecipe = {
  family: "classic",
  sky: ["wood-800", "brass-800"],
  light: { x: 50, y: 46, r: 32, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 50, y: 78, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "bookshelf", x: 16, y: 76, scale: 1.5, paint: "wood-700" },
      { part: "bookshelf", x: 84, y: 76, scale: 1.5, paint: "wood-700" },
      { part: "arch", x: 50, y: 74, scale: 1.6, paint: "stone-500" },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 76, paint: "wood-700" }],
    subject: [
      { part: "tome", x: 50, y: 86, scale: 1.9, paint: "red-900" },
      { part: "tome", x: 50, y: 79, scale: 1.6, paint: "wood-700" },
      { part: "tome", x: 50, y: 73, scale: 1.4, paint: "green-900" },
      { part: "grub", x: 50, y: 68, scale: 1.7 },
    ],
    near: [{ part: "candle", x: 84, y: 96, scale: 1.4 }],
    particles: [
      { part: "motes", x: 50, y: 46, scale: 1.4, paint: "brass-500" },
      { part: "spores", x: 50, y: 60, scale: 0.9, paint: "green-400" },
    ],
  },
};

/** Appropriations (Classic+ #40, Epic): in a domed senate hall robed councillors squabble as a chest of silver coins splits four ways, to a spear rack, a stack of books, a lyre and a healer's satchel. */
const APPROPRIATIONS: SceneRecipe = {
  family: "classic",
  sky: ["slate-800", "stone-500"],
  light: { x: 24, y: 30, r: 34, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 50, y: 72, size: 6, rotate: 0 },
  extras: [
    {
      layer: "far",
      x: 20,
      y: 72,
      scale: 0.48,
      pose: "pose-spread",
      look: { head: "head-human", hair: "hair-short", outfit: "outfit-robe", item: null, skin: "red-300", cloth: "slate-100" },
    },
    {
      layer: "far",
      x: 34,
      y: 70,
      scale: 0.42,
      pose: "pose-raise",
      look: { head: "head-human", hair: "tonsure", outfit: "outfit-robe", item: "item-scroll", skin: "red-300", cloth: "red-600" },
    },
    {
      layer: "far",
      x: 80,
      y: 72,
      scale: 0.48,
      flip: -1,
      pose: "pose-raise",
      look: { head: "head-human", hair: "hair-long", outfit: "outfit-robe", item: null, skin: "red-300", cloth: "blue-800" },
    },
  ],
  layers: {
    sky: [
      { part: "room-wall", x: 50, y: 66, paint: "stone-500" },
      { part: "arch", x: 50, y: 66, scale: 2.4, paint: "slate-700" },
    ],
    far: [
      { part: "column", x: 8, y: 74, scale: 1.6 },
      { part: "column", x: 92, y: 74, scale: 1.6 },
      { part: "tile-floor", x: 50, y: 66, paint: "stone-500" },
    ],
    mid: [
      { part: "coins", x: 32, y: 72, scale: 0.6, count: 2, spread: [6, 3] },
      { part: "coins", x: 68, y: 72, scale: 0.6, count: 2, spread: [6, 3] },
      { part: "motes", x: 50, y: 74, scale: 1.4, paint: "slate-100" },
    ],
    subject: [
      { part: "chest", x: 50, y: 80, scale: 1.6 },
      { part: "coins", x: 50, y: 62, scale: 0.9 },
    ],
    near: [
      { part: "weapon-rack", x: 12, y: 100, scale: 1.1 },
      { part: "tome", x: 34, y: 98, scale: 1, count: 2, spread: [1, 3] },
      { part: "lyre", x: 66, y: 98, scale: 1.2 },
      { part: "flask", x: 88, y: 98, scale: 1.2, paint: "red-400" },
    ],
    light: [
      { part: "light-shaft", x: 26, y: 0, scale: 1.2, paint: "orange-400" },
      { part: "halo", x: 50, y: 64, scale: 1.8, paint: "slate-100" },
    ],
    particles: [{ part: "sparks", x: 50, y: 66, scale: 1.2, count: 4, spread: [26, 10], paint: "slate-100" }],
  },
};

/** KY's Constant (Classic+ #41, Rare): KY, a stern teacher in a chalk-dusted cardigan, taps a slate where every shape has become a triangle, in a dusty green stone schoolroom. */
const KYS_CONSTANT: SceneRecipe = {
  family: "classic",
  sky: ["green-900", "slate-800"],
  light: { x: 40, y: 40, r: 30, paint: "green-200", opacity: 0.3 },
  emblem: { x: 66, y: 30, size: 6, rotate: 0 },
  figure: {
    x: 40,
    y: 86,
    scale: 1.05,
    pose: "pose-guard",
    look: { ...figureLookFor(["KY"]), hair: "hair-short", outfit: "outfit-cardigan", item: "item-staff", cloth: "green-700" },
  },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 72, paint: "stone-500" }],
    mid: [
      { part: "tile-floor", x: 50, y: 72, paint: "wood-700" },
      { part: "table", x: 14, y: 84, scale: 1 },
      { part: "table", x: 86, y: 86, scale: 1.1 },
    ],
    subject: [{ part: "slate-board", x: 66, y: 84, scale: 2.2 }],
    near: [
      { part: "tome", x: 16, y: 98, scale: 1.1 },
      { part: "apple", x: 86, y: 97, scale: 1.4 },
    ],
    light: [{ part: "light-shaft", x: 30, y: 0, scale: 1.2, paint: "green-200" }],
    particles: [
      { part: "dust", x: 60, y: 70, scale: 1.2, paint: "slate-100" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "green-200" },
    ],
  },
};

/** KY's Test (Classic+ #42, Legendary): in a vaulted examination hall KY strides between rows of sweating students and slaps down three sealed scrolls of rising size, each tied to a closed mystery box. */
const KYS_TEST: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "green-900"],
  light: { x: 62, y: 30, r: 36, paint: "green-200", opacity: 0.4 },
  emblem: { x: 82, y: 75, size: 5, rotate: 0 },
  figure: {
    x: 48,
    y: 84,
    scale: 1,
    pose: "pose-run",
    look: { ...figureLookFor(["KY"]), hair: "hair-short", outfit: "outfit-cardigan", item: "item-scroll", cloth: "green-700" },
  },
  extras: [
    {
      layer: "far",
      x: 16,
      y: 72,
      scale: 0.42,
      pose: "pose-sit",
      look: { head: "head-human", hair: "hair-short", outfit: "outfit-tunic", item: null, skin: "red-300", cloth: "blue-800" },
    },
    {
      layer: "far",
      x: 84,
      y: 72,
      scale: 0.42,
      flip: -1,
      pose: "pose-sit",
      look: { head: "head-human", hair: "hair-long", outfit: "outfit-tunic", item: null, skin: "red-300", cloth: "red-600" },
    },
    {
      layer: "mid",
      x: 24,
      y: 80,
      scale: 0.5,
      pose: "pose-sit",
      look: { head: "head-human", hair: "kerchief", outfit: "outfit-jacket", item: null, skin: "red-300", cloth: "slate-700" },
    },
    {
      layer: "mid",
      x: 76,
      y: 80,
      scale: 0.5,
      flip: -1,
      pose: "pose-sit",
      look: { head: "head-human", hair: "hood", outfit: "outfit-tunic", item: null, skin: "red-300", cloth: "wood-700" },
    },
  ],
  layers: {
    sky: [
      { part: "room-wall", x: 50, y: 64, paint: "slate-700" },
      { part: "arch", x: 18, y: 64, scale: 1.6, paint: "slate-800" },
      { part: "arch", x: 50, y: 62, scale: 1.8, paint: "slate-800" },
      { part: "arch", x: 82, y: 64, scale: 1.6, paint: "slate-800" },
    ],
    far: [
      { part: "tile-floor", x: 50, y: 64, paint: "stone-500" },
      { part: "table", x: 16, y: 72, scale: 0.6 },
      { part: "table", x: 84, y: 72, scale: 0.6 },
    ],
    mid: [
      { part: "table", x: 24, y: 82, scale: 0.75 },
      { part: "table", x: 76, y: 82, scale: 0.75 },
    ],
    near: [
      { part: "table", x: 50, y: 100, scale: 2.6, paint: "wood-700" },
      { part: "scroll", x: 22, y: 84, scale: 0.5 },
      { part: "gift-box", x: 30, y: 84, scale: 0.6, paint: "slate-700" },
      { part: "scroll", x: 42, y: 84, scale: 0.85 },
      { part: "gift-box", x: 56, y: 84, scale: 0.9, paint: "slate-700" },
      { part: "scroll", x: 70, y: 84, scale: 1.2 },
      { part: "gift-box", x: 82, y: 84, scale: 1.4, paint: "slate-700" },
      { part: "quill", x: 10, y: 84, scale: 1 },
    ],
    light: [
      { part: "light-shaft", x: 66, y: 0, scale: 1.4, paint: "green-200" },
      { part: "halo", x: 48, y: 40, scale: 1.6, paint: "green-200" },
    ],
    particles: [
      { part: "dust", x: 60, y: 50, scale: 1.4, count: 2, spread: [16, 10], paint: "slate-100" },
      { part: "motes", x: 50, y: 50, scale: 1.6, paint: "green-200" },
      { part: "sparks", x: 12, y: 76, scale: 0.8, count: 2, spread: [3, 3] },
    ],
  },
};

/** KY's Gift (Classic+ #42.1, Token): a teacher's stone study after the exam, its desk heaped with ribboned gift boxes, apples and books beside a slate of ticks, in warm late light. */
const KYS_GIFT: SceneRecipe = {
  family: "classic",
  sky: ["wood-800", "orange-400"],
  light: { x: 24, y: 36, r: 34, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 44, y: 62, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 70, paint: "stone-500" }],
    far: [
      { part: "bookshelf", x: 86, y: 72, scale: 1.3 },
      { part: "slate-board", x: 18, y: 76, scale: 1.2 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 72, paint: "wood-700" }],
    subject: [
      { part: "table", x: 50, y: 88, scale: 2, paint: "wood-700" },
      { part: "gift-box", x: 44, y: 75, scale: 1.2, count: 3, spread: [8, 0] },
      { part: "gift-box", x: 46, y: 64, scale: 0.9, paint: "blue-800" },
      { part: "apple", x: 64, y: 75, scale: 1.2, count: 3, spread: [4, 0] },
      { part: "tome", x: 30, y: 75, scale: 0.7 },
    ],
    near: [
      { part: "gift-box", x: 84, y: 98, scale: 1.2, paint: "green-700" },
      { part: "apple", x: 18, y: 97, scale: 1.4 },
    ],
    light: [{ part: "light-shaft", x: 22, y: 0, scale: 1.3, paint: "orange-400" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "orange-400" }],
  },
};

/** AI Slop (Classic+ #43, Legendary): in a crystal forge beneath a temple a loom of circuit-light weaves two half-formed creatures into one melting, uncanny beast as glitches flake off like ash. */
const AI_SLOP: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 52, r: 40, paint: "purple-400", opacity: 0.45 },
  emblem: { x: 50, y: 26, size: 6, rotate: 0 },
  layers: {
    sky: [{ part: "cave", x: 50, y: 72, paint: "slate-800" }],
    far: [
      { part: "server-rack", x: 18, y: 76, scale: 1.4 },
      { part: "server-rack", x: 82, y: 76, scale: 1.4 },
      { part: "crystal", x: 30, y: 74, scale: 1.4, count: 3, spread: [4, 1], paint: "cyan-400" },
      { part: "crystal", x: 70, y: 74, scale: 1.4, count: 3, spread: [4, 1], paint: "purple-400" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 76, paint: "slate-900" },
      { part: "light-ribbon", x: 50, y: 46, scale: 1.6, paint: "cyan-400" },
      { part: "light-ribbon", x: 50, y: 52, scale: 1.3, paint: "purple-400" },
    ],
    subject: [
      { part: "snake", x: 66, y: 66, scale: 1, flip: -1, paint: "purple-400" },
      { part: "snake", x: 70, y: 74, scale: 0.9, flip: -1, paint: "cyan-400" },
      { part: "boar", x: 48, y: 78, scale: 1.8, paint: "violet-500" },
      { part: "pale-hand", x: 34, y: 62, scale: 1.2, paint: "purple-400" },
      { part: "pale-hand", x: 30, y: 78, scale: 1, flip: -1 },
      { part: "glitch", x: 44, y: 64, scale: 1 },
    ],
    near: [
      { part: "crystal", x: 50, y: 97, scale: 1.4, count: 6, spread: [42, 2], paint: "cyan-400" },
      { part: "bone-hand", x: 86, y: 98, scale: 1.3, flip: -1 },
    ],
    light: [
      { part: "halo", x: 50, y: 50, scale: 2.6, paint: "purple-400" },
      { part: "halo", x: 50, y: 46, scale: 1.4, paint: "cyan-400" },
      { part: "rim-light", x: 48, y: 64, scale: 1.4, paint: "cyan-400" },
    ],
    particles: [
      { part: "glitch", x: 50, y: 52, scale: 1, count: 3, spread: [24, 16] },
      { part: "embers", x: 50, y: 50, scale: 1.6, count: 3, spread: [24, 14], paint: "purple-400" },
      { part: "sparks", x: 50, y: 44, scale: 1.2, count: 2, spread: [16, 8], paint: "cyan-400" },
    ],
  },
};

/** Simplicity Audit (Classic+ #44, Rare): a grey-robed auditor weighs scrolls on a counting-house's stone balance, the short ones glitching into static while the long ones stay, in cold white light. */
const SIMPLICITY_AUDIT: SceneRecipe = {
  family: "classic",
  sky: ["slate-800", "slate-400"],
  light: { x: 64, y: 44, r: 30, paint: "white", opacity: 0.4 },
  emblem: { x: 66, y: 77, size: 5, rotate: 0 },
  figure: {
    x: 40,
    y: 86,
    scale: 1.05,
    pose: "pose-guard",
    look: { head: "head-human", hair: "tonsure", outfit: "outfit-robe", item: null, skin: "red-300", cloth: "grey-500" },
  },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 72, paint: "stone-500" }],
    far: [
      { part: "cabinet", x: 14, y: 74, scale: 1.4 },
      { part: "cabinet", x: 88, y: 74, scale: 1.2 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 72, paint: "slate-700" }],
    subject: [
      { part: "scales", x: 66, y: 84, scale: 1.7 },
      { part: "scroll", x: 50, y: 60, scale: 0.4 },
      { part: "glitch", x: 50, y: 58, scale: 0.6 },
      { part: "scroll", x: 82, y: 62, scale: 0.9 },
    ],
    near: [
      { part: "coins", x: 18, y: 96, scale: 1.2 },
      { part: "scroll", x: 84, y: 97, scale: 1.2, count: 2, spread: [4, 2] },
    ],
    light: [{ part: "halo", x: 66, y: 56, scale: 1.8, paint: "white" }],
    particles: [
      { part: "glitch", x: 50, y: 50, scale: 0.7, count: 2, spread: [6, 6] },
      { part: "motes", x: 60, y: 50, scale: 1.4, paint: "white" },
    ],
  },
};

/** Complexity Audit (Classic+ #45, Rare): an auditor unrolls a scroll so long it spills down a temple stairway, its far end crumbling into glitching static, in harsh magenta-tinged glare. */
const COMPLEXITY_AUDIT: SceneRecipe = {
  family: "classic",
  sky: ["violet-700", "purple-400"],
  light: { x: 50, y: 34, r: 34, paint: "purple-400", opacity: 0.5 },
  emblem: { x: 84, y: 44, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 62,
    scale: 0.7,
    pose: "pose-spread",
    look: { head: "head-human", hair: "hat-brim", outfit: "outfit-robe", item: "item-scroll", skin: "red-300", cloth: "slate-700" },
  },
  layers: {
    far: [
      { part: "temple", x: 50, y: 58, scale: 1.3 },
      { part: "banner", x: 82, y: 62, scale: 1.3, flip: -1 },
    ],
    mid: [{ part: "stairs", x: 50, y: 60, paint: "stone-500" }],
    subject: [
      { part: "scroll", x: 50, y: 67, scale: 0.6 },
      { part: "scroll", x: 50, y: 73, scale: 0.85 },
      { part: "scroll", x: 50, y: 80, scale: 1.1 },
      { part: "scroll", x: 50, y: 88, scale: 1.4 },
    ],
    near: [{ part: "glitch", x: 50, y: 95, scale: 1.4, count: 3, spread: [18, 3] }],
    light: [{ part: "halo", x: 50, y: 40, scale: 2, paint: "purple-400" }],
    particles: [{ part: "dust", x: 50, y: 92, scale: 1.2, paint: "purple-400" }],
  },
};

/** Felinor Flagbearer (Classic+ #46, Legendary): a lean cat-folk soldier in a red tabard charges over a broken temple bridge, his paw-print banner snapping, a tide of cat warriors surging behind as storm light breaks through. */
const FELINOR_FLAGBEARER: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "slate-700"],
  light: { x: 30, y: 26, r: 36, paint: "sky-300", opacity: 0.45 },
  emblem: { x: 15, y: 76, size: 5, rotate: 0 },
  figure: {
    x: 46,
    y: 82,
    scale: 0.95,
    pose: "pose-run",
    look: { ...figureLookFor(["Felinor", "Catalyst"]), outfit: "outfit-tabard", item: "item-banner", cloth: "red-600" },
  },
  extras: [
    {
      layer: "mid",
      x: 24,
      y: 76,
      scale: 0.48,
      pose: "pose-run",
      look: { head: "head-felinor", hair: null, outfit: "outfit-tabard", item: "item-spear", skin: "orange-400", cloth: "red-900" },
    },
    {
      layer: "mid",
      x: 34,
      y: 72,
      scale: 0.4,
      pose: "pose-run",
      look: { head: "head-felinor", hair: null, outfit: "outfit-armor", item: "item-sword", skin: "wood-600", cloth: "slate-400" },
    },
    {
      layer: "mid",
      x: 72,
      y: 72,
      scale: 0.4,
      pose: "pose-raise",
      look: { head: "head-felinor", hair: null, outfit: "outfit-armor", item: "item-spear", skin: "slate-400", cloth: "slate-400" },
    },
    {
      layer: "mid",
      x: 84,
      y: 76,
      scale: 0.46,
      pose: "pose-run",
      look: { head: "head-felinor", hair: null, outfit: "outfit-tabard", item: "item-sword", skin: "orange-400", cloth: "red-900" },
    },
  ],
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 20, scale: 1.6, count: 4, spread: [36, 5], paint: "slate-800" },
      { part: "lightning", x: 84, y: 16, scale: 0.6 },
    ],
    far: [
      { part: "mountains", x: 50, y: 66, paint: "slate-800" },
      { part: "temple", x: 80, y: 64, scale: 0.7, paint: "stone-500" },
      { part: "banner", x: 14, y: 62, scale: 0.8, props: true },
      { part: "banner", x: 60, y: 60, scale: 0.7, props: true },
    ],
    mid: [
      { part: "bridge", x: 50, y: 78, scale: 1.7, paint: "stone-500" },
      { part: "fallen-column", x: 70, y: 79, scale: 0.6 },
    ],
    near: [
      { part: "dust", x: 46, y: 88, scale: 1.8, paint: "stone-500" },
      { part: "banner", x: 10, y: 100, scale: 1.4, props: true },
      { part: "banner", x: 90, y: 100, scale: 1.4, flip: -1, props: true },
      { part: "pebbles", x: 50, y: 98, scale: 1.6 },
    ],
    light: [
      { part: "light-shaft", x: 30, y: 0, scale: 1.3, count: 2, spread: [12, 0], paint: "sky-300" },
      { part: "halo", x: 46, y: 44, scale: 2, paint: "sky-300" },
      { part: "rim-light", x: 46, y: 58, scale: 1.4, paint: "white" },
    ],
    particles: [
      { part: "dust", x: 30, y: 80, scale: 1.2, count: 2, spread: [16, 4], paint: "stone-500" },
      { part: "sparks", x: 50, y: 30, scale: 1.2, count: 3, spread: [20, 8], paint: "sky-300" },
      { part: "rain", x: 50, y: 34, scale: 1.2 },
    ],
  },
};

/** Felinor Flagbearer Prime (Classic+ #46.1, Token): a grizzled cat-folk standard-bearer plants a towering banner on a hilltop shrine as identical cat soldiers spring up around it, at bright windy noon. */
const FELINOR_FLAGBEARER_PRIME: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 72, y: 24, r: 30, paint: "white", opacity: 0.4 },
  emblem: { x: 76, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 48,
    y: 84,
    scale: 1,
    pose: "pose-guard",
    look: { ...figureLookFor(["Felinor", "Prime", "Token"]), hair: "helm", outfit: "outfit-armor", item: "item-banner", skin: "slate-400", cloth: "blue-800" },
  },
  extras: [
    {
      layer: "mid",
      x: 22,
      y: 82,
      scale: 0.55,
      pose: "pose-guard",
      look: { head: "head-felinor", hair: null, outfit: "outfit-armor", item: "item-spear", skin: "orange-400", cloth: "blue-800" },
    },
    {
      layer: "mid",
      x: 78,
      y: 82,
      scale: 0.55,
      flip: -1,
      pose: "pose-guard",
      look: { head: "head-felinor", hair: null, outfit: "outfit-armor", item: "item-spear", skin: "orange-400", cloth: "blue-800" },
    },
  ],
  layers: {
    sky: [
      { part: "sun", x: 72, y: 22, scale: 0.8 },
      { part: "cloud", x: 26, y: 26, scale: 1, count: 2, spread: [14, 4], paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 70, paint: "green-700" },
      { part: "standing-stone", x: 76, y: 70, scale: 0.9 },
    ],
    mid: [{ part: "field", x: 50, y: 78, paint: "green-700" }],
    subject: [{ part: "altar", x: 48, y: 86, scale: 1.2 }],
    near: [{ part: "grass-tuft", x: 50, y: 96, scale: 1.3, count: 5, spread: [40, 3], paint: "green-700" }],
    particles: [{ part: "petals", x: 50, y: 40, scale: 1.4, paint: "white" }],
  },
};

/** Jogg's Box (Classic+ #47, Legendary): a carved puzzle box bursts open on a cracked temple altar, flinging fire, frost, moths, a healing bloom and forked lightning as its hooded opener ducks behind a pillar at a sudden dusk. */
const JOGGS_BOX: SceneRecipe = {
  family: "classic",
  sky: ["violet-700", "orange-400"],
  light: { x: 50, y: 58, r: 40, paint: "purple-400", opacity: 0.5 },
  emblem: { x: 50, y: 78, size: 5, rotate: 0 },
  extras: [
    {
      layer: "mid",
      x: 12,
      y: 88,
      scale: 0.62,
      pose: "pose-kneel",
      look: { head: "head-human", hair: "hood", outfit: "outfit-coat", item: null, skin: "red-300", cloth: "wood-700" },
    },
  ],
  layers: {
    sky: [
      { part: "stars", x: 50, y: 18, scale: 1.4 },
      { part: "moon", x: 84, y: 20, scale: 0.45 },
    ],
    far: [
      { part: "ruin-wall", x: 80, y: 66, scale: 1.2 },
      { part: "column", x: 64, y: 66, scale: 0.8 },
      { part: "temple", x: 40, y: 64, scale: 0.8, paint: "slate-400" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "stone-500" },
      { part: "shockwave", x: 50, y: 70, scale: 1.4, paint: "white" },
    ],
    subject: [
      { part: "column", x: 22, y: 92, scale: 1.7 },
      { part: "altar", x: 50, y: 86, scale: 1.6 },
      { part: "chest", x: 50, y: 72, scale: 0.75, paint: "wood-600" },
      { part: "fireball", x: 72, y: 46, scale: 1, flip: 1 },
      { part: "crystal", x: 32, y: 50, scale: 1.8, paint: "sky-300" },
      { part: "moth", x: 40, y: 34, scale: 0.9, count: 3, spread: [10, 6] },
      { part: "flowers", x: 62, y: 62, scale: 1.4, paint: "green-400" },
      { part: "lightning", x: 56, y: 22, scale: 0.8 },
    ],
    near: [
      { part: "fallen-column", x: 80, y: 96, scale: 1.2 },
      { part: "pebbles", x: 50, y: 98, scale: 1.6 },
    ],
    light: [
      { part: "halo", x: 50, y: 64, scale: 2.4, paint: "purple-400" },
      { part: "halo", x: 72, y: 46, scale: 1.2, paint: "orange-400" },
      { part: "halo", x: 62, y: 58, scale: 1, paint: "green-400" },
      { part: "halo", x: 32, y: 48, scale: 1, paint: "sky-300" },
      { part: "god-rays", x: 50, y: 64, scale: 0.7, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 50, scale: 1.4, count: 4, spread: [26, 16], paint: "orange-400" },
      { part: "sparks", x: 50, y: 50, scale: 1.2, count: 3, spread: [26, 16], paint: "sky-300" },
      { part: "embers", x: 50, y: 60, scale: 1.4, count: 2, spread: [20, 8], paint: "green-400" },
    ],
  },
};

/** Jlockeed's Lobbyist (Classic+ #48, Legendary): a slick man in a pinstriped grey coat slides a wax-sealed contract across a marble table in a columned council chamber as porters wheel in a model jet and missile crates. */
const JLOCKEEDS_LOBBYIST: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-800"],
  light: { x: 30, y: 22, r: 34, paint: "sky-300", opacity: 0.4 },
  emblem: { x: 86, y: 44, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 92,
    scale: 1.1,
    pose: "pose-guard",
    look: { ...figureLookFor(["Jlockeed"]), hair: "hair-short", outfit: "outfit-coat", item: "item-scroll", cloth: "grey-500" },
  },
  extras: [
    {
      layer: "mid",
      x: 16,
      y: 78,
      scale: 0.5,
      pose: "pose-run",
      look: { head: "head-human", hair: "kerchief", outfit: "outfit-tunic", item: null, skin: "red-300", cloth: "slate-700" },
    },
    {
      layer: "mid",
      x: 76,
      y: 78,
      scale: 0.5,
      flip: -1,
      pose: "pose-run",
      look: { head: "head-human", hair: "hat-brim", outfit: "outfit-tunic", item: null, skin: "red-300", cloth: "slate-700" },
    },
  ],
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 70, paint: "slate-700" }],
    far: [
      { part: "column", x: 10, y: 74, scale: 1.7, paint: "slate-400" },
      { part: "column", x: 36, y: 72, scale: 1.4, paint: "slate-400" },
      { part: "column", x: 64, y: 72, scale: 1.4, paint: "slate-400" },
      { part: "column", x: 90, y: 74, scale: 1.7, paint: "slate-400" },
      { part: "banner", x: 82, y: 66, scale: 1.4, props: true },
      { part: "banner", x: 20, y: 66, scale: 1.2, flip: -1, props: true },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 70, paint: "slate-400" },
      { part: "cart", x: 28, y: 78, scale: 1 },
      { part: "jet", x: 28, y: 64, scale: 0.4 },
      { part: "crate", x: 86, y: 78, scale: 1, count: 3, spread: [4, 1] },
    ],
    near: [
      { part: "table", x: 50, y: 100, scale: 2.6, paint: "slate-100" },
      { part: "scroll", x: 58, y: 81, scale: 1.1 },
      { part: "coins", x: 34, y: 81, scale: 0.9, props: true },
      { part: "quill", x: 72, y: 81, scale: 0.8 },
    ],
    light: [
      { part: "light-shaft", x: 26, y: 0, scale: 1.3, count: 2, spread: [14, 0], paint: "sky-300" },
      { part: "halo", x: 50, y: 50, scale: 1.8, paint: "blue-200" },
    ],
    particles: [
      { part: "smoke", x: 72, y: 78, scale: 1.2, paint: "slate-400" },
      { part: "motes", x: 40, y: 40, scale: 1.4, paint: "sky-300" },
    ],
  },
};

/** Jay Fungus (Classic+ #49, Rare): a broad, friendly toadstool shopkeeper with a mossy beard slides a discount coin across his stall in a temple arcade, in cosy green light. */
const JAY_FUNGUS: SceneRecipe = {
  family: "classic",
  sky: ["green-900", "green-700"],
  light: { x: 50, y: 44, r: 34, paint: "green-200", opacity: 0.35 },
  emblem: { x: 14, y: 46, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 90,
    scale: 1.05,
    pose: "pose-stand",
    look: { ...figureLookFor([]), head: "head-mushroom", hair: null, outfit: "outfit-apron", item: "item-coin", skin: "slate-100", cloth: "green-700" },
  },
  layers: {
    far: [
      { part: "arch", x: 14, y: 72, scale: 1.4, paint: "stone-500" },
      { part: "arch", x: 86, y: 72, scale: 1.4, paint: "stone-500" },
      { part: "tile-floor", x: 50, y: 72, paint: "stone-500" },
    ],
    mid: [{ part: "awning", x: 50, y: 82, scale: 2.4 }],
    near: [
      { part: "table", x: 50, y: 100, scale: 2.4, paint: "wood-700" },
      { part: "mushroom", x: 26, y: 82, scale: 1.2, count: 3, spread: [5, 0] },
      { part: "basket", x: 74, y: 82, scale: 1 },
      { part: "coins", x: 58, y: 82, scale: 0.7 },
    ],
    light: [{ part: "halo", x: 50, y: 50, scale: 1.8, paint: "green-200" }],
    particles: [{ part: "spores", x: 50, y: 40, scale: 1.4, paint: "green-200" }],
  },
};

/** Adaptive Growth (Classic+ #50, Epic): a vine-choked temple terrace split down the middle, one side's soldiers bulking up in swelling vines and the other's wilting under withered leaves, before a lopsided balance. */
const ADAPTIVE_GROWTH: SceneRecipe = {
  family: "classic",
  sky: ["green-900", "wood-700"],
  light: { x: 30, y: 40, r: 34, paint: "green-400", opacity: 0.35 },
  emblem: { x: 50, y: 30, size: 5, rotate: 0 },
  figure: {
    x: 40,
    y: 88,
    scale: 1,
    pose: "pose-flex",
    look: { head: "head-human", hair: "hair-short", outfit: "outfit-muscle", item: null, skin: "red-300", cloth: "green-700" },
  },
  extras: [
    {
      layer: "mid",
      x: 74,
      y: 80,
      scale: 0.6,
      flip: -1,
      pose: "pose-kneel",
      look: { head: "head-human", hair: "helm", outfit: "outfit-armor", item: "item-sword", skin: "red-300", cloth: "wood-600" },
    },
    {
      layer: "mid",
      x: 86,
      y: 78,
      scale: 0.5,
      flip: -1,
      pose: "pose-sit",
      look: { head: "head-human", hair: "helm", outfit: "outfit-armor", item: null, skin: "red-300", cloth: "wood-700" },
    },
  ],
  layers: {
    sky: [{ part: "cloud", x: 30, y: 20, scale: 1.2, count: 2, spread: [30, 4], paint: "green-700" }],
    far: [
      { part: "scales", x: 50, y: 62, scale: 1.4 },
      { part: "column", x: 14, y: 64, scale: 1.2 },
      { part: "column", x: 86, y: 64, scale: 1.2 },
      { part: "vine", x: 14, y: 64, scale: 1.4 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 64, paint: "stone-500" },
      { part: "dead-tree", x: 90, y: 72, scale: 1.2 },
    ],
    subject: [
      { part: "vine", x: 28, y: 90, scale: 2.2 },
      { part: "vine", x: 54, y: 90, scale: 2, flip: -1 },
    ],
    near: [
      { part: "fern", x: 12, y: 98, scale: 1.6 },
      { part: "bush", x: 30, y: 100, scale: 1.2 },
      { part: "petals", x: 80, y: 94, scale: 1.2, paint: "wood-600" },
      { part: "grass-tuft", x: 70, y: 98, count: 3, spread: [12, 2], paint: "wood-600" },
    ],
    light: [{ part: "light-shaft", x: 30, y: 0, scale: 1.2, count: 3, spread: [16, 0], paint: "green-200" }],
    particles: [
      { part: "petals", x: 76, y: 60, scale: 1.2, paint: "wood-600" },
      { part: "spores", x: 32, y: 60, scale: 1.2, paint: "green-400" },
    ],
  },
};

/** Jlockeed's J15 Fighter (Classic+ #51, Epic): a sleek grey fighter jet banks high above an old-world mountain citadel, trailing its contrail, as archers on the battlements loose arrows that fall short. */
const JLOCKEEDS_J15_FIGHTER: SceneRecipe = {
  family: "classic",
  sky: ["blue-700", "sky-300"],
  light: { x: 60, y: 32, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 44, y: 38, size: 4, rotate: 0 },
  extras: [
    {
      layer: "near",
      x: 16,
      y: 92,
      scale: 0.6,
      pose: "pose-guard",
      look: { head: "head-human", hair: "helm", outfit: "outfit-tabard", item: "item-bow", skin: "red-300", cloth: "red-600" },
    },
    {
      layer: "near",
      x: 86,
      y: 94,
      scale: 0.6,
      flip: -1,
      pose: "pose-guard",
      look: { head: "head-human", hair: "hood", outfit: "outfit-tabard", item: "item-bow", skin: "red-300", cloth: "red-600" },
    },
  ],
  layers: {
    sky: [{ part: "cloud", x: 40, y: 44, scale: 2.2, count: 4, spread: [40, 12], paint: "white" }],
    far: [
      { part: "mountains", x: 50, y: 78, paint: "slate-700" },
      { part: "tower", x: 50, y: 74, scale: 0.8, count: 3, spread: [16, 2] },
    ],
    mid: [
      { part: "mist", x: 30, y: 42, scale: 1.2, count: 2, spread: [8, 2], paint: "white" },
      { part: "arrow", x: 40, y: 66, scale: 0.7, count: 3, spread: [16, 6] },
    ],
    subject: [{ part: "jet", x: 58, y: 38, scale: 1.6 }],
    near: [
      { part: "ruin-wall", x: 16, y: 102, scale: 2 },
      { part: "ruin-wall", x: 86, y: 102, scale: 2, flip: -1 },
    ],
    light: [
      { part: "rim-light", x: 58, y: 38, scale: 1.4, paint: "white" },
      { part: "halo", x: 60, y: 32, scale: 2, paint: "white" },
    ],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1.4, paint: "white" }],
  },
};

/** Jlockeed's Permanent Defense Contract (Classic+ #52, Epic): a fortress courtyard at dawn, a wax-sealed contract unrolling endlessly from a stone pedestal beside its quill as carts of arms arrive and the sun rises behind the barracks. */
const PERMANENT_DEFENSE_CONTRACT: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "orange-400"],
  light: { x: 50, y: 56, r: 36, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 50, y: 78, size: 6, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 50, y: 56, scale: 1.6 },
      { part: "cloud", x: 24, y: 26, scale: 1, count: 2, spread: [30, 4], paint: "slate-400" },
    ],
    far: [
      { part: "house", x: 22, y: 64, scale: 1 },
      { part: "house", x: 78, y: 64, scale: 1, flip: -1 },
      { part: "tower", x: 8, y: 66, scale: 1 },
      { part: "tower", x: 92, y: 66, scale: 1 },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 64, paint: "slate-700" },
      { part: "cart", x: 24, y: 74, scale: 1 },
      { part: "cart", x: 50, y: 70, scale: 0.8 },
      { part: "cart", x: 76, y: 74, scale: 1, flip: -1 },
      { part: "weapon-rack", x: 24, y: 66, scale: 0.4 },
      { part: "crate", x: 76, y: 67, scale: 0.8 },
    ],
    subject: [
      { part: "altar", x: 50, y: 86, scale: 1.6, paint: "stone-500" },
      { part: "scroll", x: 50, y: 72, scale: 1.2 },
    ],
    near: [
      { part: "scroll", x: 50, y: 96, scale: 2.4, count: 2, spread: [3, 2] },
      { part: "quill", x: 24, y: 98, scale: 1.6 },
    ],
    light: [
      { part: "god-rays", x: 50, y: 56, scale: 0.8, paint: "orange-400" },
      { part: "halo", x: 50, y: 70, scale: 1.4, paint: "orange-400" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "slate-100" }],
  },
};

/** R3390000: Classic+ #27 to #52 and their tokens, by id. */
export const CLASSIC_PLUS_RECIPES_2: Readonly<Record<string, SceneRecipe>> = {
  "classicplus-027": ZEPHRYS_ZEALOTISM,
  "classicplus-028": NUESTRO_HOGAR,
  "classicplus-029": PORTAL_TO_THE_PAST,
  "classicplus-030": FELINOR_FUSER,
  "classicplus-031": FUSION_LAB,
  "classicplus-032": OTHERWORLDLY_REMOVAL,
  "classicplus-032-1": EXECUTE,
  "classicplus-032-2": BRAWL,
  "classicplus-032-3": BLADE_STORM,
  "classicplus-033": IVORY_TOWER,
  "classicplus-034": MEMORY_LEAK,
  "classicplus-035": ROLLBACK,
  "classicplus-036": CONJURE_BONES,
  "classicplus-036-1": BONE_STORM,
  "classicplus-037": WARDRUM,
  "classicplus-038": SOLARIUS,
  "classicplus-038-1": SOLARIUS_PRIME,
  "classicplus-039": BOOK_WORM,
  "classicplus-040": APPROPRIATIONS,
  "classicplus-041": KYS_CONSTANT,
  "classicplus-042": KYS_TEST,
  "classicplus-042-1": KYS_GIFT,
  "classicplus-043": AI_SLOP,
  "classicplus-044": SIMPLICITY_AUDIT,
  "classicplus-045": COMPLEXITY_AUDIT,
  "classicplus-046": FELINOR_FLAGBEARER,
  "classicplus-046-1": FELINOR_FLAGBEARER_PRIME,
  "classicplus-047": JOGGS_BOX,
  "classicplus-048": JLOCKEEDS_LOBBYIST,
  "classicplus-049": JAY_FUNGUS,
  "classicplus-050": ADAPTIVE_GROWTH,
  "classicplus-051": JLOCKEEDS_J15_FIGHTER,
  "classicplus-052": PERMANENT_DEFENSE_CONTRACT,
};
