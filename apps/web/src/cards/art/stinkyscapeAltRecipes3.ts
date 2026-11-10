// R3394000: the alt scene recipes of Stinkyscape's cards 1 to 90 (#3394), cards 62 to 88 and the Dormant Starship
// (stinkyscapeAltRecipes1.ts says what each is held to). Presentation only (CLAUDE.md rule 7): a recipe is read by the
// card's id alone.

import { figureLookFor } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";
import { normalLookOf } from "./stinkyscapeAltRecipes1.ts";

/** Miner v1 (Stinkyscape #62, Common), Alt art: pocket-sized Miner v1 basking on a sunlit pebble of an asteroid, solar panels spread wide, proudly tapping a dull rock. Close-up, white sun, blue shadows. */
const MINER_V1_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["sky-300", "blue-200"],
  light: { x: 76, y: 22, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 78, y: 20, scale: 1 },
      { part: "stars", x: 30, y: 22, scale: 0.8 },
    ],
    far: [{ part: "asteroid", x: 16, y: 44, scale: 0.7, count: 3, spread: [10, 8] }],
    mid: [{ part: "asteroid", x: 50, y: 96, scale: 7, paint: "slate-400" }],
    subject: [
      { part: "mining-drill", x: 36, y: 62, scale: 0.8 },
      { part: "solar-panels", x: 44, y: 50, scale: 1.1 },
      { part: "ore-chunk", x: 64, y: 66, scale: 0.8, paint: "slate-400" },
    ],
    near: [{ part: "pebbles", x: 80, y: 96, scale: 1.4, count: 2, spread: [10, 2] }],
    light: [{ part: "rim-light", x: 44, y: 58, scale: 1, paint: "white" }],
    particles: [{ part: "sparks", x: 62, y: 64, scale: 0.5, paint: "white" }],
  },
};

/** Miner v3 (Stinkyscape #64, Rare), Alt art: Miner v3's twin drills idle at a union break on an asteroid, little worker bots sitting along its rim with flasks. Soft lilac starlight. The kit's round bots hold nothing, so their flasks stand beside them. */
const MINER_V3_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-500", "violet-300"],
  light: { x: 26, y: 22, r: 34, paint: "white", opacity: 0.3 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 1.4 },
      { part: "planet", x: 22, y: 24, scale: 0.7, paint: "violet-300" },
    ],
    far: [{ part: "asteroid", x: 80, y: 46, scale: 1.4, count: 2, spread: [10, 6] }],
    mid: [{ part: "regolith", x: 50, y: 72, paint: "slate-400" }],
    subject: [
      { part: "mining-drill", x: 32, y: 56, scale: 0.9 },
      { part: "mining-drill", x: 34, y: 66, scale: 0.9 },
      { part: "round-bot", x: 60, y: 80, scale: 0.8, count: 3, spread: [12, 2] },
    ],
    near: [
      { part: "flask", x: 70, y: 92, scale: 0.8, count: 2, spread: [10, 2] },
      { part: "pebbles", x: 18, y: 96, scale: 1.2 },
    ],
    light: [{ part: "rim-light", x: 46, y: 62, scale: 1, paint: "violet-300" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1, paint: "white" }],
  },
};

/** Miner v4 (Stinkyscape #65, Epic), Alt art: Miner v4 chewing through an ice comet as its tail glows white-blue, ore pouring out faster than a cargo hauler can catch it. Wide vista. */
const MINER_V4_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-700", "sky-300"],
  light: { x: 62, y: 46, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 16, y: 78, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 20, scale: 1.6 },
      { part: "nebula", x: 24, y: 26, scale: 1, paint: "sky-300" },
    ],
    far: [{ part: "gas-giant", x: 18, y: 30, scale: 0.8 }],
    mid: [{ part: "freighter", x: 78, y: 76, scale: 0.6, flip: -1 }],
    subject: [
      { part: "mining-drill", x: 32, y: 48, scale: 1 },
      { part: "comet", x: 62, y: 46, scale: 1.5, flip: -1 },
      { part: "ore-chunk", x: 54, y: 62, scale: 0.7, count: 4, spread: [12, 8], paint: "sky-400" },
    ],
    near: [
      { part: "ice-shard", x: 14, y: 92, scale: 1.2 },
      { part: "ore-chunk", x: 86, y: 94, scale: 1.2, count: 2, spread: [6, 3], paint: "sky-400" },
    ],
    light: [{ part: "halo", x: 62, y: 46, scale: 1.4, paint: "white" }],
    particles: [
      { part: "sparks", x: 50, y: 48, scale: 0.8, paint: "white" },
      { part: "snow", x: 70, y: 40, scale: 1, paint: "white" },
    ],
  },
};

/** Miner v5 (Stinkyscape #66, Legendary), Alt art: Miner v5 swallowing a whole asteroid at the mouth of a vast trench on a dark moon, a glittering stream of resources pouring from its back. Overhead view. */
const MINER_V5_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-400", "blue-600"],
  light: { x: 56, y: 52, r: 40, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 16, y: 78, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 14, scale: 1.4 },
      { part: "planet", x: 82, y: 18, scale: 0.6, paint: "blue-200" },
    ],
    far: [
      { part: "regolith", x: 50, y: 40, paint: "slate-400" },
      { part: "bowl-crater", x: 50, y: 64, scale: 2.6 },
    ],
    mid: [
      { part: "ore-chunk", x: 18, y: 52, scale: 0.6, count: 5, spread: [8, 12], paint: "cyan-400" },
      { part: "asteroid", x: 84, y: 66, scale: 1.2, count: 3, spread: [6, 10] },
    ],
    subject: [
      { part: "mining-drill", x: 32, y: 56, scale: 1.4 },
      { part: "asteroid", x: 66, y: 54, scale: 3.4 },
      { part: "ore-chunk", x: 44, y: 44, scale: 0.6, count: 3, spread: [10, 4], paint: "cyan-400" },
    ],
    near: [
      { part: "debris", x: 86, y: 92, scale: 1.4 },
      { part: "pebbles", x: 18, y: 96, scale: 1.4 },
    ],
    light: [
      { part: "halo", x: 60, y: 54, scale: 1.6, paint: "cyan-400" },
      { part: "rim-light", x: 40, y: 56, scale: 1.2, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 52, y: 54, scale: 1, paint: "white" },
      { part: "dust", x: 60, y: 70, scale: 1.2, paint: "slate-100" },
    ],
  },
};

/** Stratos (Stinkyscape #68, Common), Alt art: a dozen Stratos hulls flying in a ragged swarm over an ocean planet's storm clouds, lightning flashing below. Grey-green light, wide angle. */
const STRATOS_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["stone-500", "jade-400"],
  light: { x: 50, y: 72, r: 34, paint: "white", opacity: 0.3 },
  emblem: { x: 84, y: 26, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 24, y: 20, scale: 1.2, paint: "slate-400" }],
    far: [
      { part: "sea", x: 50, y: 84, paint: "green-700" },
      { part: "wedge-fighter", x: 70, y: 30, scale: 0.5, count: 6, spread: [18, 8] },
    ],
    mid: [
      { part: "cloud", x: 28, y: 74, scale: 1.6, paint: "slate-400" },
      { part: "cloud", x: 76, y: 80, scale: 1.4, paint: "slate-700" },
    ],
    subject: [{ part: "wedge-fighter", x: 46, y: 46, scale: 1, count: 6, spread: [20, 12] }],
    light: [{ part: "lightning", x: 60, y: 78, scale: 0.6, paint: "white" }],
    particles: [{ part: "rain", x: 50, y: 60, scale: 1.2, paint: "slate-100" }],
  },
};

/** Falcon (Stinkyscape #69, Rare), Alt art: the Falcon after a mission, parked crooked in a rain-soaked spaceport, a mechanic patching its scratched hull under one orange floodlight. A placement may not turn the fighter, so it is parked crooked by where it stands, not by its angle. */
const FALCON_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-800", "slate-700"],
  light: { x: 74, y: 22, r: 34, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  extras: [
    {
      layer: "subject",
      x: 68,
      y: 80,
      scale: 0.42,
      flip: -1,
      pose: "pose-reach",
      look: { ...figureLookFor(["Human"]), hair: "beanie", outfit: "outfit-overalls", item: "item-hammer", skin: "wood-700", cloth: "orange-400" },
    },
  ],
  layers: {
    sky: [{ part: "cloud", x: 30, y: 18, scale: 1.4, paint: "slate-400" }],
    far: [
      { part: "tower", x: 14, y: 70, scale: 1 },
      { part: "antenna-mast", x: 88, y: 66, scale: 0.8 },
    ],
    mid: [{ part: "launch-pad", x: 50, y: 80, scale: 1.6 }],
    subject: [{ part: "needle-fighter", x: 46, y: 64, scale: 1.3 }],
    near: [
      { part: "streetlamp", x: 86, y: 98, scale: 1.4 },
      { part: "crate", x: 12, y: 98, scale: 1.3 },
    ],
    light: [{ part: "halo", x: 82, y: 54, scale: 1.4, paint: "orange-400" }],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.3, paint: "sky-300" },
      { part: "sparks", x: 62, y: 68, scale: 0.5, paint: "orange-400" },
    ],
  },
};

/** Z'Zoth (Stinkyscape #70, Legendary), Alt art: before the call: Z'Zoth asleep, a vast hooded shape curled inside a frozen debris ring, seen from far off in a red giant's rose-pink dawn. The figure kit has no curled pose, so it kneels. */
const ZZOTH_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-500", "red-300"],
  light: { x: 78, y: 72, r: 40, paint: "red-300", opacity: 0.5 },
  emblem: { x: 18, y: 28, size: 5, rotate: 0 },
  figure: { x: 50, y: 70, scale: 0.6, pose: "pose-kneel", look: normalLookOf("stinkyscape-070") },
  layers: {
    sky: [
      { part: "stars", x: 30, y: 18, scale: 1.2 },
      { part: "star-glare", x: 80, y: 74, scale: 1.4, paint: "red-400" },
    ],
    far: [{ part: "sun", x: 80, y: 76, scale: 1.6, paint: "red-400" }],
    mid: [
      { part: "debris", x: 28, y: 60, scale: 1.2, count: 3, spread: [10, 8] },
      { part: "ice-shard", x: 72, y: 56, scale: 0.8, count: 3, spread: [8, 6] },
    ],
    subject: [
      { part: "debris", x: 50, y: 52, scale: 1.1, count: 3, spread: [16, 6] },
      { part: "ice-shard", x: 40, y: 66, scale: 0.7, count: 2, spread: [6, 4] },
    ],
    near: [
      { part: "ice-shard", x: 12, y: 92, scale: 1.4 },
      { part: "debris", x: 88, y: 90, scale: 1.3 },
    ],
    light: [
      { part: "rim-light", x: 50, y: 56, scale: 1, paint: "red-300" },
      { part: "halo", x: 80, y: 74, scale: 2, paint: "red-300" },
    ],
    particles: [
      { part: "snow", x: 50, y: 50, scale: 1, paint: "white" },
      { part: "motes", x: 50, y: 40, scale: 1.2, paint: "red-300" },
    ],
  },
};

/** Paragon Mk1 (Stinkyscape #72, Epic), Alt art: Paragon Mk1 docked at an orbital office tower, its pilot filing paperwork at a desk by the window, the ship parked outside. Warm sunset orange. */
const PARAGON_MK1_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["orange-400", "red-300"],
  light: { x: 64, y: 40, r: 38, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 84, y: 76, size: 5, rotate: 0 },
  figure: {
    x: 30,
    y: 96,
    scale: 1.1,
    pose: "pose-sit",
    look: { ...figureLookFor(["Human"]), hair: "headset", outfit: "outfit-flightsuit", item: "item-clipboard", skin: "red-300", cloth: "blue-600" },
  },
  layers: {
    sky: [{ part: "stars", x: 64, y: 22, scale: 0.8 }],
    far: [
      { part: "gas-giant", x: 76, y: 30, scale: 0.8 },
      { part: "viewport", x: 64, y: 44, scale: 1.3 },
    ],
    mid: [{ part: "bookshelf", x: 90, y: 90, scale: 1.1, paint: "wood-700" }],
    subject: [{ part: "arrow-fighter", x: 64, y: 42, scale: 0.9 }],
    near: [
      { part: "desk", x: 30, y: 106, scale: 2.4, paint: "wood-700" },
      { part: "pages", x: 34, y: 82, scale: 0.6 },
    ],
    light: [{ part: "light-shaft", x: 64, y: 30, scale: 1, paint: "orange-400" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1, paint: "orange-400" }],
  },
};

/** Paragon Mk2 (Stinkyscape #73, Legendary), Alt art: Paragon Mk2 on a test stand in a vast white hangar, its shield bubble switched on for the first time as engineers step back. Clean overhead view. */
const PARAGON_MK2_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-100", "white"],
  light: { x: 50, y: 54, r: 40, paint: "sky-300", opacity: 0.3 },
  emblem: { x: 16, y: 78, size: 5, rotate: 0 },
  extras: [
    {
      layer: "mid",
      x: 18,
      y: 72,
      scale: 0.4,
      pose: "pose-stand",
      look: { ...figureLookFor(["Human"]), hair: "goggles", outfit: "outfit-labcoat", item: "item-clipboard", skin: "red-300", cloth: "slate-100" },
    },
    {
      layer: "mid",
      x: 84,
      y: 76,
      scale: 0.4,
      flip: -1,
      pose: "pose-raise",
      look: { ...figureLookFor(["Human"]), hair: "headset", outfit: "outfit-labcoat", item: null, skin: "wood-700", cloth: "slate-100" },
    },
  ],
  layers: {
    sky: [{ part: "ceiling-lights", x: 50, y: 8, scale: 1.4 }],
    far: [
      { part: "deck-floor", x: 50, y: 34 },
      { part: "gantry", x: 50, y: 42, scale: 0.9 },
    ],
    mid: [{ part: "launch-pad", x: 50, y: 72, scale: 1.6 }],
    subject: [
      { part: "arrow-fighter", x: 50, y: 58, scale: 1.2 },
      { part: "hex-shield", x: 50, y: 58, scale: 1.3 },
    ],
    near: [
      { part: "crate", x: 10, y: 100, scale: 1.2 },
      { part: "cable", x: 66, y: 96, scale: 1 },
    ],
    light: [
      { part: "halo", x: 50, y: 58, scale: 1.8, paint: "sky-300" },
      { part: "rim-light", x: 50, y: 50, scale: 1.2, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 58, scale: 1, paint: "cyan-400" },
      { part: "motes", x: 50, y: 40, scale: 1.2, paint: "white" },
    ],
  },
};

/** Badger (Stinkyscape #75, Common), Alt art: Badger's burrow: a cave in an asteroid lined with its hoard of shiny rocks, Badger curled up among them. Cool blue cave glow. The kit's miner craft does not curl, so it rests among its rocks. */
const BADGER_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-900", "blue-600"],
  light: { x: 50, y: 66, r: 34, paint: "sky-300", opacity: 0.4 },
  emblem: { x: 84, y: 76, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "cave", x: 50, y: 62, scale: 1.2 }],
    mid: [{ part: "cavern", x: 50, y: 80 }],
    subject: [
      { part: "miner-craft", x: 50, y: 70, scale: 1.2 },
      { part: "crystal", x: 50, y: 74, scale: 0.8, count: 4, spread: [18, 4], paint: "sky-300" },
    ],
    near: [{ part: "crystal", x: 12, y: 96, scale: 1.2, count: 2, spread: [6, 2], paint: "cyan-400" }],
    light: [{ part: "halo", x: 50, y: 70, scale: 1.4, paint: "sky-300" }],
    particles: [{ part: "motes", x: 50, y: 60, scale: 1, paint: "sky-300" }],
  },
};

/** Yuma (Stinkyscape #77, Epic), Alt art: before the offer: Yuma hovering at a rival's airlock in a misty violet nebula, holding out a crate like a gift as the hatch begins to open. The kit's airlock stands open, so the hatch is swung aside. */
const YUMA_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-500", "purple-400"],
  light: { x: 74, y: 50, r: 36, paint: "violet-300", opacity: 0.4 },
  emblem: { x: 18, y: 28, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "nebula", x: 30, y: 30, scale: 1.4, paint: "violet-300" },
      { part: "stars", x: 50, y: 20, scale: 1.2 },
    ],
    far: [{ part: "nebula", x: 78, y: 70, scale: 1.2, paint: "purple-400" }],
    mid: [
      { part: "freighter", x: 86, y: 52, scale: 1.4, flip: -1 },
      { part: "airlock", x: 76, y: 52, scale: 0.8 },
    ],
    subject: [
      { part: "trader", x: 42, y: 54, scale: 1.3 },
      { part: "crate", x: 56, y: 62, scale: 0.8 },
    ],
    near: [{ part: "mist", x: 50, y: 88, scale: 1.4 }],
    light: [{ part: "halo", x: 76, y: 52, scale: 1.4, paint: "violet-300" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "violet-300" }],
  },
};

/** Ozark (Stinkyscape #78, Epic), Alt art: rent day inside Ozark's cavernous teal-lit hold: tiny miner pods queueing at the barge's landlord bay, seen from above. */
const OZARK_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["jade-400", "cyan-400"],
  light: { x: 74, y: 40, r: 36, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 16, y: 78, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "ceiling-lights", x: 50, y: 8, scale: 1.2 }],
    far: [{ part: "bulkhead", x: 50, y: 34 }],
    mid: [
      { part: "deck-floor", x: 50, y: 40 },
      { part: "blast-door", x: 74, y: 48, scale: 0.9 },
    ],
    subject: [
      { part: "miner-craft", x: 34, y: 76, scale: 0.6 },
      { part: "miner-craft", x: 42, y: 68, scale: 0.6 },
      { part: "miner-craft", x: 50, y: 61, scale: 0.6 },
      { part: "miner-craft", x: 58, y: 54, scale: 0.6 },
      { part: "credit-chip", x: 66, y: 50, scale: 0.6 },
    ],
    near: [{ part: "crate", x: 12, y: 100, scale: 1.3, count: 2, spread: [8, 2] }],
    light: [{ part: "halo", x: 74, y: 40, scale: 1.4, paint: "cyan-400" }],
    particles: [{ part: "motes", x: 50, y: 60, scale: 1, paint: "white" }],
  },
};

/** Orca (Stinkyscape #79, Epic), Alt art: Orca gliding slowly beneath an asteroid shoal like a whale under fish, the jaws of its mining bay wide open. Deep blue-black space, long side view. */
const ORCA_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-900", "blue-800"],
  light: { x: 60, y: 66, r: 36, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 16, y: 80, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 30, scale: 1.6 }],
    far: [{ part: "asteroid", x: 50, y: 22, scale: 0.6, count: 6, spread: [36, 6] }],
    mid: [{ part: "asteroid", x: 50, y: 36, scale: 1.2, count: 5, spread: [30, 8] }],
    subject: [{ part: "orca-hull", x: 50, y: 64, scale: 1.3 }],
    near: [{ part: "asteroid", x: 88, y: 94, scale: 2 }],
    light: [{ part: "halo", x: 72, y: 64, scale: 1.2, paint: "orange-400" }],
    particles: [{ part: "dust", x: 50, y: 46, scale: 1.2, paint: "sky-300" }],
  },
};

/** Radix (Stinkyscape #81, Rare), Alt art: Radix drifting through a junkyard orbit at a red sun's dusk, catching weapons that tumble past in its two open hardpoints. Deep red light, playful wide view. */
const RADIX_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-900", "red-600"],
  light: { x: 78, y: 30, r: 38, paint: "red-400", opacity: 0.45 },
  emblem: { x: 16, y: 78, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 80, y: 28, scale: 1.2, paint: "red-400" },
      { part: "stars", x: 30, y: 20, scale: 1 },
    ],
    far: [{ part: "wreckage", x: 20, y: 36, scale: 0.6, count: 2, spread: [10, 6] }],
    mid: [{ part: "debris", x: 50, y: 78, scale: 1.2, count: 3, spread: [30, 6] }],
    subject: [
      { part: "frame-ship", x: 46, y: 56, scale: 1.3 },
      { part: "ray-gun", x: 62, y: 42, scale: 0.5 },
      { part: "rifle", x: 34, y: 40, scale: 0.5 },
    ],
    near: [{ part: "wreckage", x: 88, y: 92, scale: 1 }],
    light: [{ part: "rim-light", x: 46, y: 56, scale: 1.2, paint: "red-300" }],
    particles: [{ part: "sparks", x: 50, y: 50, scale: 1, paint: "orange-400" }],
  },
};

/** Phalanx (Stinkyscape #82, Epic), Alt art: Phalanx curled up at rest in a mossy green hangar on a jungle moon, its gun quills folded, a mechanic napping against its shell. The kit's gunship cannot fold its quills, so they stand. */
const PHALANX_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["green-700", "green-400"],
  light: { x: 30, y: 26, r: 34, paint: "lime-200", opacity: 0.35 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  extras: [
    {
      layer: "subject",
      x: 72,
      y: 82,
      scale: 0.42,
      flip: -1,
      pose: "pose-sit",
      look: { ...figureLookFor(["Human"]), hair: "flap-cap", outfit: "outfit-overalls", item: null, skin: "red-300", cloth: "brass-600" },
    },
  ],
  layers: {
    sky: [{ part: "light-shaft", x: 30, y: 4, scale: 1, paint: "lime-200" }],
    far: [{ part: "hangar-wall", x: 50, y: 62, paint: "green-900" }],
    mid: [
      { part: "deck-floor", x: 50, y: 62 },
      { part: "vine", x: 12, y: 40, scale: 1.4 },
    ],
    subject: [{ part: "gunship", x: 50, y: 70, scale: 1.3 }],
    near: [
      { part: "fern", x: 10, y: 98, scale: 1.8 },
      { part: "fern", x: 90, y: 98, scale: 1.6 },
    ],
    light: [{ part: "rim-light", x: 50, y: 60, scale: 1.2, paint: "lime-200" }],
    particles: [{ part: "spore-motes", x: 50, y: 50, scale: 1, paint: "lime-200" }],
  },
};

/** Halo (Stinkyscape #83, Epic), Alt art: Halo seen from far above: a tiny ring spinning over a rust-red canyon world, throwing a circle of muzzle flashes outward. Wide overhead view. Too far off to see, its pilot is left out. */
const HALO_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-600", "brass-600"],
  light: { x: 50, y: 52, r: 30, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 84, y: 78, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 28, y: 30, scale: 1, paint: "slate-100" }],
    far: [{ part: "cracked-earth", x: 50, y: 24, paint: "red-600" }],
    mid: [
      { part: "mesa", x: 22, y: 82, scale: 0.8 },
      { part: "mesa", x: 80, y: 46, scale: 0.6 },
      { part: "mesa", x: 70, y: 90, scale: 0.5 },
    ],
    subject: [
      { part: "gun-ring", x: 50, y: 52, scale: 0.6 },
      { part: "blast-rings", x: 50, y: 52, scale: 1.4 },
    ],
    near: [
      { part: "cliff", x: 14, y: 100, scale: 1.4 },
      { part: "pebbles", x: 84, y: 98, scale: 1.4 },
    ],
    light: [
      { part: "halo", x: 50, y: 52, scale: 1.4, paint: "orange-400" },
      { part: "echo-rings", x: 50, y: 52, scale: 1.2, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 52, scale: 1.2, paint: "white" },
      { part: "embers", x: 50, y: 60, scale: 1, paint: "orange-400" },
      { part: "dust", x: 60, y: 76, scale: 1.2, paint: "red-300" },
    ],
  },
};

/** Prototype-X2 (Stinkyscape #86, Mythic), Alt art: after the test: Prototype-X2 sulking alone on a cold grey moon, its five mismatched cockpits turned away from each other, frost on its welds. Bleak blue-grey. The kit's fused hull keeps its cockpits along its spine, so they cannot turn away. */
const PROTOTYPE_X2_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-400", "slate-100"],
  light: { x: 28, y: 24, r: 34, paint: "white", opacity: 0.3 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 16, scale: 1.4 },
      { part: "planet", x: 24, y: 22, scale: 0.8, paint: "blue-200" },
      { part: "cloud", x: 76, y: 20, scale: 1, paint: "slate-100" },
    ],
    far: [
      { part: "mountains", x: 50, y: 62, paint: "slate-400" },
      { part: "ice-shard", x: 84, y: 54, scale: 0.6, count: 3, spread: [6, 4] },
    ],
    mid: [
      { part: "regolith", x: 50, y: 70, paint: "slate-400" },
      { part: "bowl-crater", x: 20, y: 76, scale: 0.8 },
      { part: "debris", x: 82, y: 72, scale: 1, count: 2, spread: [8, 4] },
    ],
    subject: [
      { part: "fused-hull", x: 50, y: 66, scale: 1 },
      { part: "frost", x: 50, y: 76, scale: 1.2 },
      { part: "ice-shard", x: 34, y: 70, scale: 0.6, count: 2, spread: [6, 4] },
    ],
    near: [
      { part: "frost", x: 16, y: 96, scale: 1.4 },
      { part: "debris", x: 86, y: 92, scale: 1.4 },
      { part: "pebbles", x: 50, y: 98, scale: 1.4, count: 2, spread: [16, 2] },
    ],
    light: [
      { part: "rim-light", x: 50, y: 60, scale: 1.4, paint: "white" },
      { part: "halo", x: 24, y: 22, scale: 1, paint: "blue-200" },
    ],
    particles: [
      { part: "snow", x: 50, y: 40, scale: 1.4, paint: "white" },
      { part: "snow", x: 50, y: 74, scale: 1.2, paint: "white" },
      { part: "motes", x: 50, y: 50, scale: 1, paint: "slate-100" },
    ],
  },
};

/** Siva 1 (Stinkyscape #87, Legendary), Alt art: a close-up portrait of Siva in a quiet dressing room, ears up, idly winding the glowing leash around one paw. Soft green mirror light. The figure kit has no leash, so a ribbon of light loops beside the paw. */
const SIVA_1_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["green-900", "jade-400"],
  light: { x: 70, y: 40, r: 38, paint: "green-200", opacity: 0.45 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  figure: { x: 44, y: 108, scale: 1.5, pose: "pose-reach", look: normalLookOf("stinkyscape-087") },
  layers: {
    sky: [
      { part: "bulbs", x: 70, y: 22, scale: 1 },
      { part: "bulbs", x: 70, y: 84, scale: 0.8 },
    ],
    far: [
      { part: "room-wall", x: 50, y: 90, paint: "green-700" },
      { part: "mirror", x: 74, y: 76, scale: 1.8 },
    ],
    mid: [{ part: "drapes", x: 14, y: 20, scale: 0.9 }],
    subject: [{ part: "light-ribbon", x: 58, y: 66, scale: 0.4, paint: "lime-200" }],
    near: [
      { part: "wide-table", x: 80, y: 104, scale: 1.4, paint: "wood-700" },
      { part: "flask", x: 76, y: 88, scale: 0.6 },
      { part: "hand-mirror", x: 88, y: 90, scale: 0.8 },
      { part: "bottle", x: 12, y: 96, scale: 1, count: 2, spread: [6, 1] },
    ],
    light: [
      { part: "halo", x: 74, y: 56, scale: 1.4, paint: "green-200" },
      { part: "rim-light", x: 44, y: 60, scale: 1.2, paint: "green-200" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1, paint: "green-200" },
      { part: "sparks", x: 58, y: 64, scale: 0.6, paint: "lime-200" },
    ],
  },
};

/** Siva 2 (Stinkyscape #88, Legendary), Alt art: Siva 2 at a white formal banquet on a station's sky deck, smiling, one drop of green venom falling into a crystal glass. Bright noon light, tense. The kit has no crystal glass, so the venom falls into a teacup, and no smile, so the face is the kit's own. */
const SIVA_2_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["sky-300", "white"],
  light: { x: 70, y: 18, r: 38, paint: "white", opacity: 0.5 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 36, y: 100, scale: 1.3, pose: "pose-hold", look: { ...normalLookOf("stinkyscape-088"), item: null, cloth: "slate-100" } },
  extras: [
    {
      layer: "mid",
      x: 78,
      y: 88,
      scale: 0.8,
      flip: -1,
      pose: "pose-stand",
      look: { ...figureLookFor(["Human"]), hair: "hair-bun", outfit: "outfit-gown", item: null, skin: "wood-700", cloth: "white" },
    },
  ],
  layers: {
    sky: [
      { part: "sun", x: 72, y: 16, scale: 0.8 },
      { part: "cloud", x: 24, y: 24, scale: 1.2, paint: "white" },
    ],
    far: [
      { part: "glass-wall", x: 50, y: 70, scale: 1.4 },
      { part: "ring-station", x: 82, y: 40, scale: 0.4 },
    ],
    mid: [{ part: "column", x: 10, y: 80, scale: 1.2 }],
    subject: [
      { part: "cup", x: 56, y: 76, scale: 1.4 },
      { part: "drops", x: 56, y: 64, scale: 0.4, paint: "green-400" },
    ],
    near: [
      { part: "wide-table", x: 54, y: 106, scale: 2.4, paint: "white" },
      { part: "cup", x: 80, y: 94, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 56, y: 70, scale: 0.6, paint: "green-400" },
      { part: "god-rays", x: 72, y: 16, scale: 0.8, paint: "white" },
    ],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1, paint: "white" }],
  },
};

/** Dormant Starship (Stinkyscape token, Legendary), Alt art: the dormant hull drifting alone in a quiet debris field, cables trailing, the faint ghost of an open blue sky reflected in its dark canopy. Pale pink dawn. */
const DORMANT_STARSHIP_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-300", "slate-100"],
  light: { x: 78, y: 70, r: 40, paint: "red-300", opacity: 0.45 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 40, y: 18, scale: 1 },
      { part: "star-glare", x: 80, y: 74, scale: 1, paint: "red-300" },
    ],
    far: [{ part: "debris", x: 22, y: 30, scale: 0.7, count: 3, spread: [10, 8] }],
    mid: [
      { part: "wreckage", x: 82, y: 34, scale: 0.6 },
      { part: "debris", x: 70, y: 82, scale: 1, count: 2, spread: [10, 4] },
    ],
    subject: [
      { part: "dormant-hull", x: 54, y: 52, scale: 1.1 },
      { part: "cable", x: 30, y: 60, scale: 0.6, flip: -1 },
      { part: "cable", x: 34, y: 46, scale: 0.5, flip: -1 },
    ],
    near: [
      { part: "debris", x: 12, y: 90, scale: 1.4 },
      { part: "wreckage", x: 88, y: 96, scale: 0.9 },
    ],
    light: [
      { part: "rim-light", x: 54, y: 48, scale: 1.4, paint: "red-300" },
      { part: "halo", x: 80, y: 74, scale: 1.6, paint: "red-300" },
    ],
    particles: [
      { part: "dust", x: 50, y: 60, scale: 1.2, paint: "white" },
      { part: "motes", x: 50, y: 40, scale: 1, paint: "red-300" },
    ],
  },
};

/** R3394000: the alt recipes of #3394's cards 62 to 88 and the Dormant Starship, by id. */
export const STINKYSCAPE_ALT_RECIPES_3: Readonly<Record<string, SceneRecipe>> = {
  "stinkyscape-062": MINER_V1_ALT,
  "stinkyscape-064": MINER_V3_ALT,
  "stinkyscape-065": MINER_V4_ALT,
  "stinkyscape-066": MINER_V5_ALT,
  "stinkyscape-068": STRATOS_ALT,
  "stinkyscape-069": FALCON_ALT,
  "stinkyscape-070": ZZOTH_ALT,
  "stinkyscape-072": PARAGON_MK1_ALT,
  "stinkyscape-073": PARAGON_MK2_ALT,
  "stinkyscape-075": BADGER_ALT,
  "stinkyscape-077": YUMA_ALT,
  "stinkyscape-078": OZARK_ALT,
  "stinkyscape-079": ORCA_ALT,
  "stinkyscape-081": RADIX_ALT,
  "stinkyscape-082": PHALANX_ALT,
  "stinkyscape-083": HALO_ALT,
  "stinkyscape-086": PROTOTYPE_X2_ALT,
  "stinkyscape-087": SIVA_1_ALT,
  "stinkyscape-088": SIVA_2_ALT,
  "stinkyscape-t-dormant-starship": DORMANT_STARSHIP_ALT,
};
